//! The zoom drift cure's window part (W-D3; the design's (B),
//! `docs/proposals/zoom-drift-cure.md`, with its amendment for card
//! ZOOM-DRAG-STEP; Ian, 2026-11-26): what only the plugin's window code
//! can see, the host's calls themselves, put to the panel's
//! `zoom::follow`, which sees only the window egui gives it each pass.
//! Klinga's, through this crate; Rödalog mirrors it in its own window code
//! (ZOOM-DRIFT-CURE point 3 (a), its task R-1). No `unsafe`.
//!
//! What it wraps (the design's §1): the host's callbacks, through which
//! baseview sends the host each size the panel's window asks for
//! ([`Pairing::callbacks`]), and the window's handle, through which the
//! host gives the window its sizes ([`Pairing::around`]). Both run on the
//! host's GUI thread, as do the panel's passes ([`Pairing::pass`]), so the
//! lock is never contended; a lock, not a cell, as nice-plug needs the
//! handle to be `Send`.

use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use nice_plug::editor::dpi::{NativeSize, Size};
use nice_plug::editor::{EditorHandle, HostCallbacks, HostMethods, Modifiers, ParentWindowHandle, VirtualKeyCode};
use panelwerk_panel::egui::Context;
use panelwerk_panel::zoom::{self, ANSWER_WITHIN};

/// Whether the mouse's main button is held at this moment: the physical
/// left button, or the right one if the user has swapped them (Ian,
/// 2026-11-26, ZOOM-DRIFT-CURE point 1 (a); on Windows `GetAsyncKeyState`
/// of the physical button `GetSystemMetrics(SM_SWAPBUTTON)` names, read
/// on the GUI thread). The plugin gives it: reading it needs the platform's
/// API, which this crate doesn't depend on, so it is the seam a native
/// test gives a stand-in for. See [`Pairing`]'s example.
pub type Button = Arc<dyn Fn() -> bool + Send + Sync>;

/// A clock: the time since any fixed moment, for pairing a request with
/// its answer within [`ANSWER_WITHIN`]. [`Pairing::new`] uses the wall
/// clock, as the host's calls between the panel's passes have no egui time
/// (the design's (B): "wall-clock, GUI thread, plugin code, with an
/// injectable clock for tests"); a test gives one it moves itself
/// ([`Pairing::with_clock`]), so no test sleeps.
pub type Clock = Arc<dyn Fn() -> Duration + Send + Sync>;

/// One panel window's window part: made in each `Editor::spawn`, before the
/// window, its [`callbacks`](Pairing::callbacks) given to the window in
/// place of the host's and the window's handle wrapped
/// ([`around`](Pairing::around)); in each pass, before
/// `zoom::follow`, [`pass`](Pairing::pass); the plugin's `ZoomHost`
/// answering `answers_paired` with `true`, and its `zoom_followed` asking
/// [`dragged`](Pairing::dragged) whether to save a followed size as the
/// user's (ZOOM-DRIFT-CURE point 1 (a)). Then:
///
/// - **An answer kept off the window.** The first size the host gives after
///   the panel's request, inside the request (CLAP's `request_resize`
///   calling `gui.set_size`) or within [`ANSWER_WITHIN`] after it (VST3:
///   nice-plug posts the request, and the host's `onSize` comes inside its
///   `resizeView`), is the host's answer: not given to the window, so the
///   panel keeps the size it asked for and the host's view shows what fits
///   (Rödalog's B-PNL-43). Every other size the host gives is its own,
///   given to the window, and `follow` follows it.
/// - **A refusal reported.** The host refusing a request (CLAP's `false`)
///   calls `zoom::refused` at once, so a menu choice waiting on it is
///   undone by the very next frame (ZOOM-DRIFT-CURE point 4 (a)).
/// - **A drag is never an answer** (ZOOM-DRAG-STEP point 2 (a)): a size the
///   host gives while the main button is held ([`Button`]) is given to the
///   window and followed, even when it is the first after a request. It
///   leaves the request's answer still awaited: the next size given with
///   the button up within [`ANSWER_WITHIN`] of the request is its answer.
///   One given inside the request is that request's answer all the same,
///   given to the window if the button is held.
/// - **A drag step not overwritten** (ZOOM-DRAG-STEP point 1 (a)): a request
///   the panel makes from a view of the window older than a size the host
///   has given it since is not sent. It fails, so baseview puts the window
///   back at that size (its `WM_SIZE` revert), and the next pass, seeing it,
///   follows it. Which view a request is made from: egui gives each pass
///   the window as it is at the pass's start, but on a pass a new zoom
///   takes effect, the last pass's window rescaled (egui 0.37
///   `Context::begin_pass`; `zoom::follow` works from the same), so a
///   corner-drag step that lands before the last step's zoom has drawn is
///   not in it. No refusal is reported for it: the host was never asked,
///   and a menu choice waiting is judged by `follow` on the next pass
///   against the window it then sees.
///
/// Before the window's first pass nothing is refused as stale (the
/// request baseview makes as it shows the window is for the window's own
/// size), and a host's refusal then has no menu choice to undo. A plugin
/// that never calls [`pass`](Pairing::pass) gets no drag step kept and no
/// refusal reported, the rest as above.
///
/// ```
/// use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
/// use std::sync::Arc;
/// use std::time::Duration;
/// use panelwerk_nice::Pairing;
/// use panelwerk_panel::egui::Context;
///
/// // In a plugin, `Pairing::new` with the platform's reading of the button.
/// // Here a test's own button and clock.
/// let held = Arc::new(AtomicBool::new(false));
/// let now = Arc::new(AtomicU64::new(0));
/// let (h, n) = (held.clone(), now.clone());
/// let pairing = Pairing::with_clock(
///     Arc::new(move || h.load(Ordering::SeqCst)),
///     Arc::new(move || Duration::from_millis(n.load(Ordering::SeqCst))),
/// );
/// // In `Editor::spawn`: `pairing.callbacks(host)` for the window's host
/// // methods, `pairing.around(handle, measure)` for its handle. In each
/// // pass, before `zoom::follow`:
/// pairing.pass(&Context::default());
/// assert!(!pairing.dragged(), "no size given by the host yet");
/// ```
#[derive(Clone)]
pub struct Pairing {
    state: Arc<Mutex<State>>,
    button: Button,
    clock: Clock,
}

/// What the window part has seen, shared by the callbacks, the handle and
/// the plugin's passes.
#[derive(Default)]
struct State {
    /// The panel's context, from its last pass, for `zoom::refused`.
    ctx: Option<Context>,
    /// When the panel's last request was made, its answer still awaited.
    pending: Option<Duration>,
    /// Inside the host's handling of a request (CLAP's answer comes there).
    inside: bool,
    /// Whether the main button was held at the last size the host gave
    /// the window that changed it.
    dragged: bool,
    /// The zoom of the panel's last pass.
    zoom: Option<f32>,
    /// How many sizes of the host's own have changed the window so far.
    given: u64,
    /// `given` at the panel's last pass.
    given_at_pass: u64,
    /// `given` as the view of the window the panel's last pass worked from
    /// had it: a request made with more given since is stale. None before
    /// the first pass.
    basis: Option<u64>,
}

impl Pairing {
    /// A window part reading `button` at each size the host gives, on the
    /// wall clock ([`Clock`]).
    pub fn new(button: Button) -> Self {
        let start = Instant::now();
        Self::with_clock(button, Arc::new(move || start.elapsed()))
    }

    /// A window part on `clock` (a test's).
    pub fn with_clock(button: Button, clock: Clock) -> Self {
        Pairing { state: Arc::default(), button, clock }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The host's methods as nice-plug gives them to `Editor::spawn`, with
    /// its callbacks wrapped, for the window (nice-plug-egui's editor's
    /// `spawn`) in their place: each request the window makes is paired
    /// with its answer, or refused if it is stale, and a refusal by the
    /// host is reported. The main-thread caller passes straight through.
    pub fn callbacks(&self, host: HostMethods) -> HostMethods {
        HostMethods {
            callbacks: Box::new(Asking { inner: host.callbacks, pairing: self.clone() }),
            main_thread_caller: host.main_thread_caller,
        }
    }

    /// The window's handle, `inner`, wrapped so that each size the host
    /// gives is judged here first: an answer is kept off the window, any
    /// other passed on. `measure` reads the window's size in pixels
    /// (baseview's `Window::size`), so that a size the host gives that
    /// leaves the window as it was (a host repeating its view's size) is
    /// not counted as one the panel has yet to see. Every other call passes
    /// straight through.
    pub fn around<H: EditorHandle>(&self, inner: H, measure: fn(&H::Window) -> NativeSize<u32>) -> Paired<H> {
        Paired { inner, pairing: self.clone(), measure }
    }

    /// The panel's pass in `ctx`, once a pass, before any request it makes
    /// (in nice-plug-egui's `ui`; the requests come after the pass): the
    /// view of the window this pass works from, and the context a refusal
    /// is reported to.
    ///
    /// egui may run a second pass in one run (egui 0.37's `Context::run`,
    /// when a widget asks for a discard: a new `Grid`, a `Resize`'s sizing
    /// pass). That pass is given no new input, so it works from the first
    /// pass's view of the window (rescaled, if a zoom takes effect on it):
    /// it keeps the first pass's view here and records only its zoom and
    /// context. On a run of one pass, every pass is a first.
    pub fn pass(&self, ctx: &Context) {
        let zoom = ctx.zoom_factor();
        let mut s = self.state();
        if ctx.current_pass_index() == 0 {
            // On a new zoom's pass egui gives the last pass's window, rescaled.
            let new_zoom = s.zoom.is_some_and(|z| z != zoom);
            s.basis = Some(if new_zoom { s.given_at_pass } else { s.given });
            s.given_at_pass = s.given;
        }
        // On a second pass too: a zoom taking effect there is the one the
        // next run's first pass compares with, as that pass is given the
        // window itself, not a rescaled view.
        s.zoom = Some(zoom);
        s.ctx = Some(ctx.clone());
    }

    /// Whether the main button was held at the last size the host gave
    /// that changed the window: for the plugin's `zoom_followed`, a drag to
    /// save as the user's choice, or else the host's own layout
    /// (ZOOM-DRIFT-CURE point 1 (a); its fallback (b), saving menu choices
    /// only, is the plugin's, by not asking).
    pub fn dragged(&self) -> bool {
        self.state().dragged
    }
}

/// The error a stale request fails with, for baseview's log.
const STALE: &str = "panelwerk: a size the host gave since the panel last saw the window is kept";

/// [`ANSWER_WITHIN`] as a duration: the same quarter second `zoom::follow`
/// awaits an answer for without the window part (the design's (B) pairs
/// each request with the next host size "within 250 ms of wall-clock
/// time"; `zoom`'s `ANSWER_WITHIN` doc says it is "within the same quarter
/// second").
fn within() -> Duration {
    Duration::from_secs_f64(ANSWER_WITHIN)
}

/// The host's callbacks as the window is given them ([`Pairing::callbacks`]).
struct Asking {
    inner: Box<dyn HostCallbacks>,
    pairing: Pairing,
}

impl HostCallbacks for Asking {
    fn request_resize(&mut self, new_size: Size, scale_factor: f64) -> Result<(), Box<dyn Error>> {
        {
            let mut s = self.pairing.state();
            if s.basis.is_some_and(|b| s.given > b) {
                // Stale (point 1 (a)): baseview puts the window back at the
                // host's size, the host never hears of it.
                return Err(STALE.into());
            }
            s.pending = Some((self.pairing.clock)());
            s.inside = true;
        }
        // Unlocked: a CLAP host answers by calling the handle's `set_size`
        // inside this call.
        let asked = self.inner.request_resize(new_size, scale_factor);
        let ctx = {
            let mut s = self.pairing.state();
            s.inside = false;
            if asked.is_err() {
                s.pending = None;
            }
            s.ctx.clone()
        };
        // After the pass (baseview resizes the window after egui's pass), so
        // the context isn't locked; the new zoom takes effect at the next.
        if let (Err(_), Some(ctx)) = (&asked, ctx) {
            zoom::refused(&ctx);
        }
        asked
    }

    fn destroyed(&mut self) {
        self.inner.destroyed();
    }
}

/// A panel window's handle with the window part ([`Pairing::around`]).
///
/// ```
/// use std::sync::Arc;
/// use nice_plug::editor::dpi::NativeSize;
/// use panelwerk_nice::{Paired, Pairing};
///
/// // nice-plug-egui's handle and baseview's window size in a plugin; here
/// // the framework's stand-in handle, whose window is `()`.
/// let handle: Paired<()> = Pairing::new(Arc::new(|| false)).around((), |_| NativeSize::new(0, 0));
/// # let _ = handle;
/// ```
pub struct Paired<H: EditorHandle> {
    inner: H,
    pairing: Pairing,
    measure: fn(&H::Window) -> NativeSize<u32>,
}

impl<H: EditorHandle> EditorHandle for Paired<H> {
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
    fn hide(&self, window: &Self::Window) -> Result<(), Self::Error> {
        self.inner.hide(window)
    }
    /// A size the host gives: its answer to the panel's request kept off
    /// the window, unless the button is held; any other given to it.
    fn set_size(&self, new_size: NativeSize<u32>, window: &Self::Window) -> Result<(), Self::Error> {
        let held = (self.pairing.button)();
        let now = (self.pairing.clock)();
        {
            let mut s = self.pairing.state();
            // Saturating: an injected clock may go back (`with_clock`); then
            // no time has passed since the request, not a panic.
            let answer = s.pending.is_some_and(|at| s.inside || now.saturating_sub(at) <= within());
            if answer && !held {
                s.pending = None;
                return Ok(());
            }
            // Held (point 2 (a)): given to the window. Inside the request
            // it is still the request's answer; after it, the answer is
            // still awaited.
            if s.inside {
                s.pending = None;
            }
        }
        let before = (self.measure)(window);
        let set = self.inner.set_size(new_size, window);
        if (self.measure)(window) != before {
            let mut s = self.pairing.state();
            s.dragged = held;
            s.given += 1;
        }
        set
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
