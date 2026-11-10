//! Buttons: an outlined body, filled while under the mouse; a button with a
//! light and its name (Kör's mode buttons); an Off/On switch in a knob's
//! slot.
//!
//! ```
//! use panelwerk_panel::{button, egui::{self, pos2, vec2, Rect}};
//!
//! let ctx = egui::Context::default();
//! let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
//!     button::body(ui.painter(), Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 20.0)), false);
//! });
//! out.textures_delta.clear(); // egui's check that a frame's textures were taken
//! ```

use egui::{Align2, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::light::{self, Light};
use crate::palette::{HOVER, TEXT, TRACK};
use crate::{colours, font, gesture, Control, ParamHost};

/// A button's corners' radius.
const ROUNDING: f32 = 4.0;

/// A button's body in `rect`: outlined, filled while `hot`.
pub fn body(painter: &Painter, rect: Rect, hot: bool) {
    if hot {
        painter.rect_filled(rect, ROUNDING, HOVER);
    }
    painter.rect_stroke(rect, ROUNDING, Stroke::new(1.0, TRACK), StrokeKind::Inside);
}

/// A button with a light at its left and its name beside it, under egui ID
/// `id`: drawn here, its click the caller's.
pub fn lit(ui: &Ui, rect: Rect, id: Id, label: &str, light: Light, geometry: &light::Geometry) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    body(ui.painter(), rect, response.hovered());
    let at = Pos2::new(rect.left() + geometry.inset, rect.center().y);
    light::paint(ui.painter(), at, geometry.radius, light);
    let text_at = Pos2::new(at.x + geometry.radius + light::TEXT_GAP, at.y);
    ui.painter().text(text_at, Align2::LEFT_CENTER, label, font(), TEXT);
    response
}

/// An Off/On switch in its rectangle: its name at the top, a button `size`
/// in the middle with a light of `light_radius`, its value text at the
/// bottom; a click toggles it, one gesture.
pub fn switch(ui: &Ui, host: &dyn ParamHost, control: &Control, size: Vec2, light_radius: f32) -> Response {
    let response = ui.interact(control.rect, control.id, Sense::click());
    gesture::toggle_on_click(ui, host, &response, control.param);
    let (text, _) = colours(control.dimmed);
    let p = ui.painter();
    let rect = control.rect;
    p.text(rect.center_top(), Align2::CENTER_TOP, control.label, font(), text);
    let button = Rect::from_center_size(rect.center(), size);
    body(p, button, response.hovered());
    let on = host.value(control.param) >= 0.5;
    light::paint(p, button.center(), light_radius, Light::new(on, control.dimmed));
    p.text(rect.center_bottom(), Align2::CENTER_BOTTOM, host.text(control.param), font(), text);
    response
}
