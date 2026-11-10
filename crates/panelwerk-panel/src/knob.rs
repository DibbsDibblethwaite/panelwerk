//! The family's knob: a travel of 270° from 7:30 clockwise to 4:30, the
//! value arc over it, the body with its rim, and the pointer. Kör's look,
//! with Rödalog's details (PW-6): the pointer white, grey while dimmed; the
//! rim red under the mouse, in the text colour while dimmed. The sizes are
//! each consumer's ([`Geometry`]), and so is where a knob goes.
//!
//! ```
//! use panelwerk_panel::knob::{self, Geometry, Look};
//! use panelwerk_panel::egui::{self, pos2};
//!
//! const SMALL: Geometry = Geometry { radius: 8.5, arc_radius: 11.05, arc_width: 2.0, pointer: (2.55, 6.5), pointer_width: 2.0 };
//! let ctx = egui::Context::default();
//! let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
//!     knob::paint(ui.painter(), pos2(40.0, 40.0), &SMALL, &Look { value: 0.3, ..Look::default() });
//! });
//! out.textures_delta.clear(); // egui's check that a frame's textures were taken
//! ```

use std::f32::consts::PI;

use egui::{Align2, Painter, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};

use crate::palette::{CHOSEN, DIMMED_TEXT, KNOB, RED, RIM, TEXT, TRACK};
use crate::{colours, font, gesture, hot, Control, ParamHost};

/// A knob's sizes, each consumer's own (Kör's knob has radius 15, Rödalog's
/// 16), in points.
///
/// ```
/// use panelwerk_panel::knob::Geometry;
///
/// // Kör's main knob: its arc 30 % outside the body, the pointer from 30 %
/// // of the radius to 2 points short of the rim.
/// let koer = Geometry { radius: 15.0, arc_radius: 19.5, arc_width: 2.7, pointer: (4.5, 13.0), pointer_width: 2.0 };
/// assert!(koer.arc_radius > koer.radius);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// The body's radius.
    pub radius: f32,
    /// The radius of the travel and the value arc.
    pub arc_radius: f32,
    /// Their width.
    pub arc_width: f32,
    /// Where the pointer starts and ends, out from the centre.
    pub pointer: (f32, f32),
    /// The pointer's width.
    pub pointer_width: f32,
}

/// How a knob looks this frame. Build it with `..Look::default()`, so the
/// options later versions add (each off by default) leave it as it is.
///
/// ```
/// use panelwerk_panel::knob::Look;
///
/// let look = Look { value: 0.75, hot: true, ..Look::default() };
/// assert!(!look.dimmed);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    /// The value, 0 … 1 of the travel: the value arc's end and the
    /// pointer's angle.
    pub value: f32,
    /// Under the mouse or being dragged: the rim lit.
    pub hot: bool,
    /// Doing nothing at the moment: drawn in the dimmed colours.
    pub dimmed: bool,
}

/// The rim's width.
pub const RIM_WIDTH: f32 = 1.5;

/// 7:30, the travel's start, in egui's angles (y down, so clockwise on
/// screen), and the travel, 270°.
const MIN_ANGLE: f32 = 0.75 * PI;
const TRAVEL: f32 = 1.5 * PI;

/// The angle of `position` (0 … 1, held to it) along the travel: 7:30 at 0,
/// 12 o'clock at 0.5, 4:30 at 1, in egui's angles.
pub fn angle(position: f32) -> f32 {
    MIN_ANGLE + TRAVEL * position.clamp(0.0, 1.0)
}

/// Points along an arc around `c`, about one every 4°.
fn arc(c: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    let n = ((to - from).abs() / 0.07).ceil().max(1.0) as usize;
    (0..=n).map(|k| c + Vec2::angled(from + (to - from) * k as f32 / n as f32) * radius).collect()
}

/// Paints a knob at `centre`: its travel, the value arc (none at the
/// minimum), the body and rim, and the pointer.
pub fn paint(painter: &Painter, centre: Pos2, geometry: &Geometry, look: &Look) {
    let (_, accent) = colours(look.dimmed);
    let g = geometry;
    painter.add(Shape::line(arc(centre, g.arc_radius, MIN_ANGLE, MIN_ANGLE + TRAVEL), Stroke::new(g.arc_width, TRACK)));
    let to = angle(look.value);
    if to - MIN_ANGLE > 1e-3 {
        painter.add(Shape::line(arc(centre, g.arc_radius, MIN_ANGLE, to), Stroke::new(g.arc_width, accent)));
    }
    let rim = match (look.hot, look.dimmed) {
        (false, _) => RIM,
        (true, false) => RED,
        (true, true) => TEXT,
    };
    painter.circle(centre, g.radius, KNOB, Stroke::new(RIM_WIDTH, rim));
    let pointer = Vec2::angled(to);
    let tip = if look.dimmed { DIMMED_TEXT } else { CHOSEN };
    painter.line_segment(
        [centre + pointer * g.pointer.0, centre + pointer * g.pointer.1],
        Stroke::new(g.pointer_width, tip),
    );
}

/// The control's response, its gestures run, and its knob painted at
/// `centre`.
fn worked(ui: &Ui, host: &dyn ParamHost, control: &Control, centre: Pos2, geometry: &Geometry) -> Response {
    let response = ui.interact(control.rect, control.id, Sense::click_and_drag());
    gesture::knob(ui, host, &response, control.param);
    let look = Look { value: host.value(control.param), hot: hot(&response), dimmed: control.dimmed };
    paint(ui.painter(), centre, geometry, &look);
    response
}

/// A whole knob control in its rectangle: its name at the top, the knob at
/// `centre`, its value text (the host's) at the bottom; dragged and
/// double-clicked as [`gesture::knob`] says.
pub fn knob(ui: &Ui, host: &dyn ParamHost, control: &Control, centre: Pos2, geometry: &Geometry) -> Response {
    let response = worked(ui, host, control, centre, geometry);
    let (text, _) = colours(control.dimmed);
    let p = ui.painter();
    p.text(control.rect.center_top(), Align2::CENTER_TOP, control.label, font(), text);
    p.text(control.rect.center_bottom(), Align2::CENTER_BOTTOM, host.text(control.param), font(), text);
    response
}

/// A small knob on one row: its name at the rectangle's left, the knob at
/// `centre`, no value text (Kör's title row shows it in the help line).
pub fn inline(ui: &Ui, host: &dyn ParamHost, control: &Control, centre: Pos2, geometry: &Geometry) -> Response {
    let response = worked(ui, host, control, centre, geometry);
    let (text, _) = colours(control.dimmed);
    let at = Pos2::new(control.rect.left(), control.rect.center().y);
    ui.painter().text(at, Align2::LEFT_CENTER, control.label, font(), text);
    response
}
