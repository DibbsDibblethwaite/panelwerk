//! How a control is worked (Kör B-PNL-3, Rödalog B-PNL-2): a knob drags up
//! or down from where the button went down, [`DRAG_RANGE`] points for its
//! whole travel, [`FINE`] times finer with Ctrl or Shift, stopping at the
//! ends; a double-click sets its default; a click toggles a switch. Each
//! drag, click and double-click is one host gesture, so one undo step.
//!
//! The host shows a change once the audio thread has had it, a frame or
//! more later, so the panel goes by the value it last set until the host
//! shows it, for at most [`PENDING_FOR`] seconds of egui's input time
//! ([`latest`]).
//!
//! ```
//! use panelwerk_panel::{egui::{Id, Sense, Ui}, gesture, ParamHost};
//!
//! fn mix_knob(ui: &Ui, host: &dyn ParamHost, rect: panelwerk_panel::egui::Rect) {
//!     let response = ui.interact(rect, Id::unique("mix"), Sense::click_and_drag());
//!     gesture::knob(ui, host, &response, "mix");
//! }
//! ```

use egui::{Id, Response, Ui};

use crate::ParamHost;

/// Points of drag for a knob's whole travel.
pub const DRAG_RANGE: f32 = 200.0;
/// How much finer a drag is with Ctrl or Shift held.
pub const FINE: f32 = 10.0;
/// How long, at most, the panel goes by the value it set rather than the
/// host's, in seconds (Kör's, Rödalog's V3a.21).
pub const PENDING_FOR: f64 = 0.5;

/// A knob's gestures on `response` for parameter `param`: the drag from
/// where the button went down, [`DRAG_RANGE`] points for the whole travel
/// ([`FINE`] times finer with Ctrl or Shift), stopping at the ends; a
/// double-click for the default. The drag's own position is kept unrounded,
/// so a stepped parameter moves a step per `DRAG_RANGE / steps` points.
pub fn knob(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str) {
    if response.double_clicked() {
        set_once(ui, host, param, host.default_value(param));
        return;
    }
    let dragged = response.id.with("panelwerk_drag");
    let dy = if response.drag_started() {
        host.begin(param);
        let from = latest(ui, host, param);
        ui.data_mut(|d| d.insert_temp(dragged, from));
        // From where the button went down: the drag starts only once the
        // mouse has moved a little, and that counts too.
        ui.input(|i| i.pointer.press_origin().zip(i.pointer.interact_pos()).map_or(0.0, |(o, p)| p.y - o.y))
    } else {
        response.drag_delta().y
    };
    if response.dragged() && dy != 0.0 {
        let fine = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift);
        let range = if fine { DRAG_RANGE * FINE } else { DRAG_RANGE };
        let from = ui.data(|d| d.get_temp::<f32>(dragged)).unwrap_or_default();
        let to = (from - dy / range).clamp(0.0, 1.0);
        ui.data_mut(|d| d.insert_temp(dragged, to));
        set(ui, host, param, to);
    }
    if response.drag_stopped() {
        ui.data_mut(|d| d.remove::<f32>(dragged));
        host.end(param);
    }
}

/// A click on `response` toggles `param` (an Off/On or two-way parameter),
/// from its [`latest`] value, as one gesture.
pub fn toggle_on_click(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str) {
    if response.clicked() {
        let on = latest(ui, host, param) >= 0.5;
        set_once(ui, host, param, if on { 0.0 } else { 1.0 });
    }
}

/// Sets `param` within a gesture, remembering the value until the host
/// shows it.
pub fn set(ui: &Ui, host: &dyn ParamHost, param: &str, value: f32) {
    host.set(param, value);
    let now = ui.input(|i| i.time);
    ui.data_mut(|d| d.insert_temp(pending_id(param), (value, now)));
}

/// One change, one gesture: begin, [`set`], end.
pub fn set_once(ui: &Ui, host: &dyn ParamHost, param: &str, value: f32) {
    host.begin(param);
    set(ui, host, param, value);
    host.end(param);
}

/// `param`'s value as the panel last set it, while the host isn't showing
/// it yet and for at most [`PENDING_FOR`], else the host's: what a click
/// toggles from and a drag starts from, so two quick clicks while the host
/// lags turn a switch on and off again.
pub fn latest(ui: &Ui, host: &dyn ParamHost, param: &str) -> f32 {
    let shown = host.value(param);
    let now = ui.input(|i| i.time);
    match ui.data(|d| d.get_temp::<(f32, f64)>(pending_id(param))) {
        Some((value, at)) if value != shown && now - at < PENDING_FOR => value,
        _ => shown,
    }
}

fn pending_id(param: &str) -> Id {
    Id::unique(("panelwerk_pending", param))
}
