//! The crate's held edits through nice-plug's real VST3 wrapper, processing
//! audio, under each of the three modelled hosts (`vst3_host::model`): one
//! that hands back what it was told (Reaper), one that reads the plugin's
//! value back when the edit ends, and one that passes a change on only
//! while its edit is open (W-2b). After Kör's and Fläns's
//! `tests/b_pnl_11_host_models.rs` (their B-PNL-11's acceptance tests),
//! cut to the kinds of edit the crate must carry, on the tests' own panel
//! (`support::draw`) drawn inside `HeldEdits::frame` over the context the
//! wrapper gave (`vst3_host::panel`). Kör's and Fläns's own acceptance
//! tests stay theirs and run against the crate when they move onto it.
//!
//! Each test clicks, double-clicks or drags a control as a user does, a
//! panel frame for each step of the mouse, with the host's process calls
//! between the frames, then runs [`SETTLE`] more frames each followed by a
//! process call, and asks:
//! - whether the parameter is what the panel set, both as the plugin has it
//!   and as the controller reports it to the host (`getParamNormalized`);
//! - whether the host was told well-formed gestures: none begun while its
//!   own was open, none ended that wasn't open, none left open, and one
//!   begin–end pair per edit, on the edited parameter only.
//!
//! An edit is always followed by a frame before the next process call (a
//! 60 Hz panel draws more often than a host with long blocks calls
//! `process`), so an edit ended at the next frame without waiting for its
//! value is ended before the value has arrived, and is lost under the
//! read-back and while-open hosts. No test waits: the host's callbacks come
//! inside the panel's calls, and each frame and process call is made here,
//! in turn, on the panel's own clock ([`FRAME`]).

mod support;
mod vst3_host;

use std::cell::Cell;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};

use nice_plug::prelude::{Param, Params};
use nice_plug::wrapper::vst3::vst3::Steinberg::Vst::{
    IComponentHandler, IComponentHandlerTrait, IEditControllerTrait, ParamID, ParamValue,
};
use nice_plug::wrapper::vst3::vst3::Steinberg::{int32, tresult};
use nice_plug::wrapper::vst3::vst3::{Class, ComPtr, ComWrapper};
use panelwerk_nice::HeldEdits;
use panelwerk_panel::egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use panelwerk_panel::gesture::{DRAG_RANGE, PENDING_FOR};
use support::{panel_frame, Knobs, FRAME, MIX, ON, TIME};
use vst3_host::model::{handler, Model, ModelHost};
use vst3_host::panel::Panel;
use vst3_host::Vst3;

/// Samples in each process call: a small host block.
const BLOCK: usize = 64;

/// Frames run after an edit's last mouse step, each followed by a process
/// call, before it's checked. For an edit made in one instant: the first
/// frame comes before the value has reached the plugin (so an edit isn't
/// to end there); its process call carries what the host passes on (the
/// performed value); the second frame finds the plugin has the value, so
/// the held edit ends there, and its process call carries what the host
/// passes on then (the read-back host's value); the third shows that
/// nothing comes later to change it. Kör's `SETTLE`, for the same reason.
const SETTLE: usize = 3;

/// The frames from an edit to the process call that brings its value, in
/// the long-block path: the most there can be with that call still before
/// [`PENDING_FOR`] has passed, so that a cure that ends an edit a fixed
/// time after it, without looking at the value, ends it before the value
/// has arrived for any fixed time shorter than the limit. Kör's.
fn long_gap() -> usize {
    let gap = (PENDING_FOR / FRAME).ceil() as usize - 1;
    assert!(gap as f64 * FRAME < PENDING_FOR, "the value comes before the limit");
    gap
}

/// How far a value seen may be from the value set, normalised: a value
/// passes through the host as f64 and is held as an f32 plain value, from
/// which nice-plug works out the normalised one again, which moves it by a
/// few f32 steps (`f32::EPSILON`, 1.2e-7, near 1.0). Sixteen of them is far
/// below the smallest change here, a quarter of the range ([`DRAG_UP`]
/// over `DRAG_RANGE`), so a lost edit can't pass. Kör's and Rödalog v3's
/// `SAME`, for the same reason.
const SAME: f64 = 16.0 * f32::EPSILON as f64;

fn agrees(seen: f64, wanted: f32) -> bool {
    seen.is_finite() && wanted.is_finite() && (seen - f64::from(wanted)).abs() <= SAME
}

/// Mix before the double-click sets its default: not the default, so the
/// double-click changes it. Kör's `OFF_DEFAULT`.
const OFF_DEFAULT: f32 = 0.2;

/// A drag: [`DRAG_UP`] points up in [`DRAG_MOVES`] moves, a quarter of the
/// knob's travel (`DRAG_RANGE`, 200 points). Kör's.
const DRAG_MOVES: usize = 5;
const DRAG_UP: f32 = 50.0;

/// How far [`drag_time`] drags Time, in points: to a value whose
/// normalised form doesn't survive the plain value's round trip (found by
/// trying, as `held.rs`'s `TIME_UP`; the path checks that it still
/// doesn't). A twentieth of the travel, still far above [`SAME`].
const TIME_UP: f32 = 10.0;

/// What the plugin told the host of a gesture, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Told {
    Begin(ParamID),
    End(ParamID),
}

/// A modelled host's component handler with the plugin's begins and ends
/// also recorded: every call is passed on to the model's own handler
/// unchanged.
struct Tee {
    model: ComPtr<IComponentHandler>,
    told: Arc<Mutex<Vec<Told>>>,
}

impl Class for Tee {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for Tee {
    unsafe fn beginEdit(&self, id: ParamID) -> tresult {
        self.told.lock().unwrap().push(Told::Begin(id));
        unsafe { self.model.beginEdit(id) }
    }
    unsafe fn performEdit(&self, id: ParamID, value: ParamValue) -> tresult {
        unsafe { self.model.performEdit(id, value) }
    }
    unsafe fn endEdit(&self, id: ParamID) -> tresult {
        self.told.lock().unwrap().push(Told::End(id));
        unsafe { self.model.endEdit(id) }
    }
    unsafe fn restartComponent(&self, flags: int32) -> tresult {
        unsafe { self.model.restartComponent(flags) }
    }
}

/// The test panel over the stand-in, processing, under a modelled host.
struct Session {
    /// First, so they're dropped before the view closes (`Panel`'s drop),
    /// as a consumer's app goes with its window.
    edits: HeldEdits,
    /// The panel's egui context: its memory from frame to frame.
    ctx: Context,
    /// The panel's clock, in seconds: the time of the last frame.
    now: Cell<f64>,
    /// Before the plugin: it takes the controller's handler away, which
    /// ends the read-back host's hold on the controller.
    _panel: Panel,
    plugin: Vst3,
    host: Arc<ModelHost>,
    /// The handler the controller was given: the model's, through [`Tee`].
    _handler: ComPtr<IComponentHandler>,
    /// The gestures the plugin told the host of.
    told: Arc<Mutex<Vec<Told>>>,
    params: Arc<Knobs>,
}

/// One parameter as seen after an edit: what it was before, what the panel
/// set, what the plugin has, and what the controller tells the host.
struct Seen {
    id: &'static str,
    before: f32,
    wanted: f32,
    plugin: f32,
    host: f64,
}

impl Seen {
    fn holds(&self) -> bool {
        agrees(f64::from(self.plugin), self.wanted) && agrees(self.host, self.wanted)
    }
}

impl Session {
    fn new(model: Model) -> Self {
        let (inner, host) = handler(model);
        let told = Arc::default();
        let tee = ComWrapper::new(Tee { model: inner, told: Arc::clone(&told) });
        let handler = tee.to_com_ptr::<IComponentHandler>().unwrap();
        let panel = Panel::open(&handler);
        let plugin = panel.start();
        host.read_from(plugin.controller.clone());
        let params = panel.params();
        let edits = panel.edits();
        let s = Self {
            edits,
            ctx: Context::default(),
            now: Cell::new(0.0),
            _panel: panel,
            plugin,
            host,
            _handler: handler,
            told,
            params,
        };
        // The window's first frame: egui finds what's under the mouse from
        // the last frame's widgets.
        s.frame(vec![]);
        s
    }

    /// The panel's next frame, [`FRAME`] after the last, with `events`.
    fn frame(&self, events: Vec<Event>) {
        self.now.set(self.now.get() + FRAME);
        let (drawn, _) = panel_frame(&self.edits, &self.ctx, self.now.get(), events);
        assert!(drawn, "the window is open");
    }

    /// One process call, carrying what the host passes on.
    fn process(&self) {
        self.plugin.process(&self.host.take(), BLOCK);
    }

    /// A frame with `events`, then a process call: a step of the mouse
    /// with the host's audio running.
    fn step(&self, events: Vec<Event>) {
        self.frame(events);
        self.process();
    }

    /// [`SETTLE`] frames, each followed by a process call.
    fn settle(&self) {
        (0..SETTLE).for_each(|_| self.step(vec![]));
    }

    fn button(at: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    /// A click on the control at `rect`: the mouse onto it, pressed,
    /// released, a frame each, with a process call after each but the last
    /// (the click's edit), so the next frame comes first ([`Self::settle`]).
    fn click(&self, rect: Rect) {
        let at = rect.center();
        self.step(vec![Event::PointerMoved(at)]);
        self.step(vec![Self::button(at, true)]);
        self.frame(vec![Self::button(at, false)]);
    }

    /// A double-click on the control at `rect`, as [`Self::click`]: the
    /// edit comes with the second release.
    fn double_click(&self, rect: Rect) {
        let at = rect.center();
        self.step(vec![Event::PointerMoved(at)]);
        self.step(vec![Self::button(at, true)]);
        self.step(vec![Self::button(at, false)]);
        self.step(vec![Self::button(at, true)]);
        self.frame(vec![Self::button(at, false)]);
    }

    /// A drag on the knob at `rect`, `up` points up in [`DRAG_MOVES`]
    /// moves, a frame and a process call for each, then released on a
    /// frame of its own.
    fn drag_up(&self, rect: Rect, up: f32) {
        let from = rect.center();
        self.step(vec![Event::PointerMoved(from)]);
        self.step(vec![Self::button(from, true)]);
        let mut at = from;
        for _ in 0..DRAG_MOVES {
            at -= Vec2::new(0.0, up / DRAG_MOVES as f32);
            self.step(vec![Event::PointerMoved(at)]);
        }
        self.frame(vec![Self::button(at, false)]);
    }

    /// The host's ID of parameter `id`.
    fn hash(&self, id: &str) -> ParamID {
        let (_, ptr, _) = self.params.param_map().into_iter().find(|(i, _, _)| i == id).expect("a parameter");
        self.plugin.param(unsafe { ptr.name() })
    }

    /// Parameter `id` as the plugin has it, normalised.
    fn normalized(&self, id: &str) -> f32 {
        let (_, ptr, _) = self.params.param_map().into_iter().find(|(i, _, _)| i == id).expect("a parameter");
        unsafe { ptr.unmodulated_normalized_value() }
    }

    /// The host's own change of `id` to `normalized` (automation played
    /// back), in a process call, then settled: not a panel edit, so the
    /// same under every model.
    fn automate(&self, id: &str, normalized: f32) {
        self.plugin.process(&[(self.hash(id), f64::from(normalized))], BLOCK);
        self.settle();
        assert!(agrees(f64::from(self.normalized(id)), normalized), "the host's automation set {id}");
    }

    /// Parameter `id`, which was `before`, against `wanted`.
    fn seen(&self, id: &'static str, before: f32, wanted: f32) -> Seen {
        let host = unsafe { self.plugin.controller.getParamNormalized(self.hash(id)) };
        Seen { id, before, wanted, plugin: self.normalized(id), host }
    }
}

/// What a path did: each edit's parameter as seen once it settled, and how
/// many begin–end pairs each edited parameter may have been told.
struct Outcome {
    seen: Vec<Seen>,
    pairs: Vec<(&'static str, RangeInclusive<usize>)>,
}

/// Fails, naming the path, the model and every parameter that doesn't hold
/// with what was seen of it; or if any edit changed nothing, as one that
/// can't fail.
fn assert_hold(path: &str, model: Model, seen: &[Seen]) {
    assert!(!seen.is_empty(), "{path} checks something");
    for s in seen {
        assert!(!agrees(f64::from(s.before), s.wanted), "{path}: {} was already {}: no change", s.id, s.wanted);
    }
    let lost: Vec<String> = seen
        .iter()
        .filter(|s| !s.holds())
        .map(|s| format!("{}: set {}, the plugin has {}, the host reads {}", s.id, s.wanted, s.plugin, s.host))
        .collect();
    assert!(lost.is_empty(), "{path} under the {model:?} host: {} of {} not as set: {lost:#?}", lost.len(), seen.len());
}

/// Fails unless the host was told well-formed gestures: no parameter begun
/// while its gesture was open, none ended that wasn't open, none left open
/// once the edits have settled, and, for each parameter, a number of pairs
/// in its range in `pairs`, none for any other.
fn assert_gestures(path: &str, s: &Session, pairs: &[(&str, RangeInclusive<usize>)]) {
    let model = s.host.model();
    let told = s.told.lock().unwrap().clone();
    let mut open: Vec<ParamID> = Vec::new();
    for t in &told {
        match *t {
            Told::Begin(id) => {
                assert!(
                    !open.contains(&id),
                    "{path} under the {model:?} host: {id:#x} begun again while open: {told:?}"
                );
                open.push(id);
            }
            Told::End(id) => {
                let at = open.iter().position(|&p| p == id);
                assert!(at.is_some(), "{path} under the {model:?} host: {id:#x} ended but not open: {told:?}");
                open.remove(at.unwrap());
            }
        }
    }
    assert!(open.is_empty(), "{path} under the {model:?} host: gestures left open: {open:#x?}");
    let begun = |hash: ParamID| told.iter().filter(|t| **t == Told::Begin(hash)).count();
    let hashes: Vec<ParamID> = pairs.iter().map(|(id, _)| s.hash(id)).collect();
    for ((id, range), hash) in pairs.iter().zip(&hashes) {
        let n = begun(*hash);
        assert!(range.contains(&n), "{path} under the {model:?} host: {n} begin–end pairs on {id}, not {range:?}");
    }
    let others: Vec<&Told> =
        told.iter().filter(|t| !matches!(t, Told::Begin(h) | Told::End(h) if hashes.contains(h))).collect();
    assert!(others.is_empty(), "{path} under the {model:?} host: gestures on parameters it didn't edit: {others:?}");
}

// The paths, each from the stand-in's defaults: the switch off, Mix at
// 50 %, Time at 3.505 ms.

/// One click on the switch: on. An edit made in one instant, which the
/// read-back and while-open hosts undo unless it is held.
fn click_on(s: &Session) -> Outcome {
    let before = s.normalized("on");
    s.click(ON);
    s.settle();
    Outcome { seen: vec![s.seen("on", before, 1.0)], pairs: vec![("on", 1..=1)] }
}

/// As [`click_on`], but the host's next process call comes only after a
/// long block ([`long_gap`] frames): an edit ended a fixed time after it
/// rather than once the plugin has the value is lost.
fn long_block_click_on(s: &Session) -> Outcome {
    let before = s.normalized("on");
    s.click(ON);
    (1..long_gap()).for_each(|_| s.frame(vec![]));
    s.settle();
    Outcome { seen: vec![s.seen("on", before, 1.0)], pairs: vec![("on", 1..=1)] }
}

/// A double-click on Mix, from [`OFF_DEFAULT`]: its default, 50 %.
fn double_click_mix(s: &Session) -> Outcome {
    s.automate("mix", OFF_DEFAULT);
    s.told.lock().unwrap().clear();
    s.double_click(MIX);
    s.settle();
    Outcome {
        seen: vec![s.seen("mix", OFF_DEFAULT, s.params.mix.default_normalized_value())],
        pairs: vec![("mix", 1..=1)],
    }
}

/// A drag up on Mix: a gesture that spans audio blocks, so the plugin has
/// its last move before the end.
fn drag_mix(s: &Session) -> Outcome {
    let before = s.normalized("mix");
    s.drag_up(MIX, DRAG_UP);
    s.settle();
    Outcome { seen: vec![s.seen("mix", before, before + DRAG_UP / DRAG_RANGE)], pairs: vec![("mix", 1..=1)] }
}

/// A drag up on Time by [`TIME_UP`], to a value whose normalised form
/// doesn't survive the plain value's round trip (the path checks that it
/// still doesn't): the edit ends at the frame after the release only
/// because the adapter's `shows` compares plain values; with the trait's
/// default it would end only at `PENDING_FOR`, after the settle, and be
/// left open here.
fn drag_time(s: &Session) -> Outcome {
    let before = s.normalized("time");
    let wanted = before + TIME_UP / DRAG_RANGE;
    let time = &s.params.time;
    assert_ne!(time.preview_normalized(time.preview_plain(wanted)), wanted, "the value set rounds");
    s.drag_up(TIME, TIME_UP);
    s.settle();
    Outcome { seen: vec![s.seen("time", before, wanted)], pairs: vec![("time", 1..=1)] }
}

/// Runs `path` under `model` and asserts every parameter it set holds, and
/// that the host was told well-formed gestures, one pair an edit.
fn check(name: &str, path: fn(&Session) -> Outcome, model: Model) {
    let s = Session::new(model);
    let outcome = path(&s);
    assert_hold(name, model, &outcome.seen);
    assert_gestures(name, &s, &outcome.pairs);
}

/// One test per path and model.
macro_rules! cells {
    ($($test:ident: $path:ident, $model:ident;)*) => {
        $(
            #[test]
            fn $test() {
                check(stringify!($path), $path, Model::$model);
            }
        )*
    };
}

cells! {
    w_2b_echo_click: click_on, Echo;
    w_2b_echo_long_block_click: long_block_click_on, Echo;
    w_2b_echo_double_click: double_click_mix, Echo;
    w_2b_echo_drag: drag_mix, Echo;
    w_2b_echo_drag_rounding: drag_time, Echo;
    w_2b_read_back_click: click_on, ReadBack;
    w_2b_read_back_long_block_click: long_block_click_on, ReadBack;
    w_2b_read_back_double_click: double_click_mix, ReadBack;
    w_2b_read_back_drag: drag_mix, ReadBack;
    w_2b_read_back_drag_rounding: drag_time, ReadBack;
    w_2b_while_open_click: click_on, WhileOpen;
    w_2b_while_open_long_block_click: long_block_click_on, WhileOpen;
    w_2b_while_open_double_click: double_click_mix, WhileOpen;
    w_2b_while_open_drag: drag_mix, WhileOpen;
    w_2b_while_open_drag_rounding: drag_time, WhileOpen;
}

// The models' own checks: each does what its doc says, told directly
// through its component handler, with a stand-in for the controller's
// `getParamNormalized` (Kör's, copied with the models).

/// A parameter ID for the models' own checks.
const P: u32 = 7;
/// What the stand-in controller always reports, unlike any value performed.
const READ: f64 = 0.125;

fn bare(model: Model) -> (ComPtr<IComponentHandler>, Arc<ModelHost>) {
    let (handler, host) = handler(model);
    host.read_with(Box::new(|_| READ));
    (handler, host)
}

#[test]
fn w_2b_the_echoing_host_passes_on_the_last_value_performed_once() {
    let (h, host) = bare(Model::Echo);
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.25);
        h.performEdit(P, 0.75);
        h.endEdit(P);
    }
    assert_eq!(host.take(), vec![(P, 0.75)], "the last, not the controller's");
    assert_eq!(host.take(), vec![], "once");
    // During a gesture too.
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.5);
    }
    assert_eq!(host.take(), vec![(P, 0.5)]);
    unsafe { h.endEdit(P) };
    assert_eq!(host.take(), vec![]);
}

#[test]
fn w_2b_the_read_back_host_ends_on_what_the_controller_says_at_end_edit() {
    let (h, host) = bare(Model::ReadBack);
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.75);
        h.endEdit(P);
    }
    assert_eq!(host.take(), vec![(P, 0.75), (P, READ)], "the performed value, then the one read back");
    // A gesture over two process calls: the moves as they come, then the
    // read-back.
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.5);
    }
    assert_eq!(host.take(), vec![(P, 0.5)]);
    unsafe {
        h.performEdit(P, 0.6);
        h.endEdit(P);
    }
    assert_eq!(host.take(), vec![(P, 0.6), (P, READ)]);
    assert_eq!(host.take(), vec![]);
}

#[test]
fn w_2b_the_while_open_host_forwards_only_gestures_still_open() {
    let (h, host) = bare(Model::WhileOpen);
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.75);
        h.endEdit(P);
    }
    assert_eq!(host.take(), vec![], "begun, performed and ended between two calls: never forwarded");
    unsafe { h.performEdit(P, 0.75) };
    assert_eq!(host.take(), vec![], "nor one outside a gesture");
    unsafe {
        h.beginEdit(P);
        h.performEdit(P, 0.5);
    }
    assert_eq!(host.take(), vec![(P, 0.5)], "open at the call: forwarded");
    unsafe {
        h.performEdit(P, 0.6);
        h.endEdit(P);
    }
    assert_eq!(host.take(), vec![], "the move ended with its gesture before the next call is lost");
}

#[test]
fn w_2b_a_value_seen_agrees_only_within_same_and_never_as_nan() {
    assert!(agrees(0.5, 0.5));
    assert!(agrees(0.5 + SAME / 2.0, 0.5));
    assert!(!agrees(0.5 + 2.0 * SAME, 0.5));
    assert!(!agrees(f64::NAN, 0.5));
    assert!(!agrees(0.5, f32::NAN));
    assert!(!agrees(f64::NAN, f32::NAN));
    assert!(!agrees(f64::INFINITY, f32::INFINITY));
    // A switch's NaN: its plain value would read as off, and must not pass.
    let s = Seen { id: "on", before: 1.0, wanted: 0.0, plugin: f32::NAN, host: 0.0 };
    assert!(!s.holds());
    let s = Seen { id: "on", before: 1.0, wanted: 0.0, plugin: 0.0, host: f64::NAN };
    assert!(!s.holds());
}
