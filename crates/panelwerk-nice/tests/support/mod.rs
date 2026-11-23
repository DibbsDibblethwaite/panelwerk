//! What the tests share: a stand-in plugin's parameters ([`Knobs`]), a
//! panel of three controls drawn with Panelwerk's own gestures ([`draw`]),
//! and a GUI context that records what the host is told ([`Recorder`]).
//! Kör's and Fläns's tests used their own parameters and panel; these are
//! the same kinds of control (a switch, a linear knob, a skewed knob whose
//! normalised value doesn't survive the plain value's round trip), named
//! for no plugin (CLAUDE.md rule 1).

#![allow(dead_code)] // Each test binary uses part of it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use nice_plug::context::gui::{GuiContext, GuiContextInner};
use nice_plug::context::PluginApi;
use nice_plug::prelude::*;
use panelwerk_nice::HeldEdits;
use panelwerk_panel::egui::{pos2, Context, Event, FullOutput, Id, Pos2, RawInput, Rect, Sense, Ui, ViewportId};
use panelwerk_panel::{gesture, ParamHost};

/// The stand-in's parameters.
#[derive(Params)]
pub struct Knobs {
    /// A switch: a click toggles it.
    #[id = "on"]
    pub on: BoolParam,
    /// A linear knob, 0 … 1, at 0.5 by default.
    #[id = "mix"]
    pub mix: FloatParam,
    /// A knob over [`TIME_MIN`] … [`TIME_MAX`] ms: a normalised value set by
    /// a drag often comes back from its plain value as another, so only a
    /// plain comparison sees that the plugin has it (`Adapter::shows`).
    #[id = "time"]
    pub time: FloatParam,
}

/// The Time knob's range, in ms: Kör's Manual Delay's, linear, whose round
/// trip Kör's rounding test found inexact (`held.rs`'s checks that it still
/// is).
pub const TIME_MIN: f32 = 1.66;
pub const TIME_MAX: f32 = 5.35;

impl Default for Knobs {
    fn default() -> Self {
        let time = FloatRange::Linear { min: TIME_MIN, max: TIME_MAX };
        Self {
            on: BoolParam::new("On", false),
            mix: FloatParam::new("Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            time: FloatParam::new("Time", 3.505, time).with_unit(" ms"),
        }
    }
}

/// The window's size in points.
pub const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, pos2(200.0, 60.0));

/// The panel clock's step from one frame to the next: a 60 Hz panel, well
/// under `gesture::PENDING_FOR` (0.5 s), so no edit here ends by the time
/// limit, only by its value arriving (or by the window going).
pub const FRAME: f64 = 1.0 / 60.0;

/// One frame on `ctx` at `now` seconds with `events` (no other input, so
/// egui asks for no frame of its own accord), drawn by `run`.
pub fn frame(ctx: &Context, now: f64, events: Vec<Event>, run: impl FnMut(&mut Ui)) -> FullOutput {
    let input = RawInput { screen_rect: Some(SCREEN), time: Some(now), events, ..RawInput::default() };
    let mut run = run;
    let mut out = ctx.run_ui(input, |ui| run(ui));
    out.textures_delta.clear();
    out
}

/// One frame of the test panel drawn through `edits` ([`HeldEdits::frame`]
/// with [`draw`]) on `ctx` at `now` with `events`: whether it was drawn
/// (not before `open`), and the frame's output. One `draw` for every
/// frame in a test binary, so one instantiation of the generic `frame`
/// runs every path of it (llvm-cov counts a generic's regions by its best
/// single instantiation).
pub fn panel_frame(edits: &HeldEdits, ctx: &Context, now: f64, events: Vec<Event>) -> (bool, FullOutput) {
    let mut drawn = false;
    let out = frame(ctx, now, events, |ui| drawn = edits.frame(ui, draw).is_some());
    (drawn, out)
}

/// Whether a frame's output asks egui for another frame at once.
pub fn asks_again(out: &FullOutput) -> bool {
    out.viewport_output[&ViewportId::ROOT].repaint_delay == Duration::ZERO
}

/// Where each control is.
pub const ON: Rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(50.0, 50.0));
pub const MIX: Rect = Rect::from_min_max(pos2(60.0, 10.0), pos2(100.0, 50.0));
pub const TIME: Rect = Rect::from_min_max(pos2(110.0, 10.0), pos2(150.0, 50.0));

/// The panel: the switch and the two knobs, each worked by Panelwerk's
/// gestures through `host`, as a consumer's panel works its controls.
/// It takes `&mut Ui`, as Kör's and Fläns's `panel` do, so the tests call
/// [`HeldEdits::frame`] as those plugins must.
pub fn draw(ui: &mut Ui, host: &dyn ParamHost) {
    let on = ui.interact(ON, Id::unique("on"), Sense::click());
    gesture::toggle_on_click(ui, host, &on, "on");
    for (rect, id) in [(MIX, "mix"), (TIME, "time")] {
        let knob = ui.interact(rect, Id::unique(id), Sense::click_and_drag());
        gesture::knob(ui, host, &knob, id);
    }
}

/// A GUI context that records each call by parameter ID, and applies each
/// set to the parameter at once if `applies` (nice-plug's wrapper with
/// audio not processing), else never (a host processing that hasn't yet
/// passed the change back). Kör's and Fläns's `Recorder`.
pub struct Recorder {
    by_ptr: Vec<(ParamPtr, String)>,
    calls: Mutex<Vec<String>>,
    applies: bool,
    /// The normalised value last set, of whichever parameter.
    pub last_set: Mutex<Option<f32>>,
}

impl Recorder {
    /// A recorder for `params`' parameters, and the GUI context over it.
    pub fn new(params: &Knobs, applies: bool) -> (Arc<Self>, GuiContext) {
        let by_ptr = params.param_map().into_iter().map(|(id, ptr, _)| (ptr, id)).collect();
        let recorder = Arc::new(Self { by_ptr, calls: Mutex::default(), applies, last_set: Mutex::default() });
        let gui = GuiContext::new(recorder.clone());
        (recorder, gui)
    }

    /// The ID of one of the plugin's parameters.
    pub fn id(&self, param: ParamPtr) -> String {
        self.by_ptr.iter().find(|(p, _)| *p == param).expect("one of the plugin's parameters").1.clone()
    }

    fn log(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    /// The calls made since the last `take`, taking them.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
}

impl GuiContextInner for Recorder {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Vst3
    }
    unsafe fn raw_begin_set_parameter(&self, param: ParamPtr) {
        self.log(format!("begin {}", self.id(param)));
    }
    unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, normalized: f32) {
        self.log(format!("set {}", self.id(param)));
        *self.last_set.lock().unwrap() = Some(normalized);
        if self.applies {
            // SAFETY: the test's own parameters, alive for the test.
            unsafe { param._internal_set_normalized_value(normalized) };
        }
    }
    unsafe fn raw_end_set_parameter(&self, param: ParamPtr) {
        self.log(format!("end {}", self.id(param)));
    }
    fn get_state(&self) -> PluginState {
        unreachable!("the adapter doesn't read the state")
    }
    fn set_state(&self, _: PluginState) {
        unreachable!("the adapter doesn't set the state")
    }
    fn request_restart(&self) {
        unreachable!("the adapter doesn't restart the plugin")
    }
}

/// The value a host passes back to the plugin: sets `param` to `normalized`
/// as nice-plug's wrapper does when a host's change reaches it.
pub fn arrives(param: &impl Param, normalized: f32) {
    // SAFETY: the test's own parameter, alive for the call.
    unsafe { param.as_ptr()._internal_set_normalized_value(normalized) };
}
