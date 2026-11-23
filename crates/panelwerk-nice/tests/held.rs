//! The crate's seams, moved with the code from Kör's and Fläns's
//! `tests/b_pnl_11_held.rs` (CUBASE-K2, CUBASE-F2; W-2b): the adapter's
//! calls ([`Adapter`]); a window's handle ([`Handle`]), whose close and
//! hide hand the edits still open over as an [`EndEdits`] task, which ends
//! them; the window asking for another frame while an edit waits; and an
//! edit ending once the plugin has its value even when its normalised value
//! doesn't survive the plain value's round trip (`shows`, compared as plain
//! values). The close through nice-plug's real VST3 wrapper is
//! `tests/close.rs`; the three modelled hosts, `tests/host_models.rs`.
//!
//! No test sleeps: the panel's clock is given to each frame ([`FRAME`]).

mod support;
mod vst3_host;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use nice_plug::context::gui::AsyncExecutor;
use nice_plug::editor::dpi::NativeSize;
use nice_plug::editor::{EditorHandle, Modifiers as KeyModifiers, ParentWindowHandle, VirtualKeyCode};
use nice_plug::prelude::{Param, Params};
use panelwerk_nice::{queue_later, Adapter, EndEdits, Ends, Handle, HeldEdits, ParamMap};
use panelwerk_panel::egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use panelwerk_panel::ParamHost;
use support::{arrives, asks_again, frame, panel_frame, Knobs, Recorder, FRAME, MIX, ON, TIME};
use vst3_host::panel::{StandIn, Task};

/// Set by [`Spy::run_until_closed`], which has no instance to record in.
static RAN_UNTIL_CLOSED: AtomicBool = AtomicBool::new(false);

/// A window's own handle that records each call made on it, and, as it is
/// dropped or hidden, how many tasks had been queued by then (the edits are
/// to be handed on first, as Kör's and Fläns's handles do).
struct Spy {
    calls: Arc<Mutex<Vec<String>>>,
    queued: Arc<Mutex<Vec<EndEdits>>>,
}

impl Spy {
    fn log(&self, call: &str) {
        // Not `unwrap`: a failed test can drop the spy while unwinding with
        // the lock poisoned, and a second panic would abort the binary.
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(call.to_owned());
    }

    fn queued(&self) -> usize {
        self.queued.lock().unwrap_or_else(PoisonError::into_inner).len()
    }
}

impl Drop for Spy {
    fn drop(&mut self) {
        let queued = self.queued();
        self.log(&format!("dropped, {queued} queued"));
    }
}

impl EditorHandle for Spy {
    type Window = ();
    type Error = std::fmt::Error;

    fn run_until_closed(_window: ()) -> Result<(), std::fmt::Error> {
        RAN_UNTIL_CLOSED.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn set_parent(&self, _parent: ParentWindowHandle, _window: &()) -> Result<(), std::fmt::Error> {
        self.log("set_parent");
        Ok(())
    }
    fn show(&self, _window: &()) -> Result<(), std::fmt::Error> {
        self.log("show");
        Ok(())
    }
    fn hide(&self, _window: &()) -> Result<(), std::fmt::Error> {
        let queued = self.queued();
        self.log(&format!("hide, {queued} queued"));
        Err(std::fmt::Error)
    }
    fn set_size(&self, new_size: NativeSize<u32>, _window: &()) -> Result<(), std::fmt::Error> {
        self.log(&format!("set_size {}x{}", new_size.width, new_size.height));
        Ok(())
    }
    fn host_main_thread_callback(&self, _window: &()) {
        self.log("host_main_thread_callback");
    }
    fn adjust_size(&self, new_size: NativeSize<u32>, _window: &()) -> Option<NativeSize<u32>> {
        self.log(&format!("adjust_size {}x{}", new_size.width, new_size.height));
        Some(NativeSize::new(7, 8))
    }
    fn set_fallback_scale_factor(&self, scale_factor: f64, _window: &()) -> Result<(), std::fmt::Error> {
        self.log(&format!("set_fallback_scale_factor {scale_factor}"));
        Err(std::fmt::Error)
    }
    fn on_virtual_key_from_host(&self, _key_code: VirtualKeyCode, is_down: bool, _modifiers: KeyModifiers) -> bool {
        self.log(&format!("on_virtual_key_from_host {is_down}"));
        true
    }
    fn state_changed(&self) {
        self.log("state_changed");
    }
    fn param_value_changed(&self, id: &str, normalized_value: f32) {
        self.log(&format!("param_value_changed {id} {normalized_value}"));
    }
    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {
        self.log(&format!("param_modulation_changed {id} {modulation_offset}"));
    }
}

/// The test panel ([`draw`]) over new parameters and a [`Recorder`], drawn
/// inside [`HeldEdits::frame`], its window's handle a [`Handle`] around a
/// [`Spy`] whose close and hide queue their tasks into a list, as
/// `queue_later` queues them for the main thread.
struct Panel {
    params: Arc<Knobs>,
    recorder: Arc<Recorder>,
    edits: HeldEdits,
    ctx: Context,
    now: f64,
    calls: Arc<Mutex<Vec<String>>>,
    queued: Arc<Mutex<Vec<EndEdits>>>,
    handle: Option<Handle<Spy>>,
}

impl Panel {
    fn new(applies: bool) -> Self {
        let params: Arc<Knobs> = Arc::default();
        let (recorder, gui) = Recorder::new(&params, applies);
        let (calls, queued) = (Arc::default(), Arc::<Mutex<Vec<EndEdits>>>::default());
        let to_queue = queued.clone();
        let ends: Ends = Arc::new(move |task| to_queue.lock().unwrap().push(task));
        // As a consumer's `Editor::spawn`: the held edits made, the handle
        // around the window's own, then the window opened with the app.
        let mut edits = HeldEdits::new(params.clone());
        let spy = Spy { calls: Arc::clone(&calls), queued: Arc::clone(&queued) };
        let handle = edits.leaving(gui.clone(), ends).around(spy);
        edits.open(gui.clone());
        let mut panel =
            Self { params, recorder, edits, ctx: Context::default(), now: 0.0, calls, queued, handle: Some(handle) };
        // The first frame: egui finds what's under the mouse from the last
        // frame's widgets.
        panel.frame(vec![]);
        panel
    }

    fn handle(&self) -> &Handle<Spy> {
        self.handle.as_ref().expect("the window is open")
    }

    /// The panel's next frame, [`FRAME`] after the last, with `events`;
    /// whether it asks egui for another frame at once.
    fn frame(&mut self, events: Vec<Event>) -> bool {
        self.now += FRAME;
        let (drawn, out) = panel_frame(&self.edits, &self.ctx, self.now, events);
        assert!(drawn, "the window is open");
        asks_again(&out)
    }

    fn button(at: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    /// A click on the control at `rect`: onto it, pressed, released, a
    /// frame each.
    fn click(&mut self, rect: Rect) {
        let at = rect.center();
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Self::button(at, true)]);
        self.frame(vec![Self::button(at, false)]);
    }

    /// A drag on the knob at `rect`, pressed and moved up `up` points in two
    /// moves, not let go; where the mouse is.
    fn drag_held(&mut self, rect: Rect, up: f32) -> Pos2 {
        let at = rect.center();
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Self::button(at, true)]);
        self.frame(vec![Event::PointerMoved(at - Vec2::new(0.0, up / 2.0))]);
        let to = at - Vec2::new(0.0, up);
        self.frame(vec![Event::PointerMoved(to)]);
        to
    }

    /// The parameters of each task queued so far, by ID, taking them.
    fn queued(&self) -> Vec<Vec<String>> {
        std::mem::take(&mut *self.queued.lock().unwrap())
            .into_iter()
            .map(|task| task.ids().into_iter().map(str::to_owned).collect())
            .collect()
    }

    fn calls(&self) -> Vec<String> {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
}

#[test]
fn w_2b_hiding_the_window_hands_every_edit_open_or_waiting_to_be_ended_in_the_order_begun() {
    let mut p = Panel::new(false);
    // A click on the switch whose value never arrives (a host processing
    // that hasn't passed it back), so its edit waits; then a drag on Mix,
    // held.
    p.click(ON);
    p.frame(vec![]);
    let up = p.drag_held(MIX, 20.0);
    let told = p.recorder.take();
    assert_eq!(told.first().map(String::as_str), Some("begin on"), "{told:?}");
    assert!(!told.iter().any(|c| c.starts_with("end")), "both edits still open: {told:?}");

    // The host hides the window (CLAP's `gui.hide`): both edits are handed
    // over in one task, in the order begun, the host told nothing now, and
    // then the window's own hide is made, its error passed on.
    assert_eq!(p.handle().hide(&()), Err(std::fmt::Error));
    assert_eq!(p.calls(), ["hide, 1 queued"], "the edits handed on before the window's own hide");
    let task = p.queued.lock().unwrap().pop().expect("one task");
    assert_eq!(task.ids(), ["on", "mix"]);
    assert_eq!(p.recorder.take(), Vec::<String>::new(), "nothing told inside the host's call");

    // The task, run (on the main thread, later): each ended once.
    task.run();
    assert_eq!(p.recorder.take(), ["end on", "end mix"]);

    // Shown again, the drag let go and frames drawn: nothing more is told
    // of the edits ended, and nothing more queued.
    p.frame(vec![Panel::button(up, false)]);
    p.frame(vec![]);
    assert_eq!(p.recorder.take(), Vec::<String>::new(), "no edit ended twice");
    assert_eq!(p.queued(), Vec::<Vec<String>>::new());
    // A new click begins a new gesture.
    p.click(ON);
    assert_eq!(p.recorder.take(), ["begin on", "set on"]);
    // Its edit waits for its value, so a hide now hands it over too.
    assert_eq!(p.handle().hide(&()), Err(std::fmt::Error));
    assert_eq!(p.queued(), [["on"]]);
}

#[test]
fn w_2b_a_hide_with_nothing_open_queues_nothing() {
    let p = Panel::new(false);
    assert_eq!(p.handle().hide(&()), Err(std::fmt::Error));
    assert_eq!(p.calls(), ["hide, 0 queued"]);
    assert_eq!(p.queued(), Vec::<Vec<String>>::new(), "no task, so no thread to queue it");
}

#[test]
fn w_2b_closing_the_window_hands_the_edits_still_open_over_before_the_window_goes() {
    let mut p = Panel::new(true);
    p.drag_held(MIX, 20.0);
    assert_eq!(p.recorder.take()[0], "begin mix");
    // The host drops the spawned editor as it closes it (VST3's
    // `removed`, CLAP's `gui.destroy`), its handle first: the edit is
    // handed over, and only then is the window's own handle (here the spy)
    // dropped. In a consumer the window itself (`SpawnedEditor::window`)
    // goes after both, and its drop closes the window and drops the app.
    drop(p.handle.take());
    assert_eq!(p.calls(), ["dropped, 1 queued"]);
    assert_eq!(p.queued(), [["mix"]]);
    assert_eq!(p.recorder.take(), Vec::<String>::new(), "nothing told inside the host's call");
}

#[test]
fn w_2b_closing_with_nothing_open_queues_nothing() {
    let mut p = Panel::new(true);
    drop(p.handle.take());
    assert_eq!(p.calls(), ["dropped, 0 queued"]);
}

#[test]
fn w_2b_the_window_s_handle_passes_every_other_call_to_the_window_s_own() {
    let p = Panel::new(false);
    let h = p.handle();
    // A stand-in parent: the spy opens nothing.
    let parent = ParentWindowHandle::XlibWindow(1);
    assert_eq!(h.set_parent(parent, &()), Ok(()));
    assert_eq!(h.show(&()), Ok(()));
    assert_eq!(h.set_size(NativeSize::new(3, 4), &()), Ok(()));
    h.host_main_thread_callback(&());
    assert_eq!(h.adjust_size(NativeSize::new(5, 6), &()), Some(NativeSize::new(7, 8)));
    assert_eq!(h.set_fallback_scale_factor(1.25, &()), Err(std::fmt::Error));
    assert!(h.on_virtual_key_from_host(VirtualKeyCode::Tab, true, KeyModifiers::empty()));
    h.state_changed();
    h.param_value_changed("mix", 0.25);
    h.param_modulation_changed("mix", 0.5);
    assert_eq!(
        p.calls(),
        [
            "set_parent",
            "show",
            "set_size 3x4",
            "host_main_thread_callback",
            "adjust_size 5x6",
            "set_fallback_scale_factor 1.25",
            "on_virtual_key_from_host true",
            "state_changed",
            "param_value_changed mix 0.25",
            "param_modulation_changed mix 0.5",
        ]
    );
    assert_eq!(<Handle<Spy> as EditorHandle>::run_until_closed(()), Ok(()));
    assert!(RAN_UNTIL_CLOSED.load(Ordering::SeqCst));
    assert_eq!(p.queued(), Vec::<Vec<String>>::new(), "none of them ends an edit");
}

#[test]
fn w_2b_the_panel_asks_for_another_frame_while_an_edit_waits() {
    let mut p = Panel::new(false);
    p.click(ON);
    // The click's frame says nothing waits yet (`Held::frame` runs before
    // the panel draws); egui draws again after a frame with input anyway.
    // From the next frame on, with no input, the window asks for itself.
    assert!(p.frame(vec![]), "the click's edit waits for its value");
    assert!(p.frame(vec![]), "still waiting");
    // The value arrives (the host passes it back); the next frame ends the
    // edit and asks for no more.
    arrives(&p.params.on, 1.0);
    assert!(p.frame(vec![]), "the frame that ends it still saw it waiting at its start");
    assert_eq!(p.recorder.take(), ["begin on", "set on", "end on"]);
    assert!(!p.frame(vec![]), "nothing waits");
}

/// How far the rounding test drags Time, in points: a distance whose
/// value's normalised form doesn't survive the plain value's round trip
/// (found by trying; the test checks that it still doesn't).
const TIME_UP: f32 = 10.0;

#[test]
fn w_2b_an_edit_ends_once_the_plugin_has_its_value_though_its_normalised_value_rounds() {
    // A drag on Time, applied at once (audio not processing): nice-plug
    // keeps the plain value and works the normalised one out from it
    // again, which here isn't the one set. The edit still ends at the frame
    // after the release, not at `gesture::PENDING_FOR`: the plugin shows
    // the value as plain values compare, which is how the adapter asks.
    let mut p = Panel::new(true);
    let up = p.drag_held(TIME, TIME_UP);
    assert_eq!(p.recorder.take().first().map(String::as_str), Some("begin time"));
    let set = p.recorder.last_set.lock().unwrap().expect("the drag set Time");
    let time = p.params.time.as_ptr();
    // The case this pins: the plugin has the value set, as a plain value,
    // but its normalised value isn't the one set.
    assert_eq!(unsafe { time.unmodulated_plain_value() }, unsafe { time.preview_plain(set) });
    assert_ne!(unsafe { time.unmodulated_normalized_value() }, set, "the normalised value rounds");
    p.frame(vec![Panel::button(up, false)]);
    p.frame(vec![]);
    assert_eq!(p.recorder.take(), ["end time"]);
}

#[test]
fn w_2b_nothing_is_drawn_before_the_window_has_opened() {
    let params: Arc<Knobs> = Arc::default();
    let edits = HeldEdits::new(params);
    let (drawn, _) = panel_frame(&edits, &Context::default(), FRAME, vec![]);
    assert!(!drawn, "no frame before `open`: there's no context to tell the host through");
}

#[test]
fn w_2b_the_adapter_reads_the_plugin_s_values_and_tells_the_host_through_the_gui_context() {
    let params: Arc<Knobs> = Arc::default();
    let (recorder, gui) = Recorder::new(&params, true);
    let map = ParamMap::new(params.clone());
    let raw = Adapter::new(&map, &gui);
    assert_eq!(raw.value("mix"), 0.5);
    assert_eq!(raw.default_value("on"), 0.0);
    let time = params.time.preview_normalized(4.0);
    arrives(&params.time, time);
    assert_eq!(raw.value("time"), time);
    // The text with its unit, as the host shows it (the framework's).
    assert_eq!(raw.text("time"), params.time.normalized_value_to_string(time, true));
    assert!(raw.text("time").ends_with(" ms"), "{}", raw.text("time"));
    assert_eq!(raw.default_value("time"), params.time.preview_normalized(3.505));
    raw.begin("mix");
    raw.set("mix", 0.75);
    raw.end("mix");
    assert_eq!(recorder.take(), ["begin mix", "set mix", "end mix"]);
    assert_eq!(params.mix.unmodulated_normalized_value(), 0.75);
    // `shows` compares plain values, so a normalised value that maps to the
    // plugin's plain value counts, and another doesn't.
    assert!(raw.shows("mix", 0.75));
    assert!(!raw.shows("mix", 0.5));
}

#[test]
fn w_2b_the_map_holds_the_plugin_s_parameters_by_string_id() {
    let params: Arc<Knobs> = Arc::default();
    let ids: Vec<String> = params.param_map().into_iter().map(|(id, _, _)| id).collect();
    let map = ParamMap::new(params);
    assert_eq!(ids, ["on", "mix", "time"]);
    assert!(ids.iter().all(|id| map.has(id)));
    assert!(!map.has("rate"));
}

#[test]
#[should_panic(expected = "no entry found for key")]
fn w_2b_the_adapter_panics_on_a_parameter_the_plugin_hasn_t() {
    let params: Arc<Knobs> = Arc::default();
    let (_recorder, gui) = Recorder::new(&params, true);
    let map = ParamMap::new(params);
    Adapter::new(&map, &gui).value("rate");
}

#[test]
fn w_2b_the_map_keeps_the_parameters_alive_after_the_plugin_lets_them_go() {
    // The soundness the module relies on (`adapter.rs`'s doc): a task
    // handed on as the window goes may run after everything else holding
    // the parameters has dropped them.
    let mut p = Panel::new(true);
    p.drag_held(MIX, 20.0);
    drop(p.handle.take());
    let task = p.queued.lock().unwrap().pop().expect("the drag's edit");
    let weak = Arc::downgrade(&p.params);
    drop(p);
    assert!(weak.upgrade().is_some(), "the task keeps them");
    drop(task);
    assert!(weak.upgrade().is_none(), "and lets them go with itself");
}

#[test]
fn w_2b_queue_later_gives_each_task_from_a_thread_of_its_own_and_returns_once_it_is_given() {
    // nice-plug runs a task given on the main thread in place, inside the
    // host's call that closes the window, which is the fault `queue_later`
    // exists to avoid: so the task must be given from another thread. And
    // given before `queue_later`'s closure returns, so the close has handed
    // it on before the window goes. Two tests guard it: this one, on any
    // executor, and `close.rs`, through the real wrapper, where natively
    // the test's thread is nice-plug's main thread, so a task given in
    // place runs inside `removed()` and panics there on the editor's
    // double borrow (seen 3 runs of 3, the W-2b review).
    let given: Arc<Mutex<Vec<(std::thread::ThreadId, &'static str)>>> = Arc::default();
    let to_given = given.clone();
    let executor = AsyncExecutor::<StandIn>::new(
        Arc::new(|_| unreachable!("the edits are ended on the main thread, never in the background")),
        Arc::new(move |task| {
            let kind = match task {
                Task::EndEdits(_) => "end edits",
                Task::Marker => "marker",
            };
            to_given.lock().unwrap().push((std::thread::current().id(), kind));
        }),
    );
    let ends = queue_later(executor);
    let mut p = Panel::new(true);
    p.drag_held(MIX, 20.0);
    let handle = p.handle.take();
    // The handle's own `Ends` queued into the panel's list; this one is
    // `queue_later`'s, given the task the close handed over.
    drop(handle);
    let task = p.queued.lock().unwrap().pop().expect("the drag's edit");
    ends(task);
    let given = given.lock().unwrap().clone();
    assert_eq!(given.len(), 1, "given once, before `queue_later`'s closure returned");
    assert_ne!(given[0].0, std::thread::current().id(), "from a thread of its own");
    assert_eq!(given[0].1, "end edits", "as the plugin's own task (`From<EndEdits>`)");
}

#[test]
fn w_2b_the_frame_s_host_reads_the_plugin_s_values_as_the_adapter_does() {
    // What the panel draws its values and texts from inside a frame: the
    // adapter's, unchanged by the held edits in between.
    let p = Panel::new(true);
    arrives(&p.params.time, 0.25);
    let read = std::cell::RefCell::new(Vec::new());
    frame(&Context::default(), FRAME, vec![], |ui| {
        p.edits.frame(ui, |_, host| {
            read.borrow_mut().extend([
                (host.value("time"), host.default_value("time"), host.text("time")),
                (host.value("on"), host.default_value("on"), host.text("on")),
            ])
        });
    });
    let gui = nice_plug::context::gui::GuiContext::new(p.recorder.clone());
    let map = ParamMap::new(p.params.clone());
    let raw = Adapter::new(&map, &gui);
    let want: Vec<_> = ["time", "on"].map(|id| (raw.value(id), raw.default_value(id), raw.text(id))).into();
    assert_eq!(read.into_inner(), want);
    assert_eq!(p.recorder.take(), Vec::<String>::new(), "reading tells the host nothing");
}
