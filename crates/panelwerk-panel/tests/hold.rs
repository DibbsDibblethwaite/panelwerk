//! PW-HOLD: held edits (`hold::Held`), the cure for hosts that undo an edit
//! the panel makes in one instant (Kör's and Fläns's B-PNL-11, Rödalog's
//! B-PAR-11), and `ParamHost::shows`, what a held edit waits for.

use std::cell::Cell;

use egui::{pos2, vec2, Event, Id, Modifiers, PointerButton, Pos2, Rect, Sense, Ui};
use panelwerk_panel::gesture::{self, PENDING_FOR};
use panelwerk_panel::hold::Held;
use panelwerk_panel::ParamHost;
use panelwerk_testkit::{FakeHost, Model, Window, MODELS};

/// A host that doesn't override `shows`.
struct Plain(Cell<f32>);

impl ParamHost for Plain {
    fn value(&self, _: &str) -> f32 {
        self.0.get()
    }
    fn default_value(&self, _: &str) -> f32 {
        0.0
    }
    fn text(&self, _: &str) -> String {
        String::new()
    }
    fn begin(&self, _: &str) {}
    fn set(&self, _: &str, normalized: f32) {
        self.0.set(normalized)
    }
    fn end(&self, _: &str) {}
}

#[test]
fn pw_hold_shows_by_default_compares_the_normalised_value() {
    let host = Plain(Cell::new(0.25));
    assert!(host.shows("mix", 0.25));
    assert!(!host.shows("mix", 0.2500001));
    host.set("mix", 0.5);
    assert!(host.shows("mix", 0.5));
}

// ---- Held, driven directly: the panel's clock given to each frame ----

/// The panel's clock where a direct test starts: a whole number of seconds,
/// so `T0 + PENDING_FOR - T0` is exactly `PENDING_FOR`.
const T0: f64 = 1.0;
/// A frame well inside [`PENDING_FOR`] of `T0`.
const SOON: f64 = T0 + PENDING_FOR * 0.5;
/// A frame just inside [`PENDING_FOR`] of `T0`.
const NEARLY: f64 = T0 + PENDING_FOR * 0.9;
/// The first frame at which an edit ended at `T0` has waited the limit.
const LIMIT: f64 = T0 + PENDING_FOR;

fn lagging() -> FakeHost {
    let host = FakeHost::new(&[("a", 0.0), ("b", 0.0)]);
    host.lagging.set(true);
    host
}

/// An edit made in one instant through `held`, as `gesture::set_once` makes
/// it.
fn instant(held: &Held, host: &FakeHost, id: &str, to: f32) {
    held.begin(host, id);
    held.set(host, id, to);
    held.end(id);
}

#[test]
fn pw_hold_an_instant_edit_is_ended_for_the_host_only_once_its_value_arrives() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    assert_eq!(host.take(), ["begin a", "set a"], "the end waits");
    assert!(held.frame(&host, SOON), "it waits: the panel is to ask for another frame");
    assert!(host.take().is_empty(), "the value hasn't arrived");
    host.put("a", 1.0);
    assert!(!held.frame(&host, NEARLY), "nothing waits now");
    assert_eq!(host.take(), ["end a"]);
    held.frame(&host, LIMIT);
    assert!(host.take().is_empty(), "ended once");
}

#[test]
fn pw_hold_an_edit_whose_value_never_arrives_is_ended_at_pending_for() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    host.take();
    assert!(held.frame(&host, NEARLY));
    assert!(host.take().is_empty());
    assert!(!held.frame(&host, LIMIT));
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_a_new_panel_clock_ends_a_waiting_edit() {
    // A clock before the edit's end: a new window's, starting again.
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, LIMIT);
    instant(&held, &host, "a", 1.0);
    host.take();
    held.frame(&host, T0);
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_a_gesture_with_nothing_set_is_ended_at_the_next_frame() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    held.begin(&host, "a");
    held.end("a");
    assert_eq!(host.take(), ["begin a"]);
    assert!(!held.frame(&host, SOON));
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_a_gesture_with_nothing_set_on_a_waiting_edit_keeps_waiting_for_its_value() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    held.frame(&host, SOON);
    // A click on the control that sets nothing, while the edit waits.
    held.begin(&host, "a");
    held.end("a");
    held.frame(&host, NEARLY);
    assert_eq!(host.take(), ["begin a", "set a"], "still waiting for 1.0, no second begin");
    host.put("a", 1.0);
    held.frame(&host, NEARLY);
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_an_edit_while_one_waits_continues_its_gesture() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    held.frame(&host, SOON);
    // A second click: the host never sees a gesture begun twice, and the
    // edit now waits for the second value, from the second end.
    instant(&held, &host, "a", 0.5);
    assert_eq!(host.take(), ["begin a", "set a", "set a"]);
    host.put("a", 1.0);
    held.frame(&host, SOON);
    assert!(host.take().is_empty(), "the first value isn't the one waited for");
    held.frame(&host, LIMIT);
    assert!(host.take().is_empty(), "the limit counts from the second end");
    host.put("a", 0.5);
    held.frame(&host, LIMIT);
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_an_end_too_many_does_nothing() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    held.end("a");
    held.frame(&host, SOON);
    assert!(host.take().is_empty(), "no gesture: nothing told");
    instant(&held, &host, "a", 1.0);
    held.frame(&host, NEARLY);
    // An end on a waiting edit: neither ends it nor starts its wait again.
    held.end("a");
    held.frame(&host, NEARLY);
    assert_eq!(host.take(), ["begin a", "set a"]);
    held.frame(&host, LIMIT + SOON - T0);
    assert_eq!(host.take(), ["end a"], "ended at the limit of the first end, made at SOON");
}

#[test]
fn pw_hold_held_can_be_owned_by_a_consumers_app() {
    // nice-plug-egui's app is `Send`; the window's thread only uses it.
    fn send<T: Send>() {}
    send::<Held>();
}

#[test]
fn pw_hold_an_edit_ends_once_the_plugin_shows_its_value_as_the_plugin_rounds_it() {
    // The host's `shows`, not an exact comparison: a stepped parameter set
    // to 0.3 holds 0.25 (4 steps), and that is the value arriving.
    let host = FakeHost::new(&[("mode", 0.0)]).stepped(&[("mode", 4)]);
    let held = Held::new();
    held.frame(&host, T0);
    instant(&held, &host, "mode", 0.3);
    assert_eq!(host.get("mode"), 0.25);
    host.take();
    assert!(!held.frame(&host, SOON), "nothing waits");
    assert_eq!(host.take(), ["end mode"], "at the next frame, not at the limit");
}

/// A host whose `shows` calls back into the held edits, as a consumer's
/// adapter might through shared state.
struct Reentrant<'a> {
    host: FakeHost,
    held: &'a Held,
}

impl ParamHost for Reentrant<'_> {
    fn value(&self, id: &str) -> f32 {
        self.host.value(id)
    }
    fn default_value(&self, id: &str) -> f32 {
        self.host.default_value(id)
    }
    fn text(&self, id: &str) -> String {
        self.host.text(id)
    }
    fn begin(&self, id: &str) {
        self.host.begin(id)
    }
    fn set(&self, id: &str, normalized: f32) {
        self.host.set(id, normalized)
    }
    fn end(&self, id: &str) {
        self.host.end(id)
    }
    fn shows(&self, id: &str, normalized: f32) -> bool {
        // An end too many: changes nothing, but needs the state.
        self.held.end(id);
        self.host.shows(id, normalized)
    }
}

#[test]
fn pw_hold_a_shows_that_calls_back_into_the_held_edits_is_safe() {
    let held = Held::new();
    let raw = Reentrant { host: lagging(), held: &held };
    held.frame(&raw, T0);
    instant(&held, &raw.host, "a", 1.0);
    raw.host.take();
    assert!(held.frame(&raw, SOON), "still waiting, and no double borrow");
    raw.host.put("a", 1.0);
    assert!(!held.frame(&raw, SOON));
    assert_eq!(raw.host.take(), ["end a"]);
}

#[test]
fn pw_hold_a_set_outside_a_gesture_is_passed_on_and_held_for_nothing() {
    let host = FakeHost::new(&[("a", 0.0)]);
    let held = Held::new();
    held.set(&host, "a", 0.25);
    assert_eq!(host.get("a"), 0.25);
    assert!(!held.frame(&host, T0));
    assert_eq!(host.take(), ["set a"]);
}

#[test]
fn pw_hold_edits_ended_at_one_moment_reach_the_host_together() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    instant(&held, &host, "b", 1.0);
    host.take();
    host.put("a", 1.0);
    held.frame(&host, SOON);
    assert!(host.take().is_empty(), "b hasn't arrived, so a waits with it");
    host.put("b", 1.0);
    held.frame(&host, SOON);
    assert_eq!(host.take(), ["end a", "end b"], "in one frame, in the order begun");
}

#[test]
fn pw_hold_edits_ended_at_one_moment_are_ended_together_at_the_limit() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    instant(&held, &host, "b", 1.0);
    host.take();
    host.put("a", 1.0);
    held.frame(&host, NEARLY);
    assert!(host.take().is_empty());
    held.frame(&host, LIMIT);
    assert_eq!(host.take(), ["end a", "end b"], "b never arrived");
}

#[test]
fn pw_hold_edits_ended_at_different_moments_end_each_when_ready() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "a", 1.0);
    held.frame(&host, SOON);
    instant(&held, &host, "b", 1.0);
    host.take();
    host.put("b", 1.0);
    held.frame(&host, NEARLY);
    assert_eq!(host.take(), ["end b"], "not held up by a, ended earlier and still waiting");
    host.put("a", 1.0);
    held.frame(&host, NEARLY);
    assert_eq!(host.take(), ["end a"]);
}

#[test]
fn pw_hold_close_ends_every_gesture_open_or_waiting_once_in_the_order_begun() {
    let (host, held) = (lagging(), Held::new());
    held.frame(&host, T0);
    instant(&held, &host, "b", 1.0);
    held.begin(&host, "a");
    held.set(&host, "a", 0.5);
    host.take();
    held.close(&host);
    assert_eq!(host.take(), ["end b", "end a"]);
    held.frame(&host, LIMIT);
    held.close(&host);
    held.end("a");
    held.frame(&host, LIMIT);
    assert!(host.take().is_empty(), "each ended once");
}

#[test]
fn pw_hold_begins_nest_so_an_instant_edit_during_a_drag_does_not_cut_it_off() {
    for model in MODELS {
        let host = FakeHost::new(&[("a", 0.0)]).modelled(model);
        let held = Held::new();
        held.frame(&host, T0);
        // A drag begins and moves.
        held.begin(&host, "a");
        held.set(&host, "a", 0.25);
        host.process();
        held.frame(&host, SOON);
        // An instant edit on the same parameter (a consumer's key) inside it.
        instant(&held, &host, "a", 0.5);
        host.process();
        assert!(!held.frame(&host, SOON), "{model:?}: the drag is still open, nothing waits");
        // The drag goes on, and the host still passes its moves on.
        held.set(&host, "a", 0.75);
        host.process();
        assert_eq!(host.get("a"), 0.75, "{model:?}: the drag wasn't cut off");
        held.frame(&host, NEARLY);
        assert_eq!(host.take(), ["begin a", "set a", "set a", "set a"], "{model:?}: one begin, no end yet");
        held.end("a");
        held.frame(&host, NEARLY);
        host.process();
        assert_eq!(host.take(), ["end a"], "{model:?}");
        assert_eq!(host.get("a"), 0.75, "{model:?}");
    }
}

// ---- Held behind a panel's host, over every gesture path ----

const KNOB: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(92.0, 92.0) };
const SWITCH: Rect = Rect { min: pos2(120.0, 20.0), max: pos2(192.0, 92.0) };
const MODE: Rect = Rect { min: pos2(220.0, 20.0), max: pos2(292.0, 92.0) };
const BOTH: Rect = Rect { min: pos2(320.0, 20.0), max: pos2(392.0, 92.0) };

/// A plugin's view of its host for the panel: the modelled host, and the
/// held edits wired in between (as a consumer's adapter wires them) or not
/// (a consumer that hasn't).
struct Rig {
    host: FakeHost,
    held: Held,
    wired: bool,
    /// Audio running: the host's process step comes before a frame...
    processing: Cell<bool>,
    /// ...every this many frames (audio buffers slower than the panel's
    /// frames leave frames with no process step between them).
    every: usize,
    passes: Cell<usize>,
}

impl Rig {
    fn new(model: Option<Model>, wired: bool) -> Self {
        Rig::every(model, wired, 1)
    }

    fn every(model: Option<Model>, wired: bool, every: usize) -> Self {
        let host = FakeHost::new(&[("mix", 0.5), ("on", 0.0), ("mode", 0.0), ("a", 0.0), ("b", 0.0)])
            .stepped(&[("on", 1), ("mode", 2), ("a", 1), ("b", 1)])
            .with(&[("mix", 0.2)]);
        let host = match model {
            Some(m) => host.modelled(m),
            None => host,
        };
        Rig { host, held: Held::new(), wired, processing: Cell::new(model.is_some()), every, passes: Cell::new(0) }
    }
}

impl ParamHost for Rig {
    fn value(&self, id: &str) -> f32 {
        self.host.value(id)
    }
    fn default_value(&self, id: &str) -> f32 {
        self.host.default_value(id)
    }
    fn text(&self, id: &str) -> String {
        self.host.text(id)
    }
    fn begin(&self, id: &str) {
        match self.wired {
            true => self.held.begin(&self.host, id),
            false => self.host.begin(id),
        }
    }
    fn set(&self, id: &str, normalized: f32) {
        match self.wired {
            true => self.held.set(&self.host, id, normalized),
            false => self.host.set(id, normalized),
        }
    }
    fn end(&self, id: &str) {
        match self.wired {
            true => self.held.end(id),
            false => self.host.end(id),
        }
    }
}

/// The panel: a knob for "mix", a switch for "on", a mode button setting
/// "mode" to its last choice, and a button setting "a" and "b" in one click.
/// Each pass, the host processes first (if audio runs and it's due), then
/// the held edits get the frame, before anything is drawn, as a consumer's
/// `App` calls it.
fn panel(rig: &Rig) -> impl FnMut(&mut Ui) + '_ {
    move |ui: &mut Ui| {
        rig.passes.set(rig.passes.get() + 1);
        if rig.processing.get() && rig.passes.get().is_multiple_of(rig.every) {
            rig.host.process();
        }
        if rig.wired {
            rig.held.frame(&rig.host, ui.input(|i| i.time));
        }
        let host: &dyn ParamHost = rig;
        let k = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
        gesture::knob(ui, host, &k, "mix");
        let s = ui.interact(SWITCH, Id::unique("switch"), Sense::click());
        gesture::toggle_on_click(ui, host, &s, "on");
        if ui.interact(MODE, Id::unique("mode"), Sense::click()).clicked() {
            gesture::set_once(ui, host, "mode", 1.0);
        }
        if ui.interact(BOTH, Id::unique("both"), Sense::click()).clicked() {
            gesture::set_once(ui, host, "a", 1.0);
            gesture::set_once(ui, host, "b", 1.0);
        }
    }
}

/// Every way `gesture.rs` makes a host gesture.
#[derive(Clone, Copy, Debug)]
enum Path {
    /// `gesture::knob`'s drag: 50 points up.
    Drag,
    /// `gesture::knob`'s double-click: the default.
    DoubleClick,
    /// `gesture::toggle_on_click`.
    Toggle,
    /// `gesture::set_once`, as a mode button calls it.
    SetOnce,
}

const PATHS: [Path; 4] = [Path::Drag, Path::DoubleClick, Path::Toggle, Path::SetOnce];

impl Path {
    /// The parameter it sets and the value it should leave.
    fn want(self) -> (&'static str, f32) {
        match self {
            // From 0.2, 50 points of DRAG_RANGE up.
            Path::Drag => ("mix", 0.2 + 50.0 / gesture::DRAG_RANGE),
            Path::DoubleClick => ("mix", 0.5),
            Path::Toggle => ("on", 1.0),
            Path::SetOnce => ("mode", 1.0),
        }
    }

    fn make(self, w: &mut Window, rig: &Rig) {
        let run = &mut panel(rig);
        match self {
            Path::Drag => w.drag(KNOB.center(), &[vec2(0.0, -25.0), vec2(0.0, -25.0)], Modifiers::NONE, run),
            Path::DoubleClick => w.double_click(KNOB.center(), run),
            Path::Toggle => w.click(SWITCH.center(), run),
            Path::SetOnce => w.click(MODE.center(), run),
        }
    }
}

fn window(rig: &Rig) -> Window {
    let mut w = Window::new(vec2(400.0, 120.0));
    w.idle(&mut panel(rig));
    w
}

/// The host's process steps come every 1, 2 or 3 frames.
const CADENCES: [usize; 3] = [1, 2, 3];

/// A few frames more, then the host's last process step: whatever the host
/// was told at the last frame (a read-back value) has reached the plugin.
fn settle(w: &mut Window, rig: &Rig) {
    for _ in 0..2 * rig.every {
        w.idle(&mut panel(rig));
    }
    rig.host.process();
}

/// Every model, path and cadence.
fn cases() -> impl Iterator<Item = (Model, Path, usize)> {
    MODELS.into_iter().flat_map(|m| PATHS.into_iter().flat_map(move |p| CADENCES.map(|e| (m, p, e))))
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

/// One begin and one end of `id`, first and last, sets of `id` between.
fn one_gesture(gestures: &[String], id: &str) {
    assert_eq!(gestures.first(), Some(&format!("begin {id}")), "{gestures:?}");
    assert_eq!(gestures.last(), Some(&format!("end {id}")), "{gestures:?}");
    assert_eq!(gestures.iter().filter(|g| g.starts_with("begin")).count(), 1, "{gestures:?}");
    assert_eq!(gestures.iter().filter(|g| g.starts_with("end")).count(), 1, "{gestures:?}");
    assert!(gestures.iter().filter(|g| g.starts_with("set")).all(|g| *g == format!("set {id}")), "{gestures:?}");
}

#[test]
fn pw_hold_without_held_an_instant_edit_is_lost_under_read_back_and_while_open() {
    // The fault, as the modelled hosts make it: a consumer not wired in.
    for (model, path, every) in cases() {
        let rig = Rig::every(Some(model), false, every);
        let mut w = window(&rig);
        path.make(&mut w, &rig);
        settle(&mut w, &rig);
        let (id, want) = path.want();
        let kept = near(rig.host.get(id), want);
        let lost = !matches!(path, Path::Drag) && model != Model::Echo;
        assert_eq!(kept, !lost, "{model:?} {path:?} every {every}: {} for {want}", rig.host.get(id));
        one_gesture(&rig.host.take(), id);
    }
}

#[test]
fn pw_hold_with_held_every_gesture_path_survives_every_modelled_host() {
    for (model, path, every) in cases() {
        let rig = Rig::every(Some(model), true, every);
        let mut w = window(&rig);
        path.make(&mut w, &rig);
        settle(&mut w, &rig);
        let (id, want) = path.want();
        let case = format!("{model:?} {path:?} every {every}");
        assert!(near(rig.host.get(id), want), "{case}: {} for {want}", rig.host.get(id));
        one_gesture(&rig.host.take(), id);
        settle(&mut w, &rig);
        assert!(near(rig.host.get(id), want), "{case}: stays");
        assert!(rig.host.take().is_empty(), "{case}: nothing more told");
    }
}

#[test]
fn pw_hold_with_held_and_no_processing_an_edit_ends_at_the_next_frame() {
    // A host not processing: nice-plug applies the set at once.
    for path in PATHS {
        let rig = Rig::new(None, true);
        let mut w = window(&rig);
        path.make(&mut w, &rig);
        let (id, want) = path.want();
        assert!(near(rig.host.get(id), want), "{path:?}");
        one_gesture(&rig.host.take(), id);
    }
    let rig = Rig::new(None, true);
    let mut w = window(&rig);
    press_release(&mut w, &rig, SWITCH.center());
    assert_eq!(rig.host.take(), ["begin on", "set on"], "the end not in the frame that set");
    w.idle(&mut panel(&rig));
    assert_eq!(rig.host.take(), ["end on"], "at the next");
}

/// A click in two passes (pointer there, then down and up in the next
/// two), with no passes after: quicker than `Window::click`, so a second
/// click comes well inside [`PENDING_FOR`] of the first.
fn press_release(w: &mut Window, rig: &Rig, at: Pos2) {
    let press =
        |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    w.pass(vec![Event::PointerMoved(at)], Modifiers::NONE, &mut panel(rig));
    w.pass(vec![press(true)], Modifiers::NONE, &mut panel(rig));
    w.pass(vec![press(false)], Modifiers::NONE, &mut panel(rig));
}

#[test]
fn pw_hold_a_second_click_while_the_first_waits_continues_its_gesture() {
    for model in MODELS {
        let rig = Rig::new(Some(model), true);
        let mut w = window(&rig);
        // The host stops passing anything on: the first click's edit waits.
        rig.processing.set(false);
        press_release(&mut w, &rig, SWITCH.center());
        press_release(&mut w, &rig, SWITCH.center());
        rig.processing.set(true);
        settle(&mut w, &rig);
        // The second click toggled from the value the first set (gesture::
        // latest), back off: one gesture, two sets.
        assert_eq!(rig.host.get("on"), 0.0, "{model:?}");
        assert_eq!(rig.host.take(), ["begin on", "set on", "set on", "end on"], "{model:?}");
    }
}

#[test]
fn pw_hold_three_clicks_while_the_host_waits_leave_the_switch_on() {
    for model in MODELS {
        let rig = Rig::new(Some(model), true);
        let mut w = window(&rig);
        rig.processing.set(false);
        for _ in 0..3 {
            press_release(&mut w, &rig, SWITCH.center());
        }
        rig.processing.set(true);
        settle(&mut w, &rig);
        assert_eq!(rig.host.get("on"), 1.0, "{model:?}");
        // The second click set off, which the plugin already has, so that
        // gesture ended at the next frame and the third began its own.
        let want = ["begin on", "set on", "set on", "end on", "begin on", "set on", "end on"];
        assert_eq!(rig.host.take(), want, "{model:?}");
    }
}

#[test]
fn pw_hold_a_drag_on_a_knob_whose_double_click_waits_continues_its_gesture() {
    for model in MODELS {
        let rig = Rig::new(Some(model), true);
        let mut w = window(&rig);
        rig.processing.set(false);
        w.double_click(KNOB.center(), &mut panel(&rig));
        w.drag(KNOB.center(), &[vec2(0.0, -25.0), vec2(0.0, -25.0)], Modifiers::NONE, &mut panel(&rig));
        rig.processing.set(true);
        settle(&mut w, &rig);
        // The drag started from the default the double-click set.
        assert!(near(rig.host.get("mix"), 0.5 + 50.0 / gesture::DRAG_RANGE), "{model:?}: {}", rig.host.get("mix"));
        one_gesture(&rig.host.take(), "mix");
    }
}

#[test]
fn pw_hold_one_click_setting_two_parameters_ends_both_in_one_frame() {
    for model in MODELS {
        let rig = Rig::new(Some(model), true);
        let mut w = window(&rig);
        w.click(BOTH.center(), &mut panel(&rig));
        settle(&mut w, &rig);
        assert_eq!((rig.host.get("a"), rig.host.get("b")), (1.0, 1.0), "{model:?}");
        assert_eq!(rig.host.take(), ["begin a", "set a", "begin b", "set b", "end a", "end b"], "{model:?}");
    }
}
