//! A stand-in plugin through nice-plug's real VST3 wrapper (W-2b), wired
//! to the crate as a consumer adopts it (`lib.rs`'s "Adopting it"): its
//! `BackgroundTask` takes [`EndEdits`], its task executor runs it, its
//! editor is given [`queue_later`] of its executor, and each `spawn` makes
//! the app's [`HeldEdits`] and wraps the window's handle with
//! [`HeldEdits::leaving`]. Its editor opens no window (the test draws the
//! panel headlessly through the held edits, over the context the wrapper
//! gave): the edits reach the wrapper exactly as a window's would. After
//! Kör's `tests/vst3_host/panel.rs` and `tests/b_pnl_11_held.rs`'s
//! `closing_window`, made one.
//!
//! Its process callback does nothing: what a plugin's engine gets is the
//! parameters' values, which the wrapper sets from the host's parameter
//! changes. Its audio is stereo out only, so the host's process calls need
//! no input buffers.

use std::cell::RefCell;
use std::ffi::c_void;
use std::num::NonZeroU32;
use std::ptr;
use std::sync::Arc;
#[cfg(not(windows))]
use std::sync::{Condvar, Mutex};

use nice_plug::context::gui::GuiContext;
use nice_plug::editor::dpi::NativeSize;
use nice_plug::editor::{EditorHandle, HostMethods, SpawnedEditor};
use nice_plug::prelude::*;
use nice_plug::wrapper::vst3::subcategories::Vst3SubCategory;
use nice_plug::wrapper::vst3::vst3::Steinberg::Vst::{
    IComponent, IComponentHandler, IEditController, IEditControllerTrait,
};
use nice_plug::wrapper::vst3::vst3::Steinberg::{kResultOk, IPlugView, IPlugViewTrait, IPluginBaseTrait};
use nice_plug::wrapper::vst3::vst3::{ComPtr, ComWrapper};
use nice_plug::wrapper::vst3::{Vst3Plugin, Wrapper};
use panelwerk_nice::{queue_later, EndEdits, Ends, Handle, HeldEdits};

use super::Vst3;
#[cfg(not(windows))]
use super::{Signal, DEADLINE};
use crate::support::Knobs;

/// The stand-in's main-thread work, as a consumer's `MainTask`.
pub enum Task {
    /// The edits a closing or hidden window left open.
    EndEdits(EndEdits),
    /// Nothing: a test queues one behind the others to know when they've
    /// all run.
    Marker,
}

impl From<EndEdits> for Task {
    fn from(task: EndEdits) -> Self {
        Task::EndEdits(task)
    }
}

/// The stand-in's parameters, its panel's held edits and GUI context, and
/// its task executor, on the thread that made and opened it.
#[derive(Default)]
struct Made {
    params: Option<Arc<Knobs>>,
    edits: Option<HeldEdits>,
    context: Option<GuiContext>,
    executor: Option<AsyncExecutor<StandIn>>,
}

thread_local! {
    static MADE: RefCell<Made> = RefCell::default();
}

/// Raised as each of the stand-in's main-thread tasks has run to its end,
/// so a test goes on only once nice-plug has let go of what the task
/// borrowed.
#[cfg(not(windows))]
pub static RAN: Signal = Signal::new();

/// Whether the stand-in's main-thread tasks may run: shut while a test is
/// inside a call the host makes on its main thread.
///
/// Why: on Windows nice-plug's main-thread queue is the thread's own
/// messages, and in CLAP the host's `on_main_thread`, so a task queued
/// during a host's call runs only after that call returns, which the cure
/// relies on (`queue_later`). Natively (Linux, tests only) nice-plug runs
/// that queue on a worker thread beside the one the host calls on
/// (`event_loop/linux.rs`), so the queued end could run while `removed()`
/// still holds the editor, and meet that borrow on the worker (Rödalog v3
/// saw it under load, 2026-11-14). The gate holds the worker back, as the
/// main thread's queue does. Kör's `closing_window`'s.
#[cfg(not(windows))]
pub struct Gate {
    open: Mutex<bool>,
    opened: Condvar,
}

#[cfg(not(windows))]
impl Gate {
    pub fn set(&self, open: bool) {
        *self.open.lock().unwrap() = open;
        self.opened.notify_all();
    }

    /// Waits until it's open, at most [`DEADLINE`]: a task the test never
    /// lets run then runs anyway, and the test's own waits fail.
    fn pass(&self) {
        let open = self.open.lock().unwrap();
        drop(self.opened.wait_timeout_while(open, DEADLINE, |open| !*open).unwrap());
    }
}

#[cfg(not(windows))]
pub static GATE: Gate = Gate { open: Mutex::new(true), opened: Condvar::new() };

/// A plugin with the tests' parameters whose editor is [`Bare`].
pub struct StandIn {
    params: Arc<Knobs>,
}

impl Default for StandIn {
    fn default() -> Self {
        let params: Arc<Knobs> = Arc::default();
        MADE.with(|m| m.borrow_mut().params = Some(params.clone()));
        Self { params }
    }
}

impl Plugin for StandIn {
    const NAME: &'static str = "Panelwerk held edits stand-in";
    const VENDOR: &'static str = "test";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = "0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] =
        &[AudioIOLayout { main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() }];
    /// As Kör's and Fläns's, so the host's parameter changes reach the
    /// parameters by the same path in the wrapper.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = Task;
    type Editor = Bare;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    /// As a consumer's: the edits ended; natively behind [`GATE`], then
    /// [`RAN`] raised.
    fn task_executor(&mut self) -> TaskExecutor<Self> {
        Box::new(|task| {
            #[cfg(not(windows))]
            GATE.pass();
            match task {
                Task::EndEdits(ends) => ends.run(),
                Task::Marker => {}
            }
            #[cfg(not(windows))]
            RAN.raise();
        })
    }
    /// As a consumer's: the close's ends queued for the main thread.
    fn editor(&mut self, executor: AsyncExecutor<Self>) -> Option<Bare> {
        MADE.with(|m| m.borrow_mut().executor = Some(executor.clone()));
        Some(Bare { params: self.params.clone(), ends: queue_later(executor) })
    }
    fn process(
        &mut self,
        _: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        _: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for StandIn {
    const VST3_CLASS_ID: [u8; 16] = *b"PanelwerkNiceSIn";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

/// An editor that opens no window: as a consumer's `spawn`, it makes the
/// app's held edits and wraps the window's handle with them, and keeps the
/// held edits here for the test to draw with.
pub struct Bare {
    params: Arc<Knobs>,
    ends: Ends,
}

impl Editor for Bare {
    type Handle = Handle<BareHandle>;

    fn spawn(
        &self,
        _parent: Option<ParentWindowHandle>,
        _wait_for_parent: bool,
        _fallback_scale_factor: Option<f64>,
        context: GuiContext,
        _host: Option<HostMethods>,
    ) -> Result<SpawnedEditor<Handle<BareHandle>>, Box<dyn std::error::Error>> {
        let edits = HeldEdits::new(self.params.clone());
        let handle = edits.leaving(context.clone(), self.ends.clone()).around(BareHandle);
        MADE.with(|m| {
            let mut m = m.borrow_mut();
            m.edits = Some(edits);
            m.context = Some(context);
        });
        Ok(SpawnedEditor { handle, window: () })
    }
    fn size(&self) -> NativeSize<u32> {
        NativeSize::new(100, 100)
    }
}

/// The window's own handle, for a window that isn't there.
pub struct BareHandle;

impl EditorHandle for BareHandle {
    type Window = ();
    type Error = std::fmt::Error;

    fn run_until_closed(_window: ()) -> Result<(), std::fmt::Error> {
        unreachable!("a plugin's editor")
    }
    fn set_parent(&self, _parent: ParentWindowHandle, _window: &()) -> Result<(), std::fmt::Error> {
        unreachable!("opened with its parent")
    }
    fn show(&self, _window: &()) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn hide(&self, _window: &()) -> Result<(), std::fmt::Error> {
        unreachable!("VST3 has no hide")
    }
    fn set_size(&self, _new_size: NativeSize<u32>, _window: &()) -> Result<(), std::fmt::Error> {
        Ok(())
    }
    fn host_main_thread_callback(&self, _window: &()) {}
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
}

/// The stand-in, created through nice-plug's VST3 wrapper, given `handler`,
/// and its view opened as a host opens one.
pub struct Panel {
    pub component: ComPtr<IComponent>,
    pub controller: ComPtr<IEditController>,
    view: Option<ComPtr<IPlugView>>,
}

impl Panel {
    pub fn open(handler: &ComPtr<IComponentHandler>) -> Self {
        #[cfg(not(windows))]
        super::keep_another::<StandIn>();
        let component = ComWrapper::new(Wrapper::<StandIn>::new()).to_com_ptr::<IComponent>().unwrap();
        unsafe {
            assert_eq!(component.initialize(ptr::null_mut()), kResultOk);
            let controller: ComPtr<IEditController> = component.cast().unwrap();
            assert_eq!(controller.setComponentHandler(handler.as_ptr()), kResultOk);
            let view = ComPtr::from_raw(controller.createView(c"editor".as_ptr())).expect("a view");
            #[cfg(windows)]
            let platform = nice_plug::wrapper::vst3::vst3::Steinberg::kPlatformTypeHWND;
            #[cfg(not(windows))]
            let platform = nice_plug::wrapper::vst3::vst3::Steinberg::kPlatformTypeX11EmbedWindowID;
            // A stand-in parent: the editor opens no window.
            assert_eq!(view.attached(ptr::dangling_mut::<c_void>(), platform), kResultOk);
            Self { component, controller, view: Some(view) }
        }
    }

    /// The stand-in set up and processing, as a host makes it before its
    /// first process call: while it processes, nice-plug's wrapper leaves a
    /// panel edit for the host to pass back (`context.rs`,
    /// `raw_set_parameter_normalized`).
    pub fn start(&self) -> Vst3 {
        Vst3::set_up(self.component.clone())
    }

    /// The stand-in's parameters.
    pub fn params(&self) -> Arc<Knobs> {
        MADE.with(|m| m.borrow().params.clone().expect("the stand-in is made"))
    }

    /// The open view's held edits, opened (nice-plug-egui's `build`) on
    /// the context the wrapper gave it; taken, as the app owns them.
    pub fn edits(&self) -> HeldEdits {
        MADE.with(|m| {
            let mut m = m.borrow_mut();
            let mut edits = m.edits.take().expect("the view is open");
            edits.open(m.context.clone().expect("the view's context"));
            edits
        })
    }

    /// The stand-in's task executor, as its editor was given it.
    pub fn executor(&self) -> AsyncExecutor<StandIn> {
        MADE.with(|m| m.borrow().executor.clone().expect("the editor is made"))
    }

    /// The host closes the view (`IPlugView::removed`), which drops the
    /// spawned editor and so the window's handle; whether it survived.
    pub fn close(&mut self) -> bool {
        let view = self.view.take().expect("the view is open");
        unsafe { view.removed() == kResultOk }
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        if self.view.is_some() {
            assert!(self.close());
        }
        // Also breaks the cycle a handler that holds this controller makes
        // (`model::ModelHost`).
        let _ = unsafe { self.controller.setComponentHandler(ptr::null_mut()) };
        MADE.with(|m| *m.borrow_mut() = Made::default());
    }
}
