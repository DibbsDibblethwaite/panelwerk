//! Resizing and scale (Kör B-PNL-6; PW-7: the mechanism shared, the range
//! and the menu's sizes each consumer's): the panel is laid out at its base
//! size and scaled as a whole by egui's zoom, keeping its aspect ratio.
//!
//! How the window follows (nice-plug-egui 0.6 on egui-baseview 0.9): egui
//! applies a new zoom at the start of the next pass, and egui-baseview then
//! resizes the window to the size it last had, in the old zoom's points,
//! times the new zoom. So the menu's choice needs nothing more; a size the
//! host gives (the window's corner dragged) is turned into a zoom, and on
//! the pass the new zoom takes effect [`follow`] asks for the base size in
//! points ([`egui::ViewportCommand::InnerSize`]), which egui-baseview
//! multiplies by the zoom: the allowed size nearest the one asked for.
//!
//! ```
//! use panelwerk_panel::zoom::Range;
//!
//! const KOER: Range = Range { min: 0.75, max: 3.0, menu: &[0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0] };
//! assert_eq!(KOER.clamp(5.0), 3.0);
//! ```

use egui::{Align2, Context, Id, Order, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, ViewportCommand};

use crate::font;
use crate::palette::{BORDER, CHOICE_TEXT, CHOSEN, HOVER, SECTION};

/// A consumer's zoom range and its menu's sizes (Kör's 75 … 300 %).
///
/// ```
/// use panelwerk_panel::egui::vec2;
/// use panelwerk_panel::zoom::Range;
///
/// let r = Range { min: 0.75, max: 3.0, menu: &[1.0, 2.0] };
/// assert_eq!(r.for_size(vec2(780.0, 420.0), vec2(520.0, 280.0)), 1.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    /// The smallest zoom.
    pub min: f32,
    /// The largest.
    pub max: f32,
    /// The menu's sizes, smallest first.
    pub menu: &'static [f32],
}

impl Range {
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
    /// The user chose `zoom` (the menu or the window's corner): saved with
    /// the project and, when the window closes, in the settings file.
    fn zoom_chosen(&self, zoom: f32);
}

/// Choose `zoom`, held to `range`: the window follows once it takes effect,
/// and the host saves it.
pub fn choose(ctx: &Context, host: &dyn ZoomHost, range: &Range, zoom: f32) {
    let zoom = range.clamp(zoom);
    ctx.set_zoom_factor(zoom);
    host.zoom_chosen(zoom);
}

/// What the panel saw the last pass: the screen's size and the zoom.
#[derive(Clone, Copy, PartialEq)]
struct Seen {
    screen: Vec2,
    zoom: f32,
}

fn seen_id() -> Id {
    Id::unique("panelwerk_zoom_seen")
}

/// Once a pass, before drawing a panel of `base` points: follows a size the
/// host gave the window, and keeps the window at the base size × the zoom.
pub fn follow(ctx: &Context, host: &dyn ZoomHost, base: Vec2, range: &Range) {
    let now = Seen { screen: ctx.content_rect().size(), zoom: ctx.zoom_factor() };
    let before = ctx.data(|d| d.get_temp::<Seen>(seen_id()));
    ctx.data_mut(|d| d.insert_temp(seen_id(), now));
    // The window's first pass only looks: the zoom it opened at has just
    // taken effect, and egui's screen for that pass is a guess.
    let Some(before) = before else { return };
    let off_size = (now.screen - base).abs().max_elem() > 0.5;
    if before.zoom != now.zoom {
        // The new zoom has taken effect: the window to its size.
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(base));
    } else if off_size && before.screen != now.screen {
        // The host resized the window: the zoom for its size.
        let zoom = range.for_size(now.screen * now.zoom, base);
        if zoom == now.zoom {
            // Off the aspect ratio, or past the range: back to the size.
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(base));
        } else {
            choose(ctx, host, range, zoom);
        }
    }
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
/// Rödalog's lists; a click on one chooses it, and any click closes the
/// menu (not the right-click that opened it). Run once a pass, after the
/// controls, so it is drawn over them.
pub fn menu(ctx: &Context, host: &dyn ZoomHost, range: &Range, at: Pos2) {
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
                choose(ctx, host, range, *z);
            }
        }
        ui.allocate_rect(all, Sense::hover());
    });
    if ctx.input(|i| i.pointer.any_released() && !i.pointer.button_released(PointerButton::Secondary)) {
        ctx.data_mut(|d| d.insert_temp(menu_id(), false));
    }
}
