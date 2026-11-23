//! The panel's close through nice-plug's real VST3 `IPlugView::removed`
//! (W-2b), with the crate wired as a consumer wires it
//! (`vst3_host::panel`: `queue_later`, the task run by the plugin's
//! executor), under a host whose `endEdit` sets the parameter back through
//! the controller, as a host may. Moved with the code from Kör's and
//! Fläns's `tests/b_pnl_11_held.rs`, `closing_window` (after Rödalog v3's
//! `tests/held_edits.rs`): the fault it guards (CUBASE-1b, 2026-11-14) is
//! a host aborted inside `removed()`. Natively only: on Windows nice-plug's
//! main-thread queue is a hidden window's messages, and no Windows test
//! here runs (Kör's `b_pnl_11_close.rs` is its Windows counterpart).
//!
//! No test sleeps: the one wait, for the queued end on nice-plug's worker
//! thread, is on an event with a deadline (`vst3_host::DEADLINE`).

#![cfg(not(windows))]

mod support;
mod vst3_host;

use std::sync::Mutex;

use nice_plug::prelude::Param;
use nice_plug::wrapper::vst3::vst3::Steinberg::Vst::{
    IComponentHandler, IComponentHandlerTrait, IEditController, IEditControllerTrait, ParamID, ParamValue,
};
use nice_plug::wrapper::vst3::vst3::Steinberg::{int32, kResultOk, tresult, IPluginBaseTrait};
use nice_plug::wrapper::vst3::vst3::{Class, ComPtr, ComWrapper};
use panelwerk_panel::egui::{Context, Event, Modifiers, PointerButton, Vec2};
use support::{panel_frame, FRAME, MIX};
use vst3_host::panel::{Panel, Task, GATE, RAN};
use vst3_host::Signal;

/// What the host sets Mix to at its `endEdit`: unlike any value the drag
/// sets (Mix from 50 % upwards), so the plugin's value changes, as it must
/// for the wrapper to tell the editor of it.
const SET_BACK: f64 = 0.125;

/// A host that, at each `endEdit`, sets the parameter through the
/// controller (`setParamNormalized`) to [`SET_BACK`], and counts the begins
/// and the ends.
struct SetsBack {
    controller: Mutex<Option<ComPtr<IEditController>>>,
    begins: Signal,
    ends: Signal,
}

impl Class for SetsBack {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for SetsBack {
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        self.begins.raise();
        kResultOk
    }
    unsafe fn performEdit(&self, _id: ParamID, _value: ParamValue) -> tresult {
        kResultOk
    }
    unsafe fn endEdit(&self, id: ParamID) -> tresult {
        let controller = self.controller.lock().unwrap().clone().expect("the controller");
        assert_eq!(unsafe { controller.setParamNormalized(id, SET_BACK) }, kResultOk);
        self.ends.raise();
        kResultOk
    }
    unsafe fn restartComponent(&self, _flags: int32) -> tresult {
        kResultOk
    }
}

#[test]
fn w_2b_closing_the_panel_mid_drag_queues_the_end_and_the_host_survives_its_set_back() {
    // With audio not processing, the wrapper applies the host's
    // `setParamNormalized` at once and tells the editor of the change;
    // inside `IPlugView::removed`, which holds the editor while it drops
    // it, that would borrow it twice and panic inside a host's call. The
    // close must end the edit, once, and the host survive it.
    let host = ComWrapper::new(SetsBack { controller: Mutex::new(None), begins: Signal::new(), ends: Signal::new() });
    let handler = host.to_com_ptr::<IComponentHandler>().unwrap();
    let mut panel = Panel::open(&handler);
    *host.controller.lock().unwrap() = Some(panel.controller.clone());
    let (params, edits, executor) = (panel.params(), panel.edits(), panel.executor());

    // A drag on Mix begun, moved and not let go as the window closes.
    let ctx = Context::default();
    let at = MIX.center();
    let press =
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE };
    let steps = [
        vec![],
        vec![Event::PointerMoved(at)],
        vec![press],
        vec![Event::PointerMoved(at - Vec2::new(0.0, 10.0))],
        vec![Event::PointerMoved(at - Vec2::new(0.0, 20.0))],
    ];
    for (n, events) in steps.into_iter().enumerate() {
        let (drawn, _) = panel_frame(&edits, &ctx, FRAME * n as f64, events);
        assert!(drawn, "the window is open");
    }
    assert_eq!(host.begins.count(), 1, "the drag began");
    assert_ne!(params.mix.unmodulated_normalized_value(), 0.5, "and moved Mix (not processing: at once)");
    assert_eq!(host.ends.count(), 0, "the drag is held as the window closes");

    let ran = RAN.count();
    GATE.set(false);
    assert!(panel.close(), "the host survives the close");
    assert_eq!(host.ends.count(), 0, "nothing ended inside the host's call");
    GATE.set(true);
    // The queued end runs on nice-plug's main-thread queue: natively its
    // worker thread (`GATE`). There the task calls the host's `endEdit`,
    // whose `setParamNormalized` queues the editor's update behind it on
    // the same worker. So the test waits for the task to have returned,
    // then for one more task queued behind that update, before it takes the
    // handler away: else, natively, its `setComponentHandler` could meet
    // the worker's borrow (Rödalog v3, under load, 2026-11-14).
    assert!(RAN.wait_for(ran + 1), "the queued end ran");
    assert_eq!(host.ends.count(), 1, "the drag's gesture ended for the host, once");
    assert_eq!(params.mix.unmodulated_normalized_value(), SET_BACK as f32, "the host's own set, taken");
    std::thread::spawn(move || executor.execute_gui(Task::Marker)).join().unwrap();
    assert!(RAN.wait_for(ran + 2), "the queue ran behind the end");
    drop(edits);
    let component = panel.component.clone();
    drop(panel);
    *host.controller.lock().unwrap() = None;
    assert_eq!(unsafe { component.terminate() }, kResultOk);
    assert_eq!((host.begins.count(), host.ends.count()), (1, 1), "one begin–end pair");
    drop(handler);
}
