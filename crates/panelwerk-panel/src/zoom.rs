//! Resizing and scale (Kör B-PNL-6; PW-7: the mechanism shared, the range
//! and the menu's sizes each consumer's): the panel is laid out at its base
//! size and scaled as a whole by egui's zoom, keeping its aspect ratio.
//!
//! How the window follows (nice-plug-egui 0.6 on egui-baseview 0.9): egui
//! applies a new zoom at the start of the next pass, and egui-baseview then
//! resizes the window to the size it last had, in the old zoom's points,
//! times the new zoom. So the menu's choice needs nothing more to resize the
//! window; a size the host gives (the window's corner dragged) is turned
//! into a zoom, and on the pass the new zoom takes effect [`follow`] asks
//! for the base size in points ([`egui::ViewportCommand::InnerSize`]),
//! which egui-baseview multiplies by the zoom: the allowed size nearest the
//! one asked for.
//!
//! The host's answer to that request is not a size of its own (Ian,
//! 2026-11-26, ZOOM-DRIFT-CURE): before v0.6.0 `follow` read an answer a
//! pixel short as the corner dragged, a new zoom, saved and asked for
//! again, so the zoom drifted, ran away, or the window resized every pass.
//! Now the answer is kept as it is and never asked for again, a menu choice
//! is told once the window has taken it, and a plugin whose window code
//! pairs requests and answers itself says so ([`ZoomHost::answers_paired`]).
//! The design and its fact-check are in `docs/proposals/`.
//!
//! That is egui-baseview's model, and it is all the tests here check. Under
//! VST3 in Cubase LE 16 the size also needs three framework patches, which
//! each consumer carries in its own vendored copy, not here (W-12): the
//! panel's resize request reaching the host (Rödalog's patch 10), the
//! view taking the host's scale factor before `attached` (patch 12), and
//! baseview's `WM_PAINT` leaving the whole window valid (patch 13), all in
//! Rödalog's `vendor/PATCHES.md`. Kör and Fläns carry all three (their
//! `vendor/PATCHES.md`, Changes 4 to 6, read 2026-11-23); Klinga, with no
//! editor yet, carries none of them (its Changes 1 to 3). A consumer that
//! adopts `follow` without them passes every test here and can still
//! resize wrongly in Cubase.
//!
//! ```
//! use panelwerk_panel::zoom::Range;
//!
//! const KOER: Range = Range::new(0.75, 3.0, &[0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0]);
//! assert_eq!(KOER.clamp(5.0), 3.0);
//! ```

use egui::{Align2, Context, Id, Order, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, ViewportCommand};

use crate::font;
use crate::palette::{BORDER, CHOICE_TEXT, CHOSEN, HOVER, SECTION};

/// A consumer's zoom range and its menu's sizes (Kör's 75 … 300 %).
///
/// [`Range::new`], the only way to make one (W-6b), guarantees `min <= max`
/// with both finite, which [`Range::clamp`], called by every zoom path,
/// relies on: `f32::clamp` panics otherwise, which would be on the GUI
/// thread, inside the host. `new` also refuses an empty menu (whose
/// [`menu`](fn@menu) would have no last row) and a menu not strictly
/// smallest first. A bad `const` fails to compile.
///
/// Why private fields read by [`Range::min`], [`Range::max`] and
/// [`Range::menu`], not `#[non_exhaustive]` with the fields public (the
/// review's suggestion, the same in what it refuses of a literal):
/// `non_exhaustive` leaves the fields assignable, so a range made by `new`
/// could still be given a NaN bound afterwards, in a `const` block too. With
/// private fields what `new` checked stays checked.
///
/// ```
/// use panelwerk_panel::egui::vec2;
/// use panelwerk_panel::zoom::Range;
///
/// let r = Range::new(0.75, 3.0, &[1.0, 2.0]);
/// assert_eq!(r.for_size(vec2(780.0, 420.0), vec2(520.0, 280.0)), 1.5);
/// assert_eq!((r.min(), r.max(), r.menu()), (0.75, 3.0, &[1.0, 2.0][..]));
/// ```
///
/// Outside this crate a struct literal doesn't compile (W-6b):
///
/// ```compile_fail
/// use panelwerk_panel::zoom::Range;
///
/// const KOER: Range = Range { min: 0.75, max: 3.0, menu: &[1.0] };
/// ```
///
/// Nor does changing a field of a range already made:
///
/// ```compile_fail
/// use panelwerk_panel::zoom::Range;
///
/// let mut r = Range::new(0.75, 3.0, &[1.0]);
/// r.min = f32::NAN;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    min: f32,
    max: f32,
    menu: &'static [f32],
}

impl Range {
    /// The range from `min` to `max`, with `menu`'s sizes: refused, by a
    /// panic, unless `min <= max`, both finite, and `menu` has at least one
    /// size, smallest first. In a `const` the refusal is a compile error,
    /// so a typo in a consumer's range never reaches a host (W-6).
    ///
    /// ```
    /// use panelwerk_panel::zoom::Range;
    ///
    /// const FLAENS: Range = Range::new(0.75, 3.0, &[0.75, 1.0, 1.5, 2.0, 3.0]);
    /// assert_eq!(FLAENS.clamp(0.5), 0.75);
    /// ```
    ///
    /// ```compile_fail
    /// use panelwerk_panel::zoom::Range;
    ///
    /// // min above max: refused as the crate compiles.
    /// const _: Range = Range::new(3.0, 0.75, &[1.0]);
    /// ```
    pub const fn new(min: f32, max: f32, menu: &'static [f32]) -> Range {
        assert!(min.is_finite() && max.is_finite(), "a zoom range's min and max must be finite");
        assert!(min <= max, "a zoom range needs min <= max");
        assert!(!menu.is_empty(), "a zoom range's menu needs at least one size");
        let mut k = 1;
        while k < menu.len() {
            // `>`, not `>=`: a size twice is a typo, and a NaN compares
            // false, so it's refused too.
            assert!(menu[k] > menu[k - 1], "a zoom range's menu must go smallest first");
            k += 1;
        }
        Range { min, max, menu }
    }

    /// The smallest zoom.
    pub const fn min(&self) -> f32 {
        self.min
    }

    /// The largest zoom.
    pub const fn max(&self) -> f32 {
        self.max
    }

    /// The menu's sizes, smallest first, at least one.
    pub const fn menu(&self) -> &'static [f32] {
        self.menu
    }

    /// `zoom` within the range; anything not a number is 100 %.
    pub fn clamp(&self, zoom: f32) -> f32 {
        if zoom.is_nan() {
            1.0
        } else {
            zoom.clamp(self.min, self.max)
        }
    }

    /// The zoom for a window `size` points (at zoom 1) of a panel `base`
    /// points: its height's share of the base height, as nice-plug's
    /// aspect-ratio adjustment goes by the height, within the range.
    pub fn for_size(&self, size: Vec2, base: Vec2) -> f32 {
        self.clamp(size.y / base.y)
    }
}

/// What the zoom needs from the plugin.
///
/// ```
/// use std::cell::Cell;
/// use panelwerk_panel::zoom::ZoomHost;
///
/// struct Saved(Cell<f32>);
/// impl ZoomHost for Saved {
///     fn zoom_chosen(&self, zoom: f32) { self.0.set(zoom) }
/// }
/// ```
pub trait ZoomHost {
    /// The user chose `zoom` from the menu, and the window has taken it (or
    /// the plugin called [`choose`]); by default also every size followed,
    /// as [`ZoomHost::zoom_followed`]'s default calls this: saved with the
    /// project and, when the window closes, in the settings file.
    fn zoom_chosen(&self, zoom: f32);

    /// The window followed a size the host gave it on its own, not its
    /// answer to the panel's request (the corner dragged, or the host's own
    /// layout): `zoom` fits it. A plugin that can tell a drag from the
    /// host's layout overrides this, to save only a drag (Ian, 2026-11-26,
    /// ZOOM-DRIFT-CURE, point 1 (a)); by default it is the user's choice,
    /// as v0.5.0 read every size the host gave.
    fn zoom_followed(&self, zoom: f32) {
        self.zoom_chosen(zoom)
    }

    /// Whether the plugin's window code pairs each of the panel's resize
    /// requests with the host's answer, keeps the answer off the window,
    /// and reports a refusal by calling [`refused`] (the cure's window part,
    /// `docs/proposals/zoom-drift-cure.md` (B)). Then every size that
    /// reaches the window is the host's own, but a refused request's, which
    /// baseview puts back as the window was and [`refused`] accounts for,
    /// and [`follow`] awaits no answer. False by default (rule 12): [`follow`]
    /// then takes the window on the first pass after a request, if within
    /// [`ANSWER_WITHIN`] of it, as the answer.
    fn answers_paired(&self) -> bool {
        false
    }
}

/// How long after the panel asks for a size, in seconds of egui's input
/// time, the window's size on the next pass is still the host's answer
/// rather than a size of the host's own: a quarter second (Ian, 2026-11-26,
/// ZOOM-DRIFT-CURE; the design's (A), the fact-check's finding 4). Why a
/// time and not "the next frame" alone: frames come on demand, so the next
/// one may come seconds later, and the first step of a drag then would be
/// swallowed as an answer. Why a quarter second: a host answers inside its
/// handling of the request (VST3's `resizeView`, from the task the
/// framework posts; CLAP's `request_resize`), normally before the next
/// frame, far sooner (supposed, not yet proved on Windows: the design's
/// §6). The cost: a size of the host's own that is the first the panel
/// sees this soon after a request is kept as it is, not followed: after a
/// request the host never answered; when it lands in the same gap as the
/// answer (both before the next pass, the window then the later); and in a
/// corner drag past the range's limits or off the aspect ratio, where the
/// panel asks the window back to its shape and the drag's next step is
/// taken as that request's answer (open: Ian's card ZOOM-DRAG-STEP,
/// [`follow`]'s note). The design's window part (B, task W-D3) pairs a
/// request with its answer within the same 250 ms.
pub const ANSWER_WITHIN: f64 = 0.25;

/// The plugin's window code saw the host refuse the panel's last resize
/// request (CLAP's `false`; the design's (B)): a menu choice waiting on it
/// is undone at once, so the very next frame is drawn at the zoom before
/// it, and it is never told (Ian, 2026-11-26, ZOOM-DRIFT-CURE, point 4 (a)).
/// No answer is awaited for a refused request, and the window, back as the
/// panel last saw it, is not then taken for a size of the host's own. One
/// that was no menu choice's (the opening's, or a window asked back to its
/// shape) changes no zoom and is not asked again. Called on the GUI thread,
/// between passes.
///
/// ```
/// use panelwerk_panel::egui::Context;
/// use panelwerk_panel::zoom;
///
/// let ctx = Context::default();
/// ctx.set_zoom_factor(1.25);
/// // A new zoom takes effect at the start of the next pass.
/// ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
/// zoom::refused(&ctx);
/// assert_eq!(ctx.zoom_factor(), 1.25, "nothing to undo");
/// ```
pub fn refused(ctx: &Context) {
    let choice = ctx.data_mut(|d| {
        d.remove_temp::<Awaited>(awaited_id());
        // Baseview put the window back at the size it had when the panel last
        // looked, not the base size the panel asked for. Without this, a
        // window asked back to its shape and refused would be found off its
        // shape again on the next pass and asked again, every pass (with
        // `answers_paired`, which awaits nothing to absorb it).
        if let Some(seen) = d.get_temp::<Seen>(seen_id()) {
            d.insert_temp(seen_id(), Seen { asked: false, ..seen });
        }
        d.remove_temp::<Choice>(choice_id())
    });
    if let Some(c) = choice {
        ctx.set_zoom_factor(c.from);
    }
}

/// Choose `zoom`, held to `range`: the window follows once it takes effect,
/// and the host is told at once and saves it. The menu doesn't call this: it
/// tells the host only once the window has taken its size. A menu choice
/// still waiting to be told is replaced, neither told nor undone.
pub fn choose(ctx: &Context, host: &dyn ZoomHost, range: &Range, zoom: f32) {
    let zoom = range.clamp(zoom);
    // The plugin's zoom replaces the menu's waiting one. Left waiting, the
    // menu's would be told after this one (the plugin saving a zoom it no
    // longer shows), or judged on the pass this zoom takes effect, when egui
    // gives the window as it was, so taken for refused and undone. Rejected:
    // judging a choice only on a pass with no new zoom, which still tells
    // the menu's zoom after the plugin's.
    ctx.data_mut(|d| {
        d.remove_temp::<Proposed>(proposed_id());
        d.remove_temp::<Choice>(choice_id());
    });
    ctx.set_zoom_factor(zoom);
    host.zoom_chosen(zoom);
}

/// The menu's choice of `zoom`, held to `range`: the host is told once the
/// window has taken it ([`follow`]), so a host refusing the size leaves the
/// zoom and the saved zoom as they were (ZOOM-DRIFT-CURE). The current
/// zoom needs nothing of the window, so it is told at once.
fn propose(ctx: &Context, host: &dyn ZoomHost, range: &Range, zoom: f32) {
    let zoom = range.clamp(zoom);
    if zoom == ctx.zoom_factor() {
        host.zoom_chosen(zoom);
    } else {
        ctx.set_zoom_factor(zoom);
        ctx.data_mut(|d| d.insert_temp(proposed_id(), Proposed(zoom)));
    }
}

/// What the panel saw the last pass: the screen's size, the zoom, and
/// whether it asked for the base size (after which the window should be
/// at the base size, whatever it was).
#[derive(Clone, Copy, PartialEq)]
struct Seen {
    screen: Vec2,
    zoom: f32,
    asked: bool,
}

/// A request for a size the window didn't have, at this input time: the
/// panel's own (a new zoom's size, the window asked back to its shape) or
/// baseview's as it shows the window (awaited from the window's first
/// pass). The next pass's window is the host's answer, if within
/// [`ANSWER_WITHIN`].
#[derive(Clone, Copy, Default)]
struct Awaited(f64);

/// The menu's choice, set and waiting for its zoom to take effect (dropped
/// by [`choose`], which replaces it).
#[derive(Clone, Copy, Default)]
struct Proposed(f32);

/// The menu's choice taken effect, its size asked for: told on the next
/// pass unless the window is back at `before` (its pixels, before the
/// request), the host having refused it, or [`refused`] undid it first, or
/// [`choose`] replaced it.
#[derive(Clone, Copy, Default)]
struct Choice {
    zoom: f32,
    from: f32,
    before: Vec2,
}

fn seen_id() -> Id {
    Id::unique("panelwerk_zoom_seen")
}

fn awaited_id() -> Id {
    Id::unique("panelwerk_zoom_awaited")
}

fn proposed_id() -> Id {
    Id::unique("panelwerk_zoom_proposed")
}

fn choice_id() -> Id {
    Id::unique("panelwerk_zoom_choice")
}

/// Half a pixel: two window sizes within it in both dimensions are the same
/// window, as a size worked from a zoom is rounded to whole pixels, which
/// moves it by at most this.
const HALF_PIXEL: f32 = 0.5;

/// How far, in points, the window may be from the base size before a size
/// the host gives is followed: v0.5.0's tolerance (no source recorded with
/// it), kept as it was so a host's own size is followed as before.
const OFF_SIZE: f32 = 0.5;

/// Whether two sizes in pixels are the same window ([`HALF_PIXEL`]).
fn same(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_elem() <= HALF_PIXEL
}

/// Once a pass, before drawing a panel of `base` points: follows a size the
/// host gave the window on its own, and keeps the window at the base size ×
/// the zoom; the host's answer to the panel's own request is never taken
/// for a new zoom and never asked for again (Ian, 2026-11-26,
/// ZOOM-DRIFT-CURE; `docs/tasks-v0.6.md` has the rule).
///
/// - **An answer:** after the window's first pass (baseview asks for the
///   opening size as it shows the window), and after a new zoom or a
///   window off its shape when the window must change size, the next pass's
///   window, if within [`ANSWER_WITHIN`], is the host's answer: kept as it
///   is, whatever its size. With [`ZoomHost::answers_paired`] nothing is
///   awaited: the plugin keeps the answers off the window.
/// - **A menu choice** is told ([`ZoomHost::zoom_chosen`]) on the pass after
///   its size was asked for, unless the window is back at its old size (a
///   CLAP host refused it, and baseview put the window back): then the zoom
///   goes back and nothing is told. If the window, as the panel last saw
///   it, already had its size, nothing is awaited and it is told at once.
/// - **Any other size** the host gives is its own: the zoom for its height
///   ([`ZoomHost::zoom_followed`]); off the aspect ratio or past the range,
///   the window is asked back to its shape.
///
/// Open, not cured (Ian's card ZOOM-DRAG-STEP): whether the window must
/// change size is judged from the window as egui last gave it, which on a
/// new zoom's pass is the last pass's window rescaled. So a corner-drag
/// step that lands before the previous step's zoom has drawn is not seen:
/// the request for the previous step's size reaches the host, overwrites
/// the step, and is not awaited, so a host answering it a pixel short has
/// its answer followed as a new zoom and saved (one drift step). And after
/// the panel asks the window back to its shape (off the aspect ratio or
/// past the range) the drag's next step, within [`ANSWER_WITHIN`], is taken
/// as that request's answer and kept, not followed: dragged past the
/// largest zoom the window stays larger than the panel; a last step back in
/// range leaves the panel cut. The design's §4 (iii) counts it an answer,
/// its §2 has every drag step followed.
pub fn follow(ctx: &Context, host: &dyn ZoomHost, base: Vec2, range: &Range) {
    let screen = ctx.content_rect().size();
    let zoom = ctx.zoom_factor();
    let time = ctx.input(|i| i.time);
    let paired = host.answers_paired();
    let before = ctx.data(|d| d.get_temp::<Seen>(seen_id()));
    let Some(before) = before else {
        // The window's first pass only looks: the zoom it opened at has just
        // taken effect, and egui's screen for that pass is a guess. Baseview
        // asks the host for the window's size as it shows it, so the next
        // pass's window is that request's answer.
        ctx.data_mut(|d| {
            d.insert_temp(seen_id(), Seen { screen, zoom, asked: false });
            if !paired {
                d.insert_temp(awaited_id(), Awaited(time));
            }
        });
        return;
    };
    // The window in pixels. On the pass a new zoom takes effect egui doesn't
    // read the window: it gives the last pass's screen rescaled to the new
    // zoom (egui 0.37 `Context::begin_pass`, "to avoid jitter"), so then
    // this is the window as it was, which is what the panel's request
    // starts from. On any other pass egui-baseview's zoom is egui's.
    let native = ctx.native_pixels_per_point().unwrap_or(1.0);
    let pixels = screen * zoom * native;
    let (awaited, choice) =
        ctx.data_mut(|d| (d.remove_temp::<Awaited>(awaited_id()), d.remove_temp::<Choice>(choice_id())));
    // A menu choice is judged on the first pass after its request, however
    // late: a refusal puts the window back inside the request.
    let back = choice.is_some_and(|c| same(pixels, c.before));
    match choice {
        Some(c) if back => ctx.set_zoom_factor(c.from),
        Some(c) => host.zoom_chosen(c.zoom),
        None => {}
    }
    let answer = back || awaited.is_some_and(|Awaited(at)| time - at <= ANSWER_WITHIN);
    let mut asked = false;
    // The window asked for the base size: awaits the answer if the window
    // must change size, as baseview sends the host only a size that differs
    // from the window's. Judged from `from_pixels`, the window as egui last
    // gave it: a host size that landed since isn't seen, so its request
    // reaches the host unawaited (the open case in the note above).
    let mut ask = |from_pixels: Vec2| {
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(base));
        asked = true;
        let wanted = !same(from_pixels, base * zoom * native);
        if wanted && !paired {
            ctx.data_mut(|d| d.insert_temp(awaited_id(), Awaited(time)));
        }
        wanted
    };
    if before.zoom != zoom {
        // A new zoom has taken effect: the window to its size. If the window
        // as egui gives it (the last pass's, rescaled) already has it (a
        // corner-drag step just followed), nothing is awaited, so a following
        // drag step is followed (the fact-check's finding 1). A step that
        // landed before this pass is the open case in the note above.
        let proposed = ctx.data_mut(|d| d.remove_temp::<Proposed>(proposed_id()));
        let wanted = ask(pixels);
        match proposed {
            Some(Proposed(z)) if wanted => {
                ctx.data_mut(|d| d.insert_temp(choice_id(), Choice { zoom: z, from: before.zoom, before: pixels }));
            }
            Some(Proposed(z)) => host.zoom_chosen(z),
            None => {}
        }
    } else if !answer {
        // After the panel asked, the window should be at the base size;
        // otherwise as it was.
        let expected = if before.asked { base } else { before.screen };
        let off_size = (screen - base).abs().max_elem() > OFF_SIZE;
        if off_size && screen != expected {
            // The host resized the window on its own: the zoom for its size.
            let fits = range.for_size(screen * zoom, base);
            if fits == zoom {
                // Off the aspect ratio, or past the range: back to the size.
                // Its answer is awaited, so a drag's next step on the next
                // pass is kept, not followed (open, the note above).
                ask(pixels);
            } else {
                ctx.set_zoom_factor(fits);
                host.zoom_followed(fits);
            }
        }
    }
    ctx.data_mut(|d| d.insert_temp(seen_id(), Seen { screen, zoom, asked }));
}

/// A menu row's size, in points.
pub const MENU_ROW: Vec2 = Vec2::new(76.0, 20.0);

fn menu_id() -> Id {
    Id::unique("panelwerk_zoom_menu")
}

/// Opens the zoom menu (a consumer's right-click on its name).
pub fn open_menu(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp(menu_id(), true));
}

/// Row `k` of a menu whose first row's top left is `at`.
pub fn menu_row(at: Pos2, k: usize) -> Rect {
    Rect::from_min_size(at + Vec2::new(0.0, MENU_ROW.y * k as f32), MENU_ROW)
}

/// The zoom menu, while open, its rows from `at` down: the range's sizes,
/// the current one in white and the others in the choices' grey, as
/// Rödalog's lists; a click on one chooses it (told to the host once the
/// window has taken it: [`follow`]), and any click closes the menu (not the
/// right-click that opened it). Run once a pass, after the
/// controls, so it is drawn over them.
pub fn menu(ctx: &Context, host: &dyn ZoomHost, range: &Range, at: Pos2) {
    // No guard for an empty menu here (W-6 had one): `Range::new` is the
    // only way to make a range (W-6b), and it refuses one, so `len() - 1`
    // below can't underflow.
    if !ctx.data(|d| d.get_temp::<bool>(menu_id())).unwrap_or(false) {
        return;
    }
    let current = ctx.zoom_factor();
    let all = menu_row(at, 0).union(menu_row(at, range.menu.len() - 1));
    egui::Area::new(menu_id()).order(Order::Foreground).fixed_pos(all.min).show(ctx, |ui| {
        let p = ui.painter();
        p.rect_filled(all.expand(2.0), 4.0, SECTION);
        p.rect_stroke(all.expand(2.0), 4.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
        for (k, z) in range.menu.iter().enumerate() {
            let rect = menu_row(at, k);
            let r = ui.interact(rect, menu_id().with(k), Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(rect, 2.0, HOVER);
            }
            let colour = if (z - current).abs() < 1e-3 { CHOSEN } else { CHOICE_TEXT };
            let text = format!("{} %", (z * 100.0).round());
            ui.painter().text(rect.left_center() + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, text, font(), colour);
            if r.clicked() {
                propose(ctx, host, range, *z);
            }
        }
        ui.allocate_rect(all, Sense::hover());
    });
    if ctx.input(|i| i.pointer.any_released() && !i.pointer.button_released(PointerButton::Secondary)) {
        ctx.data_mut(|d| d.insert_temp(menu_id(), false));
    }
}
