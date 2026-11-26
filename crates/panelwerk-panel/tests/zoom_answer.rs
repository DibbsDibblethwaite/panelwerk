//! The zoom drift's cure (Ian, 2026-11-26, ZOOM-DRIFT-CURE; the design and
//! its fact-check in `docs/proposals/`, the rule in `docs/tasks-v0.6.md`):
//! the size a window gets in answer to the panel's own request is never a
//! new zoom and is never asked for again; a size the host gives at any other
//! time is the host's own and the zoom follows it; a menu choice is told to
//! the plugin only once the window has taken it.
//!
//! The window is modelled as egui-baseview and baseview run it (the design's
//! §1): egui is given the window's pixels over the display's scale and the
//! zoom egui-baseview last saw (the last pass's); after a pass, a changed
//! zoom or an `InnerSize` command resizes the window, and only a size that
//! differs from the window's reaches the host as a request (baseview returns
//! early on an unchanged size). The child takes the size asked at once, as
//! egui-baseview's own resize does; the host's answer then lands as the
//! test's [`Answer`] says, before the next pass or, for a late answer, one
//! pass later. egui is given the display's scale as egui-baseview gives it
//! (the test kit's `Window::display_scale`, Ian, 2026-11-26, ZOOM-DRAG-STEP,
//! point 5), so `follow`'s sizes in pixels are checked at 125 %.

use std::cell::RefCell;

use egui::{vec2, Event, Modifiers, PointerButton, Pos2, Ui, Vec2, ViewportCommand, ViewportId};
use panelwerk_panel::zoom::{self, Range, ZoomHost, ANSWER_WITHIN};
use panelwerk_testkit::Window;

/// A range wide enough for Rödalog's 90 % and the menu sizes the tests
/// choose; a test range, not a consumer's.
const RANGE: Range = Range::new(0.6, 3.0, &[0.75, 0.8, 1.0, 1.25, 1.5, 1.75, 2.0]);
/// Kör's panel in points, the size the other zoom tests use.
const BASE: Vec2 = vec2(520.0, 280.0);
/// Rödalog's panel in points (its B-PNL-39), for its 90 % runaway.
const ROEDALOG: Vec2 = vec2(1520.0, 548.0);
/// Ian's display scaling, 125 % (Rödalog's and Klinga's Windows runs).
const SCALE_125: f32 = 1.25;
/// Where the menu's first row is drawn.
const MENU_AT: Pos2 = Pos2::new(12.0, 32.0);
/// Passes to run for a drift to show: v0.5.0 made 20 requests in 40 (the
/// fact-check's late-answer probe, `docs/proposals/zoom-drift-cure.md`'s
/// change 2), so 40 shows many steps of it. Not a standard; the count only
/// has to be well past one request.
const ROUNDS: usize = 40;
/// The passes over which a settled window keeps its size: more than the two
/// passes a request and its answer take, so a resize every pass shows.
const SETTLED_OVER: usize = 4;
/// The design's cure: at most one request per zoom (the design's §4, (iii),
/// "face 2: at most one request per zoom", `docs/proposals/zoom-drift-cure.md`;
/// ZOOM-DRIFT-CURE, the card's "What the cure does").
const ONE_REQUEST: usize = 1;
/// How far egui's input time moves each pass, in seconds: a tenth, as the
/// test kit's window steps it. Named here, as the wait test's gaps are
/// worked from it ([`ANSWER_WITHIN`] against the time between two passes).
const FRAME: f64 = 0.1;
/// A display at 300 %, for the display-scale test: there a pixel is a third
/// of a point, inside [`zoom`]'s half-pixel tolerance if sizes were worked in
/// points, so a size worked without the display's scale shows. At 125 % or
/// 200 % a pixel is half a point or more, and egui's rounding of the screen
/// to 1/32 point decides which side of the tolerance it falls.
const SCALE_300: f32 = 3.0;

/// The plugin's side: every zoom told it, by which call, and whether its
/// window code pairs requests and answers itself (the design's (B)).
#[derive(Default)]
struct Plugin {
    chosen: RefCell<Vec<f32>>,
    followed: RefCell<Vec<f32>>,
    paired: bool,
}

impl ZoomHost for Plugin {
    fn zoom_chosen(&self, zoom: f32) {
        self.chosen.borrow_mut().push(zoom);
    }
    fn zoom_followed(&self, zoom: f32) {
        self.followed.borrow_mut().push(zoom);
    }
    fn answers_paired(&self) -> bool {
        self.paired
    }
}

/// What the host does with a size the panel asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Answer {
    /// Gives the size asked.
    Exact,
    /// Gives a size this many pixels shorter (Cubase's view a pixel short;
    /// Klinga's B-PNL-12 test host by 100).
    Short(f32),
    /// This many pixels taller.
    Taller(f32),
    /// Refuses: the window is put back as it was (CLAP's `false`; baseview
    /// reverts the window).
    Refuse,
}

fn panel(ui: &mut Ui, plugin: &Plugin, base: Vec2) {
    let ctx = ui.ctx().clone();
    zoom::follow(&ctx, plugin, base, &RANGE);
    zoom::menu(&ctx, plugin, &RANGE, MENU_AT);
}

/// The panel's window in a host, as the module's note models it.
struct Hosted {
    /// egui's side: the screen, the clock, the display's scale.
    w: Window,
    plugin: Plugin,
    base: Vec2,
    /// The window's size in physical pixels.
    pixels: Vec2,
    scale: f32,
    /// The zoom egui-baseview last saw.
    seen: f32,
    answer: Answer,
    /// The host's answer lands one pass after the child took the size asked.
    late: bool,
    /// A late answer still to land.
    due: Option<Vec2>,
    /// Each size the window asked the host for, in pixels.
    requests: Vec<Vec2>,
    /// The window's size after each pass, in pixels.
    sizes: Vec<Vec2>,
    /// The zoom each pass was drawn at.
    drawn: Vec<f32>,
}

impl Hosted {
    /// A window opened at `zoom` on a display at `scale`, made at the
    /// panel's size; its first pass run, then the host's answer to the size
    /// baseview asks for at its show (patch 10's queued request), which
    /// lands before the second pass.
    fn open(base: Vec2, zoom: f32, scale: f32, answer: Answer, paired: bool) -> Self {
        let mut w = Window::new(base);
        w.display_scale = Some(scale);
        w.ctx.set_zoom_factor(zoom);
        let mut h = Hosted {
            w,
            plugin: Plugin { paired, ..Plugin::default() },
            base,
            pixels: (base * zoom * scale).round(),
            scale,
            seen: zoom,
            answer,
            late: false,
            due: None,
            requests: vec![],
            sizes: vec![],
            drawn: vec![],
        };
        h.pass(vec![]);
        let shown = h.pixels;
        h.pixels = h.answered(shown, shown);
        h
    }

    /// The host's answer to a request for `asked`, the window having been
    /// `old`. A plugin that pairs answers keeps them off the window (the
    /// design's (B)): only a refusal moves it, back, as baseview reverts it.
    fn answered(&self, asked: Vec2, old: Vec2) -> Vec2 {
        match self.answer {
            Answer::Refuse => old,
            _ if self.plugin.paired => asked,
            Answer::Exact => asked,
            Answer::Short(n) => asked - vec2(0.0, n),
            Answer::Taller(n) => asked + vec2(0.0, n),
        }
    }

    /// One pass with `events`, then the resize it asked for, if any.
    fn pass(&mut self, events: Vec<Event>) {
        self.w.screen = self.pixels / (self.scale * self.seen);
        let (plugin, base) = (&self.plugin, self.base);
        let commands = self.w.pass(events, Modifiers::NONE, &mut |ui| panel(ui, plugin, base)).viewport_output
            [&ViewportId::ROOT]
            .commands
            .clone();
        self.drawn.push(self.w.ctx.zoom_factor());
        if let Some(d) = self.due.take() {
            self.pixels = d;
        }
        let zoom = self.w.ctx.zoom_factor();
        let mut ask = None;
        if zoom != self.seen {
            ask = Some(self.pixels / self.seen * zoom);
            self.seen = zoom;
        }
        for c in &commands {
            if let ViewportCommand::InnerSize(s) = c {
                ask = Some(*s * zoom * self.scale);
            }
        }
        if let Some(a) = ask.map(Vec2::round).filter(|a| *a != self.pixels) {
            let old = self.pixels;
            self.requests.push(a);
            self.pixels = a;
            let answer = self.answered(a, old);
            if self.answer == Answer::Refuse && self.plugin.paired {
                // (B) sees CLAP's `false` inside the request and says so.
                zoom::refused(&self.w.ctx);
            }
            if self.late {
                self.due = Some(answer);
            } else {
                self.pixels = answer;
            }
        }
        self.sizes.push(self.pixels);
    }

    fn idle(&mut self, passes: usize) {
        for _ in 0..passes {
            self.pass(vec![]);
        }
    }

    /// The menu opened, two passes, and its row for `zoom` clicked: the
    /// pointer moved there, pressed and released, the release the pass that
    /// chooses.
    fn choose(&mut self, zoom: f32) {
        let k = RANGE.menu().iter().position(|&z| z == zoom).expect("a menu size");
        let at = zoom::menu_row(MENU_AT, k).center();
        zoom::open_menu(&self.w.ctx);
        // An egui area takes no clicks on its first pass, which only sizes
        // it; two, as the test kit's click gives a menu opened by one.
        self.idle(2);
        let press = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.pass(vec![Event::PointerMoved(at)]);
        self.pass(vec![press(true)]);
        self.pass(vec![press(false)]);
    }

    /// The panel's size at `zoom` in this window's pixels.
    fn panel_at(&self, zoom: f32) -> Vec2 {
        (self.base * zoom * self.scale).round()
    }

    fn chosen(&self) -> Vec<f32> {
        self.plugin.chosen.borrow().clone()
    }

    fn followed(&self) -> Vec<f32> {
        self.plugin.followed.borrow().clone()
    }

    fn zoom(&self) -> f32 {
        self.w.ctx.zoom_factor()
    }

    fn settled(&self) -> bool {
        let last = &self.sizes[self.sizes.len() - SETTLED_OVER..];
        last.iter().all(|s| *s == last[0])
    }
}

/// Within a pixel: the window's size is whole pixels, the panel's at a zoom
/// worked from a whole-pixel height is not (the design's note on its
/// finding 1: compare sizes in pixels, not the zoom exactly).
fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_elem() <= PIXEL
}

/// A pixel: [`near`]'s limit, the window's own rounding.
const PIXEL: f32 = 1.0;

// ---- The host's answer to the panel's own request ----

#[test]
fn zoom_drift_cure_a_host_answering_a_menu_size_short_or_taller_is_asked_once_and_the_zoom_stays() {
    for answer in [Answer::Short(1.0), Answer::Short(100.0), Answer::Taller(1.0)] {
        for scale in [1.0, SCALE_125] {
            let mut h = Hosted::open(BASE, 1.0, scale, Answer::Exact, false);
            h.idle(1);
            h.answer = answer;
            h.choose(1.25);
            h.idle(ROUNDS);
            let at = format!("{answer:?} at {scale}");
            assert_eq!(h.zoom(), 1.25, "{at}: the zoom drifted, sizes {:?}", h.sizes);
            assert_eq!(h.requests.len(), ONE_REQUEST, "{at}: {:?}", h.requests);
            assert_eq!(h.chosen(), [1.25], "{at}: the menu's choice told once");
            assert!(h.followed().is_empty(), "{at}: the answer is not the host's own size: {:?}", h.followed());
            assert!(h.settled(), "{at}: {:?}", h.sizes);
        }
    }
}

#[test]
fn zoom_drift_cure_the_answer_to_the_opening_size_is_not_a_new_zoom() {
    // Face 1: the host's view at the open a pixel short, or 100 short
    // (Klinga's B-PNL-12 host), or a pixel taller: the window takes it, the
    // zoom it opened at stays, nothing is told and nothing asked.
    for answer in [Answer::Short(1.0), Answer::Short(100.0), Answer::Taller(1.0)] {
        for zoom in [1.0, 1.25] {
            let mut h = Hosted::open(BASE, zoom, SCALE_125, answer, false);
            h.idle(ROUNDS);
            let at = format!("{answer:?} from {zoom}");
            assert_eq!(h.zoom(), zoom, "{at}: {:?}", h.sizes.last());
            assert!(h.requests.is_empty(), "{at}: never asked again: {:?}", h.requests);
            assert!(h.chosen().is_empty() && h.followed().is_empty(), "{at}: {:?} {:?}", h.chosen(), h.followed());
        }
    }
}

#[test]
fn zoom_drift_cure_roedalogs_90_percent_runaway_is_one_request() {
    // Rödalog at 90 % on Windows (the card's evidence: 99 requests, 1710 ×
    // 616 down to 1437 × 517): the corner dragged to 90 % after the open,
    // the zoom for that height not a whole-pixel width, so the panel asks
    // for its own width, and the host answers each request a pixel short.
    let mut h = Hosted::open(ROEDALOG, 1.0, SCALE_125, Answer::Exact, false);
    h.idle(1);
    h.answer = Answer::Short(1.0);
    h.pixels = vec2(1710.0, 616.0);
    h.idle(ROUNDS);
    // The drag's zoom, compared in pixels, as `ctx.content_rect()`, which it
    // is worked from, is rounded to egui's grid of 1/32 point (`round_ui`):
    // the panel at it within a pixel of the dragged window.
    assert!(near(h.panel_at(h.zoom()), vec2(1710.0, 616.0)), "the drag's zoom kept: {} {:?}", h.zoom(), h.sizes);
    assert_eq!(h.followed(), [h.zoom()], "the drag followed once");
    assert!(h.chosen().is_empty());
    assert_eq!(h.requests.len(), ONE_REQUEST, "{:?}", h.requests);
    assert!(h.settled(), "{:?}", h.sizes);
}

#[test]
fn zoom_drift_cure_a_menu_choice_is_told_once_the_window_has_taken_it() {
    let mut h = Hosted::open(BASE, 1.0, SCALE_125, Answer::Exact, false);
    h.idle(1);
    h.choose(1.5);
    assert!(h.chosen().is_empty(), "not on the click's pass");
    h.idle(1);
    assert!(h.chosen().is_empty(), "not on the pass the zoom took effect, when the window is asked for its size");
    h.idle(1);
    assert_eq!(h.chosen(), [1.5], "on the pass after the window took it");
    h.idle(ROUNDS);
    assert_eq!(h.chosen(), [1.5], "once");
    assert_eq!(h.pixels, h.panel_at(1.5));
    assert_eq!(h.requests, [h.panel_at(1.5)]);
}

#[test]
fn zoom_drift_cure_a_menu_choice_the_window_already_has_is_told_at_once() {
    // The host's view at the open the size of 150 % (taken, not followed):
    // choosing 150 % then needs no request, so nothing is awaited.
    let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Exact, false);
    h.pixels = h.panel_at(1.5);
    h.idle(2);
    assert_eq!(h.zoom(), 1.0);
    h.choose(1.5);
    h.idle(1);
    assert_eq!(h.chosen(), [1.5], "on the pass the zoom took effect");
    h.idle(ROUNDS);
    assert_eq!(h.chosen(), [1.5]);
    assert!(h.requests.is_empty(), "{:?}", h.requests);
}

#[test]
fn zoom_drift_cure_the_menus_current_size_is_told_at_once() {
    let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Exact, false);
    h.idle(1);
    h.choose(1.0);
    assert_eq!(h.chosen(), [1.0], "on the click's pass: nothing for the window to take");
    h.idle(ROUNDS);
    assert_eq!(h.chosen(), [1.0]);
    assert!(h.requests.is_empty());
}

// ---- Corner drags (the fact-check's finding 1) ----

#[test]
fn zoom_drift_cure_a_two_step_corner_drag_to_175_percent_loses_no_step() {
    // The second step seen on the pass after the one the first step's zoom
    // took effect on (gap 2: the pass the first prototype read as an
    // answer), or on the pass after that (gap 3), as the fact-check's probe.
    // Not gap 1, the step landing before the zoom takes effect: egui shows
    // that pass the last pass's window, so the panel asks for the first
    // step's size over it, as v0.5.0 did, and awaits no answer, so a host
    // answering a pixel short gives one drift step, followed and saved.
    // That is cured only in the window part (Ian, 2026-11-26,
    // ZOOM-DRAG-STEP, point 1 (a); `follow`'s note): `panelwerk-nice`'s
    // `tests/pairing.rs` has gap 1.
    for gap in [2, 3] {
        for scale in [1.0, SCALE_125] {
            let mut h = Hosted::open(BASE, 1.0, scale, Answer::Exact, false);
            h.idle(1);
            h.pixels = h.panel_at(1.5);
            h.idle(gap);
            h.pixels = h.panel_at(1.75);
            h.idle(8);
            let at = format!("gap {gap} at {scale}");
            assert!(
                near(h.pixels, h.panel_at(h.zoom())),
                "{at}: the window {:?}, the panel {:?}",
                h.pixels,
                h.panel_at(h.zoom())
            );
            assert!(
                near(h.pixels, h.panel_at(1.75)),
                "{at}: the second step lost: zoom {}, window {:?}",
                h.zoom(),
                h.pixels
            );
            assert_eq!(h.followed().len(), 2, "{at}: both steps followed: {:?}", h.followed());
            assert!(h.chosen().is_empty(), "{at}");
        }
    }
}

// ---- The wait for an answer, a quarter second (ANSWER_WITHIN) ----

#[test]
fn zoom_drift_cure_a_size_seen_within_the_wait_is_the_answer_and_after_it_the_hosts_own() {
    for (gap, own) in [(ANSWER_WITHIN / 2.0, false), (ANSWER_WITHIN * 2.0, true)] {
        let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Exact, false);
        h.idle(1);
        h.choose(1.25);
        // The pass the zoom takes effect on asks for its size, at its time.
        h.idle(1);
        assert_eq!(h.requests.len(), 1);
        // The next frame comes `gap` after that pass (frames come on demand,
        // so it may be seconds), the window then dragged to 150 %: the clock
        // moved on by the gap less the step the next pass adds itself.
        assert!(FRAME < gap, "a gap of {gap} s can't be made with passes {FRAME} s apart");
        h.w.time += gap - FRAME;
        h.pixels = h.panel_at(1.5);
        h.idle(8);
        let at = format!("{gap} s after the request");
        assert_eq!(h.chosen(), [1.25], "{at}");
        if own {
            assert_eq!((h.zoom(), h.followed()), (1.5, vec![1.5]), "{at}: a drag after the wait is followed");
        } else {
            assert_eq!((h.zoom(), h.followed()), (1.25, vec![]), "{at}: within the wait it is the answer");
        }
    }
}

// ---- A plugin that pairs requests and answers (the design's (B)) ----

#[test]
fn zoom_drift_cure_with_answers_paired_a_host_size_on_the_next_pass_is_followed() {
    // (B) keeps the answers off the window, so whatever reaches it is the
    // host's own, even on the pass after a request: the opening's or a menu
    // choice's.
    let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Short(1.0), true);
    h.pixels = h.panel_at(1.5);
    h.idle(4);
    assert_eq!((h.zoom(), h.followed()), (1.5, vec![1.5]), "on the opening's answer pass");
    h.choose(2.0);
    h.idle(1);
    h.pixels = h.panel_at(1.75);
    h.idle(4);
    assert_eq!(h.chosen(), [2.0]);
    assert_eq!((h.zoom(), h.followed()), (1.75, vec![1.5, 1.75]), "on the menu's answer pass");
}

/// A check of the model's window part, not of `follow`: with (B) keeping
/// the late answer off the window, the window only ever has the size asked,
/// so v0.5.0's `follow` passes it too. The pairing this model assumes is
/// checked against the window part itself in `panelwerk-nice`'s
/// `tests/pairing.rs` (W-D3), a late answer included.
#[test]
fn zoom_drift_cure_with_answers_paired_a_late_answer_kept_off_the_window_asks_once() {
    let mut h = Hosted::open(BASE, 1.0, SCALE_125, Answer::Short(1.0), true);
    h.late = true;
    h.idle(1);
    h.choose(1.5);
    h.idle(ROUNDS);
    assert_eq!((h.zoom(), h.chosen()), (1.5, vec![1.5]));
    assert_eq!(h.requests.len(), ONE_REQUEST, "{:?}", h.requests);
    assert_eq!(h.pixels, h.panel_at(1.5), "the panel's size kept");
}

// ---- A CLAP host refusing a menu size (point 4 (a)) ----

#[test]
fn zoom_drift_cure_a_refused_menu_size_goes_back_and_nothing_is_told() {
    for paired in [false, true] {
        let mut h = Hosted::open(BASE, 1.0, SCALE_125, Answer::Refuse, paired);
        h.idle(1);
        let old = h.pixels;
        h.choose(1.5);
        let click = h.drawn.len();
        h.idle(ROUNDS);
        // Drawn at the refused zoom on the pass it took effect; then, with
        // (B) reporting the refusal inside the request, the very next pass
        // at the old zoom; without (B), one pass later.
        let refused_on = if paired { 1 } else { 2 };
        let want: Vec<f32> = (0..4).map(|k| if k < refused_on { 1.5 } else { 1.0 }).collect();
        assert_eq!(h.drawn[click..click + 4], want[..], "paired {paired}");
        assert!(
            h.chosen().is_empty() && h.followed().is_empty(),
            "paired {paired}: {:?} {:?}",
            h.chosen(),
            h.followed()
        );
        assert_eq!(h.pixels, old, "paired {paired}");
        assert_eq!(h.requests.len(), ONE_REQUEST, "paired {paired}: {:?}", h.requests);
    }
}

#[test]
fn zoom_drift_cure_a_refusal_reported_with_no_menu_choice_changes_no_zoom() {
    // (B) reports a CLAP `false` with no menu choice waiting (here nothing
    // waits at all; in a host, the panel asking for its own width after a
    // drag off the aspect ratio): there is no choice to undo.
    // Opened at 125 %, so a `refused` that put the zoom back to 100 % shows.
    let mut h = Hosted::open(BASE, 1.25, 1.0, Answer::Exact, true);
    h.idle(1);
    zoom::refused(&h.w.ctx);
    h.idle(4);
    assert_eq!(h.zoom(), 1.25);
    assert!(h.chosen().is_empty() && h.followed().is_empty());
}

#[test]
fn zoom_drift_cure_with_answers_paired_a_refused_request_to_go_back_to_the_shape_is_asked_once() {
    // The corner dragged 100 px wider, off the aspect ratio: the panel asks
    // for its own shape, and a CLAP host refuses (baseview puts the window
    // back; (B) reports it). The window is then as it was before the
    // request, which the panel had already seen: nothing new to follow, so
    // it asks no more, as without (B), where the next pass is the answer.
    for paired in [false, true] {
        let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Exact, paired);
        h.idle(1);
        h.answer = Answer::Refuse;
        let dragged = h.panel_at(1.0) + vec2(100.0, 0.0);
        h.pixels = dragged;
        h.idle(ROUNDS);
        assert_eq!(h.requests.len(), ONE_REQUEST, "paired {paired}: {:?}", h.requests);
        assert_eq!((h.zoom(), h.pixels), (1.0, dragged), "paired {paired}");
        assert!(h.chosen().is_empty() && h.followed().is_empty(), "paired {paired}");
    }
}

// ---- The plugin's own choice while the menu's waits ----

#[test]
fn zoom_drift_cure_choose_while_a_menu_choice_waits_replaces_it() {
    // `zoom::choose` (the plugin's own zoom) on the pass after a menu click,
    // before its zoom has taken effect (0), or after its size was asked for
    // and before it is judged (1): the plugin's choice replaces the menu's,
    // which is neither told nor undone.
    for after in [0, 1] {
        let mut h = Hosted::open(BASE, 1.0, 1.0, Answer::Exact, false);
        h.idle(1);
        h.choose(1.5);
        h.idle(after);
        let from = h.drawn.len();
        zoom::choose(&h.w.ctx, &h.plugin, &RANGE, 2.0);
        h.idle(ROUNDS);
        assert_eq!(h.chosen(), [2.0], "after {after}: the plugin's zoom told, once, the menu's not");
        assert_eq!((h.zoom(), h.pixels), (2.0, h.panel_at(2.0)), "after {after}: {:?}", h.sizes);
        assert!(h.drawn[from..].iter().all(|z| *z == 2.0), "after {after}: drawn {:?}", &h.drawn[from..]);
        assert!(h.followed().is_empty(), "after {after}: {:?}", h.followed());
    }
}

// ---- The display's scale ----

#[test]
fn zoom_drift_cure_a_drag_on_a_300_percent_display_a_pixel_off_the_shape_asks_once() {
    // On a 300 % display the corner dragged to 150 %'s height and a pixel
    // wider than its width: in pixels the window must change size, so the
    // panel asks for its width and awaits the answer, which, 100 pixels
    // short (Klinga's B-PNL-12 host), is kept. Sizes worked in points
    // instead (the display's scale left out) would call the window already
    // the zoom's size, await nothing, and follow the answer as a new zoom.
    let mut h = Hosted::open(BASE, 1.0, SCALE_300, Answer::Exact, false);
    h.idle(1);
    h.answer = Answer::Short(100.0);
    let dragged = h.panel_at(1.5) + vec2(1.0, 0.0);
    h.pixels = dragged;
    h.idle(ROUNDS);
    assert_eq!(h.followed(), [1.5], "the drag followed once: sizes {:?}", h.sizes);
    assert_eq!(h.zoom(), 1.5);
    assert_eq!(h.requests.len(), ONE_REQUEST, "{:?}", h.requests);
}
