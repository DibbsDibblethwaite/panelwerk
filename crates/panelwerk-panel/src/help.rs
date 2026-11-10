//! The help line along a panel's bottom (Kör B-PNL-8; PW-14: the line
//! shared, the hints each consumer's): the hovered control's hint at the
//! left, and a value at the right in bright red, the hint clipped before
//! the value rather than run under it.
//!
//! ```
//! use panelwerk_panel::{egui::{self, pos2, Rect}, help};
//!
//! let line = Rect::from_min_max(pos2(0.0, 254.0), pos2(520.0, 280.0));
//! assert_eq!(line.height(), help::HEIGHT);
//! let ctx = egui::Context::default();
//! let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
//!     help::line(ui.painter(), line, Some("How loud the hiss is."), Some("-60.0 dBFS"));
//! });
//! out.textures_delta.clear(); // egui's check that a frame's textures were taken
//! ```

use egui::{Align2, Painter, Pos2, Rect};

use crate::font;
use crate::palette::{BRIGHT_RED, PANEL, TEXT};

/// The line's height in points.
pub const HEIGHT: f32 = 26.0;
/// Its text's margin at each end.
pub const MARGIN: f32 = 12.0;
/// Between a hint and the value at the right.
pub const VALUE_GAP: f32 = 12.0;

/// The help line in `rect`, on the panel's dark: `hint` at the left (a
/// hovered control's, or a message), `value` at the right in bright red;
/// nothing but the dark if neither.
pub fn line(painter: &Painter, rect: Rect, hint: Option<&str>, value: Option<&str>) {
    painter.rect_filled(rect, 0.0, PANEL);
    let left = Pos2::new(rect.left() + MARGIN, rect.center().y);
    let right = rect.right() - MARGIN;
    let value = value.map(|v| painter.layout_no_wrap(v.to_owned(), font(), BRIGHT_RED));
    let end = value.as_ref().map_or(right, |v| right - v.size().x - VALUE_GAP);
    if let Some(hint) = hint {
        let clip = Rect::from_x_y_ranges(rect.left()..=end, rect.y_range());
        painter.with_clip_rect(clip).text(left, Align2::LEFT_CENTER, hint, font(), TEXT);
    }
    if let Some(v) = value {
        let at = Pos2::new(right - v.size().x, left.y - v.size().y / 2.0);
        painter.galley(at, v, BRIGHT_RED);
    }
}
