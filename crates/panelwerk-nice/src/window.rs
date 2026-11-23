//! A window's held edits on nice-plug: what Kör's and Fläns's `app.rs`
//! (`App`'s held edits, `Leaving`, `Handle`) and `lib.rs` (`queue_later`)
//! each carried, made one. No `unsafe`: it reaches the plugin only through
//! [`crate::adapter`]'s safe types.

use std::sync::{Arc, Mutex, PoisonError};

use nice_plug::context::gui::{AsyncExecutor, GuiContext};
use nice_plug::editor::dpi::NativeSize;
use nice_plug::editor::{EditorHandle, Modifiers, ParentWindowHandle, VirtualKeyCode};
use nice_plug::prelude::{Params, Plugin};
use panelwerk_panel::egui;
use panelwerk_panel::hold::Held;
use panelwerk_panel::ParamHost;

use crate::adapter::{Adapter, EndEdits, ParamMap};

/// How a window's close or hide has the edits still open ended for the
/// host: given the task that ends them. In a plugin, [`queue_later`]; in a
/// test, anything that keeps the task.
pub type Ends = Arc<dyn Fn(EndEdits) + Send + Sync>;

/// The panel's edits held open, shared by the app, which draws the frames
/// that end them, and the window's [`Handle`], which takes those still
/// open as the window goes. Both on the host's GUI thread (on Windows the
/// main thread, where the window's paints and the host's calls both come),
/// so the lock is never contended; a lock, not a cell, because nice-plug
/// needs the handle, and nice-plug-egui the app, to be `Send`.
type Shared = Arc<Mutex<Held>>;

/// One panel window's held edits (Panelwerk's [`Held`], `hold.rs`, whose
/// module doc is the design), owned by the app that lives as long as the
/// window: made in each `Editor::spawn`, before the window, so that
/// [`HeldEdits::leaving`] can wrap the window's handle.
pub struct HeldEdits {
    map: ParamMap,
    gui: Option<GuiContext>,
    held: Shared,
}

impl HeldEdits {
    /// The held edits of a window about to open over `params`.
    pub fn new(params: Arc<dyn Params>) -> Self {
        Self { map: ParamMap::new(params), gui: None, held: Shared::default() }
    }

    /// What the window's handle needs to end the edits still open as the
    /// window closes or is hidden: they are ended through `gui` (the
    /// context the window is opened with) by `ends`. Made before the
    /// window, since nice-plug-egui takes the app as it makes the window;
    /// wrapped round the window's handle with [`Leaving::around`].
    pub fn leaving(&self, gui: GuiContext, ends: Ends) -> Leaving {
        Leaving { held: self.held.clone(), map: self.map.clone(), gui, ends }
    }

    /// The window has opened (nice-plug-egui's `build`): the context its
    /// edits go through.
    pub fn open(&mut self, gui: GuiContext) {
        self.gui = Some(gui);
    }

    /// One frame (in nice-plug-egui's `ui`): the held edits whose values
    /// have arrived are ended first, at egui's input time (the panel's
    /// clock), then `draw` draws the panel in `ui` with the host it is to be
    /// given, whose begins, sets and ends go through the held edits; while
    /// an edit still waits, another frame is asked for, since the window
    /// draws only when asked and the edit would otherwise end only at the
    /// next input. Before [`Self::open`], nothing: `None`, and `draw` isn't
    /// called. `ui` is handed on, mutable, because a consumer's panel takes
    /// `&mut Ui` (Kör's and Fläns's `panel(ui, host)`), which a closure
    /// couldn't borrow while `frame` held it too.
    ///
    /// Every control in the window must be drawn with this one host
    /// (`gesture::knob`'s doc): a consumer's own host trait (zoom, its
    /// extras) wraps it, passing `ParamHost`'s calls straight through.
    pub fn frame<R>(&self, ui: &mut egui::Ui, draw: impl FnOnce(&mut egui::Ui, &dyn ParamHost) -> R) -> Option<R> {
        let gui = self.gui.as_ref()?;
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let raw = Adapter::new(&self.map, gui);
        let waits = held.frame(&raw, ui.input(|i| i.time));
        let drawn = draw(ui, &Edits { raw: &raw, held: &held });
        if waits {
            ui.ctx().request_repaint();
        }
        Some(drawn)
    }
}

/// One frame's host for the panel: the values from the [`Adapter`], the
/// edits through the held edits. Kör's and Fläns's `Frame`, but for their
/// own traits (zoom, the unsupported rate), which stay theirs.
struct Edits<'a> {
    raw: &'a Adapter<'a>,
    held: &'a Held,
}

impl ParamHost for Edits<'_> {
    fn value(&self, id: &str) -> f32 {
        self.raw.value(id)
    }
    fn default_value(&self, id: &str) -> f32 {
        self.raw.default_value(id)
    }
    fn text(&self, id: &str) -> String {
        self.raw.text(id)
    }
    fn begin(&self, id: &str) {
        self.held.begin(self.raw, id)
    }
    fn set(&self, id: &str, normalized: f32) {
        self.held.set(self.raw, id, normalized)
    }
    fn end(&self, id: &str) {
        self.held.end(id)
    }
    // `shows` is left at the trait's default, as Kör's and Fläns's `Frame`
    // leave it: the held edits ask `raw`'s, the plain comparison, and
    // nothing in the panel asks the panel's own host.
}

/// What a window's [`Handle`] needs to end its app's edits still open
/// ([`HeldEdits::leaving`]).
pub struct Leaving {
    held: Shared,
    map: ParamMap,
    gui: GuiContext,
    ends: Ends,
}

impl Leaving {
    /// The window's own handle, `inner`, wrapped so that its close and hide
    /// end the edits still open.
    pub fn around<H>(self, inner: H) -> Handle<H> {
        Handle { inner, leaving: self }
    }

    /// The window gone from view, closed or hidden: every edit still open
    /// or waiting is taken from the held edits, in the order begun, and
    /// handed to [`Ends`] to be ended for the host later, since the window
    /// draws no frame now to end it, and ending it here, inside the host's
    /// call, could crash the host ([`queue_later`]). An edit made after the
    /// window is shown again begins a new gesture.
    fn gone(&self) {
        let open = self.held.lock().unwrap_or_else(PoisonError::into_inner).take_open();
        // None open, the usual case: no task, and no thread to queue it.
        if !open.is_empty() {
            (self.ends)(EndEdits::new(self.gui.clone(), self.map.clone(), open));
        }
    }
}

/// An open window's handle, which the host drops as the window closes
/// (VST3's `IPlugView::removed`, CLAP's `gui.destroy`) and calls `hide` on
/// as it hides it (CLAP's `gui.hide`; VST3 has no such call, so there an
/// edit waiting as the host hides the window stays open until the window
/// is shown again or closes). Everything else passes straight through to
/// the window's own handle.
///
/// Why here and not in the app's `Drop`, where Panelwerk's `hold.rs` puts
/// the close: only the handle hears of a hide, and one place for both is
/// one path, as Rödalog v3's `window::Open::gone`. The edits are taken
/// before the rest of the spawned editor goes: the inner handle (with
/// nice-plug-egui, one whose drop only marks the window closed), then the
/// window itself (`SpawnedEditor::window`, dropped after the handle in
/// field order), whose drop closes the window and drops the app.
pub struct Handle<H> {
    inner: H,
    leaving: Leaving,
}

/// The window closing: its edits still open are taken here, first, before
/// `inner` is dropped and, after it, the window itself
/// (`SpawnedEditor::window`), whose drop closes the window.
impl<H> Drop for Handle<H> {
    fn drop(&mut self) {
        self.leaving.gone();
    }
}

impl<H: EditorHandle> EditorHandle for Handle<H> {
    type Window = H::Window;
    type Error = H::Error;

    fn run_until_closed(window: Self::Window) -> Result<(), Self::Error> {
        H::run_until_closed(window)
    }
    fn set_parent(&self, parent: ParentWindowHandle, window: &Self::Window) -> Result<(), Self::Error> {
        self.inner.set_parent(parent, window)
    }
    fn show(&self, window: &Self::Window) -> Result<(), Self::Error> {
        self.inner.show(window)
    }
    /// Hidden counts as closed for the edits still waiting (a hidden
    /// window draws no frames to end them; Kör's B-PNL-11 as amended, Ian,
    /// 2026-11-14, KOER-PNL-3), whether or not the window's own hide
    /// succeeds: if it fails, the window may stay in view, and an edit made
    /// next begins a new gesture, which is harmless. Likewise a drag held
    /// as CLAP hides the window: its gesture is ended for the host, so if
    /// the mouse keeps moving and frames still run, the panel's sets reach
    /// the host with no gesture open, and the release ends nothing. Rare,
    /// and not tested.
    fn hide(&self, window: &Self::Window) -> Result<(), Self::Error> {
        self.leaving.gone();
        self.inner.hide(window)
    }
    fn set_size(&self, new_size: NativeSize<u32>, window: &Self::Window) -> Result<(), Self::Error> {
        self.inner.set_size(new_size, window)
    }
    fn host_main_thread_callback(&self, window: &Self::Window) {
        self.inner.host_main_thread_callback(window)
    }
    fn adjust_size(&self, new_size: NativeSize<u32>, window: &Self::Window) -> Option<NativeSize<u32>> {
        self.inner.adjust_size(new_size, window)
    }
    fn set_fallback_scale_factor(&self, scale_factor: f64, window: &Self::Window) -> Result<(), Self::Error> {
        self.inner.set_fallback_scale_factor(scale_factor, window)
    }
    fn on_virtual_key_from_host(&self, key_code: VirtualKeyCode, is_down: bool, modifiers: Modifiers) -> bool {
        self.inner.on_virtual_key_from_host(key_code, is_down, modifiers)
    }
    fn state_changed(&self) {
        self.inner.state_changed()
    }
    fn param_value_changed(&self, id: &str, normalized_value: f32) {
        self.inner.param_value_changed(id, normalized_value)
    }
    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {
        self.inner.param_modulation_changed(id, modulation_offset)
    }
}

/// [`Ends`] that queues each task with `executor` for the main thread's
/// next turn, never running it in place: nice-plug runs a task given on
/// the main thread at once, so it is given from a thread of its own,
/// joined before this returns, and nice-plug queues it (VST3: a message to
/// the window it keeps for its queue; CLAP: the host's
/// `request_callback`). Kör's, Fläns's and Rödalog v3's `queue_later`. The
/// plugin's `BackgroundTask` takes an [`EndEdits`] (`From`), and its task
/// executor runs it ([`EndEdits::run`]).
///
/// Why: the panel's window closes inside the host's call to close it
/// (VST3's `IPlugView::removed`, CLAP's `gui.destroy`), both of which hold
/// nice-plug's handle on the editor while they drop it. In VST3, ending the
/// edits still open there calls the host's `endEdit`, and a host that
/// answers with `setParamNormalized` while audio isn't processing has the
/// wrapper tell the editor of the change in place, borrowing that handle a
/// second time: a panic inside the host's call, which takes the host down
/// (found in Rödalog v3, CUBASE-1b, 2026-11-14; `tests/close.rs` here).
/// CLAP has no such path (its gesture's end is an output event the host
/// takes later), so there queuing is only the same code for both.
///
/// A hide (CLAP's `gui.hide`; VST3 has none) is also made inside the
/// host's call, which holds nice-plug's editor handle (shared, not
/// mutably) while the window's `hide` runs. Ending the edits there would be
/// safe in CLAP for the same reason, but they are queued all the same: one
/// path for the close and the hide, with no case of its own for which call
/// is under way. Its cost: in CLAP the end runs at the host's timing, so a
/// window shown again with the same parameter edited before then would have
/// the host told of a new begin while the old gesture is open, the queued
/// end ending it early, and the new edit's own end arriving with none open.
///
/// When the queued end runs: in VST3 on Windows, nice-plug's queue is a
/// message posted to its own window on the same thread, and Windows hands a
/// thread its posted messages before its input and paint messages, so the
/// edits end just after the close returns and before any input to a window
/// opened again (Windows' documented order, not tested). In CLAP the task
/// runs when the host answers `request_callback` with `on_main_thread`.
/// Both run it on the host's main thread, so not during another of the
/// host's calls, unless the host pumps its messages inside one of its own
/// calls (a nested modal loop, say), which would run the task there.
/// Natively (Linux, built for tests only) nice-plug's VST3 queue is a
/// worker thread beside the host's, so there the end could run while
/// `removed()` still holds the editor; `tests/close.rs` holds that worker
/// back as a main thread's queue would.
pub fn queue_later<P>(executor: AsyncExecutor<P>) -> Ends
where
    P: Plugin,
    P::BackgroundTask: From<EndEdits>,
{
    Arc::new(move |task| {
        let executor = &executor;
        std::thread::scope(|s| {
            s.spawn(move || executor.execute_gui(task.into()));
        });
    })
}
