//! The title strip: white text on a red strip filling its rectangle, as
//! Rödalog's section titles and Kör's name (Kör B-PNL-9).
//!
//! ```
//! use panelwerk_panel::{egui::{self, pos2, vec2, Id, Rect}, title, zoom};
//!
//! fn name(ui: &egui::Ui) {
//!     let rect = Rect::from_min_size(pos2(12.0, 4.0), vec2(44.0, 24.0));
//!     // The consumer decides what a click does: Kör's opens the zoom menu.
//!     if title::strip(ui, rect, Id::unique("name"), "Kör", 15.0).secondary_clicked() {
//!         zoom::open_menu(ui.ctx());
//!     }
//! }
//! ```

use egui::{Align2, FontId, Id, Rect, Response, Sense, Ui};

use crate::palette::{TITLE_STRIP, TITLE_TEXT};

/// The strip's corners' radius.
const ROUNDING: f32 = 4.0;

/// A title strip filling `rect` under egui ID `id`, `text` centred on it in
/// `size` points (Kör's name 15; egui's fonts have no bold). Its clicks are
/// the caller's.
pub fn strip(ui: &Ui, rect: Rect, id: Id, text: &str, size: f32) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, ROUNDING, TITLE_STRIP);
    p.text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(size), TITLE_TEXT);
    response
}
