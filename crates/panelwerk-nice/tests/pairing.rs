//! W-D3, the zoom cure's window part (the design's (B),
//! `docs/proposals/zoom-drift-cure.md`, with its amendment of 2026-11-26,
//! card ZOOM-DRAG-STEP; the rule in `docs/tasks-v0.6.md`): [`Pairing`]
//! pairs each of the panel's resize requests with the host's answer and
//! keeps the answer off the window, reports a host's refusal through
//! `zoom::refused`, says whether the main button was held at the last size
//! the host gave the window, takes every size given while the button is
//! held for a drag, and keeps a drag step that lands before the previous
//! step's zoom has drawn from being overwritten.
//!
//! The window is modelled natively as baseview and egui-baseview run it on
//! Windows (the design's §1; `zoom_answer.rs` in the panel's tests models
//! the same without the window part): egui is given the window's pixels
//! over the display's scale and the zoom egui-baseview last saw; after a
//! pass, a changed zoom or an `InnerSize` command resizes the window, and
//! only a size that differs from the window's reaches the host, through the
//! callbacks [`Pairing::callbacks`] wrapped. The child takes the size at
//! once; if the request fails, baseview puts the window back as it was
//! (`WM_SIZE`'s revert). The host gives sizes through the handle
//! [`Pairing::around`] wrapped, whose inner handle sets the stand-in
//! window's size, as baseview's `Window::resize` does (host-originated: no
//! request back). What this model can't show, the real window, the host's
//! real timing and the physical button, waits for the consumers' Windows
//! runs (`docs/tasks-v0.6.md`, W-D3's note).
//!
//! No test sleeps: [`Pairing`]'s clock is the test's ([`Clock`]), moved on
//! with egui's input time.

use std::cell::{Cell, OnceCell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use nice_plug::editor::dpi::{NativeSize, PhysicalSize, Size};
use nice_plug::editor::{
    EditorHandle, HostCallbacks, HostMainThreadCaller, HostMethods, Modifiers as KeyModifiers, ParentWindowHandle,
    VirtualKeyCode,
};
use panelwerk_nice::{Paired, Pairing};
use panelwerk_panel::egui::{vec2, Event, Modifiers, PointerButton, Pos2, Vec2, ViewportCommand, ViewportId};
use panelwerk_panel::zoom::{self, Range, ZoomHost, ANSWER_WITHIN};
use panelwerk_testkit::Window;

/// A range as Kör's and Klinga's, 75 … 300 %, so that the review's probe
/// past the 300 % maximum has its maximum; a test range, not a consumer's.
const RANGE: Range = Range::new(0.75, 3.0, &[0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0]);
/// Kör's panel in points, the size the panel's zoom tests use.
const BASE: Vec2 = vec2(520.0, 280.0);
/// Ian's display scaling, 125 % (Rödalog's and Klinga's Windows runs).
const SCALE_125: f32 = 1.25;
/// Where the menu's first row is drawn.
const MENU_AT: Pos2 = Pos2::new(12.0, 32.0);
/// How far the clock moves each pass: the test kit's window moves egui's
/// input time a tenth of a second a pass, and the window part's clock is
/// moved with it.
const FRAME: f64 = 0.1;
/// Passes to run for a drift or a resize every pass to show: well past the
/// two passes a request and its answer take (as `zoom_answer.rs`'s ROUNDS:
/// v0.5.0 made 20 requests in 40).
const ROUNDS: usize = 40;
/// The design's cure: at most one request per zoom (the design's §4 (iii),
/// "face 2: at most one request per zoom").
const ONE_REQUEST: usize = 1;

/// The test's clock for [`Pairing`]: a time in nanoseconds the test moves.
#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);

impl Clock {
    fn add(&self, seconds: f64) {
        self.0.fetch_add(Duration::from_secs_f64(seconds).as_nanos() as u64, Ordering::SeqCst);
    }
    fn reading(&self) -> Arc<dyn Fn() -> Duration + Send + Sync> {
        let at = self.0.clone();
        Arc::new(move || Duration::from_nanos(at.load(Ordering::SeqCst)))
    }
}

/// The stand-in window: its size in physical pixels, set by its handle as
/// baseview's `Window::resize` sets a real one.
#[derive(Default)]
struct Win {
    pixels: Cell<Vec2>,
}

fn measure(win: &Win) -> NativeSize<u32> {
    let p = win.pixels.get();
    NativeSize::new(p.x as u32, p.y as u32)
}

/// The window's own handle: a host's size is the window's (nice-plug-egui's
/// `set_size`, baseview's `Window::resize`); every other call recorded.
#[derive(Default)]
struct Child {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Child {
    fn log(&self, call: String) {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(call);
    }
}

/// Set by [`Child::run_until_closed`], which has no instance to record in.
static RAN_UNTIL_CLOSED: AtomicBool = AtomicBool::new(false);

impl EditorHandle for Child {
    type Window = Win;
    type Error = std::fmt::Error;

    fn run_until_closed(_window: Win) -> Result<(), std::fmt::Error> {
        RAN_UNTIL_CLOSED.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn set_parent(&self, _parent: ParentWindowHandle, _window: &Win) -> Result<(), std::fmt::Error> {
        self.log("set_parent".into());
        Ok(())
    }
    fn show(&self, _window: &Win) -> Result<(), std::fmt::Error> {
        self.log("show".into());
        Ok(())
    }
    fn hide(&self, _window: &Win) -> Result<(), std::fmt::Error> {
        self.log("hide".into());
        Err(std::fmt::Error)
    }
    fn set_size(&self, new_size: NativeSize<u32>, window: &Win) -> Result<(), std::fmt::Error> {
        window.pixels.set(vec2(new_size.width as f32, new_size.height as f32));
        Ok(())
    }
    fn host_main_thread_callback(&self, _window: &Win) {
        self.log("host_main_thread_callback".into());
    }
    fn adjust_size(&self, new_size: NativeSize<u32>, _window: &Win) -> Option<NativeSize<u32>> {
        self.log(format!("adjust_size {}x{}", new_size.width, new_size.height));
        Some(NativeSize::new(7, 8))
    }
    fn set_fallback_scale_factor(&self, scale_factor: f64, _window: &Win) -> Result<(), std::fmt::Error> {
        self.log(format!("set_fallback_scale_factor {scale_factor}"));
        Err(std::fmt::Error)
    }
    fn on_virtual_key_from_host(&self, _key_code: VirtualKeyCode, is_down: bool, _modifiers: KeyModifiers) -> bool {
        self.log(format!("on_virtual_key_from_host {is_down}"));
        is_down
    }
    fn state_changed(&self) {
        self.log("state_changed".into());
    }
    fn param_value_changed(&self, id: &str, normalized_value: f32) {
        self.log(format!("param_value_changed {id} {normalized_value}"));
    }
    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {
        self.log(format!("param_modulation_changed {id} {modulation_offset}"));
    }
}

/// How the host answers a request.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Answer {
    /// The size asked.
    Exact,
    /// This many pixels shorter (Cubase's view a pixel short; Klinga's
    /// B-PNL-12 test host by 100).
    Short(f32),
    /// Refuses (CLAP's `false`; baseview then puts the window back).
    Refuse,
    /// Gives no answer at all.
    None,
}

/// When the host's answer comes, as the design's §1 has each format.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Format {
    /// Inside the request (CLAP's `request_resize` calling `gui.set_size`).
    Clap,
    /// After the request returns (VST3: nice-plug posts the request,
    /// patch 10, and the host's `onSize` comes inside `resizeView`), before
    /// the next pass.
    Vst3,
    /// After the next pass, still within [`ANSWER_WITHIN`] (a pass is
    /// [`FRAME`]).
    Late,
}

/// What the host and the window share in the model.
struct World {
    win: Win,
    /// The host's view: the size it last gave the window, its own or an
    /// answer (the design's (B): the window may keep the panel's size
    /// inside a view of another).
    view: Cell<Vec2>,
    handle: OnceCell<Paired<Child>>,
    answer: Cell<Answer>,
    format: Cell<Format>,
    /// Each size the window asked the host for, in pixels.
    requests: RefCell<Vec<Vec2>>,
    /// Answers still to come (VST3's, a late one's), each with the passes
    /// still to run before it lands.
    due: RefCell<Vec<(usize, Vec2)>>,
}

impl World {
    fn give(&self, size: Vec2) {
        self.view.set(size);
        let handle = self.handle.get().expect("the handle made");
        handle.set_size(NativeSize::new(size.x as u32, size.y as u32), &self.win).expect("the stand-in sets it");
    }
}

/// The host's side of the window's requests.
struct Host(Rc<World>);

impl HostCallbacks for Host {
    fn request_resize(&mut self, new_size: Size, scale_factor: f64) -> Result<(), Box<dyn Error>> {
        let asked: PhysicalSize<u32> = new_size.to_physical(scale_factor);
        let asked = vec2(asked.width as f32, asked.height as f32);
        self.0.requests.borrow_mut().push(asked);
        let given = match self.0.answer.get() {
            Answer::Refuse => return Err("refused".into()),
            Answer::None => return Ok(()),
            Answer::Exact => asked,
            Answer::Short(n) => asked - vec2(0.0, n),
        };
        match self.0.format.get() {
            Format::Clap => self.0.give(given),
            Format::Vst3 => self.0.due.borrow_mut().push((0, given)),
            Format::Late => self.0.due.borrow_mut().push((1, given)),
        }
        Ok(())
    }

    fn destroyed(&mut self) {
        self.0.requests.borrow_mut().push(vec2(-1.0, -1.0));
    }
}

/// The main-thread caller nice-plug gives (only X11 needs one).
struct Caller(Arc<AtomicBool>);

impl HostMainThreadCaller for Caller {
    fn call_main_thread(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// The plugin's side: every zoom told it, and whether the button was held
/// at each size followed, as a plugin saving only drags reads it.
struct Plugin {
    pairing: Pairing,
    chosen: RefCell<Vec<f32>>,
    followed: RefCell<Vec<(f32, bool)>>,
}

impl ZoomHost for Plugin {
    fn zoom_chosen(&self, zoom: f32) {
        self.chosen.borrow_mut().push(zoom);
    }
    fn zoom_followed(&self, zoom: f32) {
        self.followed.borrow_mut().push((zoom, self.pairing.dragged()));
    }
    fn answers_paired(&self) -> bool {
        true
    }
}

/// The panel's window with the window part, in a host.
struct Hosted {
    w: Window,
    world: Rc<World>,
    callbacks: Box<dyn HostCallbacks>,
    plugin: Plugin,
    clock: Clock,
    button: Arc<AtomicBool>,
    scale: f32,
    /// The zoom egui-baseview last saw.
    seen: f32,
    /// The window's size after each pass, in pixels.
    sizes: Vec<Vec2>,
    /// The zoom each pass was drawn at.
    drawn: Vec<f32>,
}

impl Hosted {
    /// A window opened at `zoom` on a display at `scale`, the host
    /// answering as `answer` in `format`; its first pass run, then its
    /// opening's answer (to the size baseview asks for at its show), which
    /// the window part keeps off the window.
    fn open(zoom: f32, scale: f32, answer: Answer, format: Format) -> Self {
        let clock = Clock::default();
        let button = Arc::new(AtomicBool::new(false));
        let held = button.clone();
        let pairing = Pairing::with_clock(Arc::new(move || held.load(Ordering::SeqCst)), clock.reading());
        let world = Rc::new(World {
            win: Win::default(),
            view: Cell::new(Vec2::ZERO),
            handle: OnceCell::new(),
            answer: Cell::new(answer),
            format: Cell::new(format),
            requests: RefCell::default(),
            due: RefCell::default(),
        });
        world.win.pixels.set((BASE * zoom * scale).round());
        world.view.set(world.win.pixels.get());
        let _ = world.handle.set(pairing.around(Child::default(), measure));
        let methods = pairing.callbacks(HostMethods {
            callbacks: Box::new(Host(world.clone())),
            main_thread_caller: Box::new(Caller(Arc::default())),
        });
        let mut w = Window::new(BASE);
        w.display_scale = Some(scale);
        w.ctx.set_zoom_factor(zoom);
        let plugin = Plugin { pairing, chosen: RefCell::default(), followed: RefCell::default() };
        let mut h = Hosted {
            w,
            world,
            callbacks: methods.callbacks,
            plugin,
            clock,
            button,
            scale,
            seen: zoom,
            sizes: vec![],
            drawn: vec![],
        };
        // Baseview's request at its show, for the size the window has.
        let shown = h.pixels();
        h.request(shown);
        h.land();
        h.pass(vec![]);
        h
    }

    fn pixels(&self) -> Vec2 {
        self.world.win.pixels.get()
    }

    fn view(&self) -> Vec2 {
        self.world.view.get()
    }

    /// The window asks the host for `size`: the child takes it, and if the
    /// request fails baseview puts it back.
    fn request(&mut self, size: Vec2) {
        let old = self.pixels();
        self.world.win.pixels.set(size);
        let asked = Size::Physical(PhysicalSize::new(size.x as u32, size.y as u32));
        if self.callbacks.request_resize(asked, f64::from(self.scale)).is_err() {
            self.world.win.pixels.set(old);
        }
    }

    /// Between passes: the host's answers with no pass left to wait land,
    /// the others wait one pass less.
    fn land(&mut self) {
        let due: Vec<(usize, Vec2)> = self.world.due.borrow_mut().drain(..).collect();
        for (wait, size) in due {
            match wait {
                0 => self.world.give(size),
                _ => self.world.due.borrow_mut().push((wait - 1, size)),
            }
        }
    }

    /// One pass with `events`, then the resize it asked for, if any, and
    /// the host's answers due.
    fn pass(&mut self, events: Vec<Event>) {
        self.clock.add(FRAME);
        self.w.screen = self.pixels() / (self.scale * self.seen);
        let plugin = &self.plugin;
        let run = &mut |ui: &mut panelwerk_panel::egui::Ui| {
            let ctx = ui.ctx().clone();
            plugin.pairing.pass(&ctx);
            zoom::follow(&ctx, plugin, BASE, &RANGE);
            zoom::menu(&ctx, plugin, &RANGE, MENU_AT);
        };
        let commands = self.w.pass(events, Modifiers::NONE, run).viewport_output[&ViewportId::ROOT].commands.clone();
        self.drawn.push(self.w.ctx.zoom_factor());
        let zoom = self.w.ctx.zoom_factor();
        let mut ask = None;
        if zoom != self.seen {
            ask = Some(self.pixels() / self.seen * zoom);
            self.seen = zoom;
        }
        for c in &commands {
            if let ViewportCommand::InnerSize(s) = c {
                ask = Some(*s * zoom * self.scale);
            }
        }
        if let Some(a) = ask.map(Vec2::round).filter(|a| *a != self.pixels()) {
            self.request(a);
        }
        self.land();
        self.sizes.push(self.pixels());
    }

    fn idle(&mut self, passes: usize) {
        for _ in 0..passes {
            self.pass(vec![]);
        }
    }

    /// The host gives the window `size` on its own (a drag step, its
    /// layout), between passes.
    fn give(&self, size: Vec2) {
        self.world.give(size);
    }

    /// The menu opened and its row for `zoom` clicked (as `zoom_answer.rs`).
    fn choose(&mut self, zoom: f32) {
        let k = RANGE.menu().iter().position(|&z| z == zoom).expect("a menu size");
        let at = zoom::menu_row(MENU_AT, k).center();
        zoom::open_menu(&self.w.ctx);
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

    fn hold(&self, held: bool) {
        self.button.store(held, Ordering::SeqCst);
    }

    /// The panel's size at `zoom` in this window's pixels.
    fn panel_at(&self, zoom: f32) -> Vec2 {
        (BASE * zoom * self.scale).round()
    }

    fn zoom(&self) -> f32 {
        self.w.ctx.zoom_factor()
    }

    fn requests(&self) -> Vec<Vec2> {
        self.world.requests.borrow().clone()
    }

    fn chosen(&self) -> Vec<f32> {
        self.plugin.chosen.borrow().clone()
    }

    fn followed(&self) -> Vec<(f32, bool)> {
        self.plugin.followed.borrow().clone()
    }

    /// Whether the window and the panel drawn in it are the same size.
    fn fits(&self) -> bool {
        near(self.pixels(), self.panel_at(self.zoom()))
    }
}

/// Within a pixel: the window's size is whole pixels, the panel's at a zoom
/// worked from a whole-pixel height is not (the design's note on its
/// finding 1: compare sizes in pixels).
fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_elem() <= PIXEL
}

/// A pixel: [`near`]'s limit, the window's own rounding.
const PIXEL: f32 = 1.0;

const FORMATS: [Format; 3] = [Format::Clap, Format::Vst3, Format::Late];

// ---- The host's answer kept off the window (the design's (B)) ----

#[test]
fn w_d3_an_answer_to_a_menu_size_is_kept_off_the_window_and_asked_once() {
    for format in FORMATS {
        for answer in [Answer::Short(1.0), Answer::Short(100.0)] {
            let mut h = Hosted::open(1.0, SCALE_125, Answer::Exact, format);
            h.idle(1);
            let opened = h.requests().len();
            h.world.answer.set(answer);
            h.choose(1.5);
            h.idle(ROUNDS);
            let at = format!("{answer:?} {format:?}");
            assert_eq!(h.zoom(), 1.5, "{at}: {:?}", h.sizes);
            assert_eq!(h.pixels(), h.panel_at(1.5), "{at}: the window keeps the panel's size");
            assert_eq!(h.requests().len() - opened, ONE_REQUEST, "{at}: {:?}", h.requests());
            assert_eq!(h.chosen(), [1.5], "{at}: told once");
            assert!(h.followed().is_empty(), "{at}: the answer is not a size of the host's own: {:?}", h.followed());
        }
    }
}

#[test]
fn w_d3_the_answer_to_the_opening_size_is_kept_off_the_window() {
    for format in [Format::Clap, Format::Vst3] {
        let mut h = Hosted::open(1.25, SCALE_125, Answer::Short(100.0), format);
        h.idle(ROUNDS);
        assert_eq!((h.zoom(), h.pixels()), (1.25, h.panel_at(1.25)), "{format:?}");
        assert_eq!(h.requests().len(), 1, "{format:?}: the show's request only");
        assert!(h.chosen().is_empty() && h.followed().is_empty(), "{format:?}");
    }
}

#[test]
fn w_d3_a_host_size_after_the_wait_is_the_hosts_own_and_followed() {
    // A request the host never answers: a size of its own given within
    // ANSWER_WITHIN is taken for the answer (the design's cost, §6), one
    // given after it is followed.
    for (gap, own) in [(ANSWER_WITHIN / 2.0, false), (ANSWER_WITHIN * 2.0, true)] {
        let mut h = Hosted::open(1.0, 1.0, Answer::Exact, Format::Clap);
        h.idle(1);
        h.world.answer.set(Answer::None);
        h.choose(1.25);
        h.idle(1);
        assert_eq!(h.chosen(), [] as [f32; 0]);
        h.clock.add(gap);
        h.give(h.panel_at(1.5));
        h.idle(8);
        let at = format!("{gap} s after the request");
        assert_eq!(h.chosen(), [1.25], "{at}");
        if own {
            assert_eq!(h.followed(), [(1.5, false)], "{at}");
            assert!(h.fits(), "{at}");
        } else {
            assert_eq!((h.zoom(), h.followed()), (1.25, vec![]), "{at}: the answer, kept off");
            assert_eq!(h.pixels(), h.panel_at(1.25), "{at}");
        }
    }
}

// ---- A refusal (CLAP's `false`; ZOOM-DRIFT-CURE point 4 (a)) ----

#[test]
fn w_d3_a_refused_menu_size_is_undone_by_the_very_next_frame_and_never_told() {
    let mut h = Hosted::open(1.0, SCALE_125, Answer::Exact, Format::Clap);
    h.idle(1);
    let old = h.pixels();
    h.world.answer.set(Answer::Refuse);
    h.choose(1.5);
    let click = h.drawn.len();
    h.idle(ROUNDS);
    // Drawn at 150 % on the pass it took effect, whose request is refused;
    // the very next pass at 100 % (the window part called `zoom::refused`).
    assert_eq!(h.drawn[click..click + 3], [1.5, 1.0, 1.0]);
    assert!(h.chosen().is_empty() && h.followed().is_empty(), "{:?} {:?}", h.chosen(), h.followed());
    assert_eq!(h.pixels(), old);
}

#[test]
fn w_d3_a_refusal_before_the_first_pass_changes_nothing() {
    // The show's request refused, before any pass gave the window part the
    // panel's context: nothing to undo, and the window opens as it was.
    let mut h = Hosted::open(1.25, 1.0, Answer::Refuse, Format::Clap);
    h.idle(4);
    assert_eq!((h.zoom(), h.pixels()), (1.25, h.panel_at(1.25)));
    assert!(h.chosen().is_empty() && h.followed().is_empty());
}

// ---- The main button at each host size (ZOOM-DRIFT-CURE point 1 (a)) ----

#[test]
fn w_d3_dragged_says_whether_the_main_button_was_held_at_the_last_host_size() {
    let mut h = Hosted::open(1.0, 1.0, Answer::Exact, Format::Clap);
    h.idle(1);
    assert!(!h.plugin.pairing.dragged(), "no host size yet");
    h.hold(true);
    h.give(h.panel_at(1.5));
    h.hold(false);
    h.idle(4);
    assert_eq!(h.followed(), [(1.5, true)], "a drag");
    // The host giving the window the size it has changes nothing: the last
    // size given is still the drag's.
    h.give(h.pixels());
    assert!(h.plugin.pairing.dragged());
    h.give(h.panel_at(2.0));
    h.idle(4);
    let followed = h.followed();
    assert_eq!(followed.len(), 2, "{followed:?}");
    // The zoom worked from the window's height, compared in pixels (the
    // design's note on its finding 1).
    assert!(near(h.panel_at(followed[1].0), h.panel_at(2.0)) && !followed[1].1, "the host's own layout: {followed:?}");
}

// ---- ZOOM-DRAG-STEP point 1 (a): a step before the last step's zoom has drawn ----

#[test]
fn zoom_drag_step_1a_a_step_landing_before_the_previous_steps_zoom_has_drawn_is_not_overwritten() {
    // W-D1's probe, gap 1: at 125 %, a drag to 150 % then 175 %, the
    // second step landing after the pass that followed the first and
    // before the pass its zoom takes effect on, whose view is the first
    // step's window. Without the window part's cure the panel asks for the
    // first step's size, 975 × 525, over the second, and a host answering
    // a pixel short has its answer followed (1.497154).
    for held in [true, false] {
        for format in FORMATS {
            let mut h = Hosted::open(1.0, SCALE_125, Answer::Short(1.0), format);
            h.idle(1);
            let opened = h.requests().len();
            h.hold(held);
            h.give(h.panel_at(1.5));
            h.idle(1);
            h.give(h.panel_at(1.75));
            h.idle(1);
            h.hold(false);
            h.idle(8);
            let at = format!("held {held}, {format:?}");
            let stale = h.panel_at(1.5);
            assert!(!h.requests()[opened..].contains(&stale), "{at}: asked for {stale:?}: {:?}", h.requests());
            assert!(near(h.pixels(), h.panel_at(1.75)), "{at}: the second step overwritten: {:?}", h.pixels());
            assert!(h.fits(), "{at}: window {:?}, zoom {}", h.pixels(), h.zoom());
            assert_eq!(h.view(), h.pixels(), "{at}: the host's view is the window's");
            let zooms: Vec<f32> = h.followed().iter().map(|(z, _)| *z).collect();
            assert_eq!(zooms.len(), 2, "{at}: both steps followed, nothing more: {zooms:?}");
            assert!(near(h.panel_at(zooms[1]), h.panel_at(1.75)), "{at}: {zooms:?}");
            assert!(h.followed().iter().all(|(_, d)| *d == held), "{at}: {:?}", h.followed());
            assert!(h.chosen().is_empty(), "{at}");
        }
    }
}

#[test]
fn zoom_drag_step_1a_a_menu_choice_whose_request_is_stale_is_told_and_the_hosts_size_followed() {
    // The host gives a size of its own (its layout) between the menu
    // click's pass and the pass the chosen zoom takes effect on, whose view
    // is the window before it: the request for the menu's size is stale,
    // never sent, and the window keeps the host's size. The choice is told
    // on the next pass (the window is not back at its old size), and the
    // host's size followed; not undone, which would ask for the old zoom's
    // size from a view older still.
    let mut h = Hosted::open(1.0, 1.0, Answer::Exact, Format::Clap);
    h.idle(1);
    let opened = h.requests().len();
    h.choose(1.5);
    h.give(h.panel_at(2.0));
    h.idle(6);
    assert_eq!(h.requests().len(), opened, "nothing sent: {:?}", h.requests());
    assert_eq!(h.chosen(), [1.5]);
    let followed = h.followed();
    assert_eq!(followed.len(), 1, "{followed:?}");
    assert!(near(h.panel_at(followed[0].0), h.panel_at(2.0)), "{followed:?}");
    assert!(h.fits() && h.view() == h.pixels(), "window {:?}, zoom {}", h.pixels(), h.zoom());
}

// ---- ZOOM-DRAG-STEP point 2 (a): with the button held, never an answer ----

/// Opened at 250 % on a display at 100 %, the corner dragged past the
/// 300 % maximum, to 330 % (1716 × 924), with the main button held, by a
/// host that from then on gives the panel's requests no answer (a VST3
/// host's `resizeView` refusing, which nice-plug discards, so nothing tells
/// the plugin; or a CLAP host saying yes and resizing nothing): two passes,
/// the first following the step (the zoom to 300 %), the second asking the
/// window back to 300 % (1560 × 840), whose answer is then awaited.
fn snapped_past_the_maximum() -> Hosted {
    let mut h = Hosted::open(2.5, 1.0, Answer::Exact, Format::Vst3);
    h.idle(1);
    h.world.answer.set(Answer::None);
    h.hold(true);
    h.give(BASE * 3.3);
    let before = h.requests().len();
    h.idle(2);
    assert_eq!((h.zoom(), h.pixels()), (3.0, h.panel_at(3.0)), "snapped back");
    assert_eq!(h.requests().len(), before + 1, "asked once: {:?}", h.requests());
    h
}

#[test]
fn zoom_drag_step_2a_with_the_button_held_a_step_past_the_maximum_just_after_a_snap_is_followed() {
    // The review's probe (W-D1's, without the window part: the window left
    // 1716 × 924 around a 1560 × 840 panel). The drag's next step, 1716 ×
    // 924 again, lands at once after the request to go back to 300 %, well
    // within ANSWER_WITHIN of it. Without point 2 (a) it is that request's
    // answer, kept off the window and never seen; with the button held it
    // is a drag, followed: the window takes it, and the panel asks it back
    // to 300 % again. The host's view stays 1716 × 924 either way, as this
    // host answers nothing.
    let mut h = snapped_past_the_maximum();
    let before = h.requests().len();
    h.give(BASE * 3.3);
    h.idle(1);
    assert_eq!(h.requests().len(), before + 1, "the step seen and the window asked back again: {:?}", h.requests());
    h.hold(false);
    h.idle(4);
    assert_eq!((h.zoom(), h.pixels()), (3.0, h.panel_at(3.0)), "the window the panel's size: {:?}", h.sizes);
}

#[test]
fn zoom_drag_step_2a_with_the_button_held_a_last_step_back_in_range_just_after_a_snap_is_followed() {
    // The review's second probe: after the request to go back to 300 %,
    // the drag's last step comes back in range at 280 %, at once, well
    // within ANSWER_WITHIN of that request. Without point 2 (a) it is the
    // request's answer, kept off the window: the panel stays at 300 % in a
    // view of 280 %, cut. With the button held it is followed: the window,
    // the panel and the host's view all 280 %, saved as a drag.
    let mut h = snapped_past_the_maximum();
    h.give(h.panel_at(2.8));
    h.idle(1);
    h.hold(false);
    h.idle(4);
    assert!(near(h.pixels(), h.panel_at(2.8)), "window {:?}, zoom {}", h.pixels(), h.zoom());
    assert!(h.fits(), "window {:?}, zoom {}", h.pixels(), h.zoom());
    assert_eq!(h.view(), h.pixels(), "the host's view is the window's");
    assert_eq!(h.followed().last().map(|(_, d)| *d), Some(true), "saved as a drag: {:?}", h.followed());
}

#[test]
fn zoom_drag_step_2a_an_answer_inside_the_request_with_the_button_held_answers_it_all_the_same() {
    // A CLAP host answering the request to go back to 300 % inside it,
    // the button held: the answer is given to the window (a drag's size),
    // and the request is answered, so the next size the host gives with the
    // button up, at once after (well within ANSWER_WITHIN), is its own:
    // followed, not kept off as an answer.
    let mut h = Hosted::open(2.5, 1.0, Answer::Exact, Format::Clap);
    h.idle(1);
    h.hold(true);
    h.give(BASE * 3.3);
    let before = h.requests().len();
    h.idle(2);
    assert_eq!(h.requests().len(), before + 1, "{:?}", h.requests());
    assert_eq!((h.zoom(), h.pixels(), h.view()), (3.0, h.panel_at(3.0), h.panel_at(3.0)), "answered inside");
    h.hold(false);
    h.give(h.panel_at(2.0));
    h.idle(4);
    assert!(h.fits() && near(h.pixels(), h.panel_at(2.0)), "followed: window {:?}, zoom {}", h.pixels(), h.zoom());
    assert_eq!(h.followed().last().map(|(_, d)| *d), Some(false), "{:?}", h.followed());
}

// ---- Everything else passes straight through ----

#[test]
fn w_d3_every_other_call_on_the_handle_and_the_callbacks_passes_straight_through() {
    let pairing = Pairing::new(Arc::new(|| false));
    let child = Child::default();
    let calls = child.calls.clone();
    let handle = pairing.around(child, measure);
    let win = Win::default();
    let parent = ParentWindowHandle::Win32Hwnd(std::num::NonZeroIsize::new(1).unwrap());
    assert_eq!(handle.set_parent(parent, &win), Ok(()));
    assert_eq!(handle.show(&win), Ok(()));
    assert_eq!(handle.hide(&win), Err(std::fmt::Error));
    handle.host_main_thread_callback(&win);
    assert_eq!(handle.adjust_size(NativeSize::new(3, 4), &win), Some(NativeSize::new(7, 8)));
    assert_eq!(handle.set_fallback_scale_factor(1.5, &win), Err(std::fmt::Error));
    assert!(handle.on_virtual_key_from_host(VirtualKeyCode::Space, true, KeyModifiers::empty()));
    handle.state_changed();
    handle.param_value_changed("mix", 0.5);
    handle.param_modulation_changed("mix", 0.25);
    assert_eq!(<Paired<Child> as EditorHandle>::run_until_closed(Win::default()), Ok(()));
    assert!(RAN_UNTIL_CLOSED.load(Ordering::SeqCst));
    assert_eq!(
        *calls.lock().unwrap(),
        [
            "set_parent",
            "show",
            "hide",
            "host_main_thread_callback",
            "adjust_size 3x4",
            "set_fallback_scale_factor 1.5",
            "on_virtual_key_from_host true",
            "state_changed",
            "param_value_changed mix 0.5",
            "param_modulation_changed mix 0.25",
        ]
    );
    let world = Rc::new(World {
        win: Win::default(),
        view: Cell::new(Vec2::ZERO),
        handle: OnceCell::new(),
        answer: Cell::new(Answer::None),
        format: Cell::new(Format::Clap),
        requests: RefCell::default(),
        due: RefCell::default(),
    });
    let called = Arc::new(AtomicBool::new(false));
    let mut methods = pairing.callbacks(HostMethods {
        callbacks: Box::new(Host(world.clone())),
        main_thread_caller: Box::new(Caller(called.clone())),
    });
    methods.callbacks.destroyed();
    methods.main_thread_caller.call_main_thread();
    assert_eq!(*world.requests.borrow(), [vec2(-1.0, -1.0)], "destroyed passed on");
    assert!(called.load(Ordering::SeqCst));
    // The wall clock, the default: a request now, a size at once after it,
    // well within ANSWER_WITHIN, is its answer, kept off the window.
    assert!(methods.callbacks.request_resize(Size::Physical(PhysicalSize::new(10, 10)), 1.0).is_ok());
    handle.set_size(NativeSize::new(20, 20), &win).unwrap();
    assert_eq!(win.pixels.get(), Vec2::ZERO, "kept off the window");
}
