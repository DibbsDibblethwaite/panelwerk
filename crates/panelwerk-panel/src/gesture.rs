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
///
/// A drag's gesture is ended for the host when egui stops it, even if this
/// knob is no longer drawn then (a preset, automation or a mode change hid
/// it mid-drag): the next call of `knob` or [`toggle_on_click`] for any
/// control in the window ends it, through that call's `host` (W-3).
///
/// **Every control in a window must be given the same host**: a lost
/// drag's end goes through the host of whichever control is drawn next, so
/// with two hosts one could get the drag's `begin` and sets and the other
/// its `end`.
///
/// A drag still open when the window closes, or when nothing these
/// functions work is drawn again, isn't ended here: the record of open
/// drags is private. With held edits, [`crate::hold::Held::close`] or
/// [`crate::hold::Held::take_open`] ends it; without them, only a consumer
/// whose own `ParamHost` tracks the begins it hasn't seen ended can end it
/// at the close.
pub fn knob(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str) {
    knob_drag(ui, host, response, param);
    end_lost_drags(ui, host);
}

fn knob_drag(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str) {
    if response.double_clicked() {
        set_once(ui, host, param, host.default_value(param));
        return;
    }
    let dragged = response.id.with("panelwerk_drag");
    let dy = if response.drag_started() {
        host.begin(param);
        ui.data_mut(|d| {
            d.get_temp_mut_or_default::<Vec<(Id, String)>>(open_drags_id()).push((response.id, param.to_owned()))
        });
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
    // The knob's own end. With `end_lost_drags` after it in `knob`, which
    // finds this widget no longer dragged in the same call, deleting this
    // branch would change nothing a test sees; it is kept so the knob ends
    // its own drag, not by way of the guard for lost ones.
    if response.drag_stopped() {
        end_drag(ui, host, response.id);
    }
}

/// Ends, for `host`, the knob drag begun on widget `widget`, if it is still
/// open: once, whether the knob's own `drag_stopped` or a later call's
/// [`end_lost_drags`] comes to it first.
fn end_drag(ui: &Ui, host: &dyn ParamHost, widget: Id) {
    let open = ui.data_mut(|d| {
        d.remove::<f32>(widget.with("panelwerk_drag"));
        let drags = d.get_temp_mut_or_default::<Vec<(Id, String)>>(open_drags_id());
        drags.iter().position(|(w, _)| *w == widget).map(|at| drags.remove(at).1)
    });
    if let Some(param) = open {
        host.end(&param);
    }
}

/// Ends every knob drag still open whose widget egui no longer drags: one
/// whose knob wasn't drawn when the button went up, so it never saw its own
/// `drag_stopped` (W-3). Without this the host's gesture stays open, and a
/// host in Touch or Latch keeps writing automation.
fn end_lost_drags(ui: &Ui, host: &dyn ParamHost) {
    // Copied out first: asking egui what it drags inside `data` would take
    // the context's lock twice.
    let open = ui.data(|d| d.get_temp::<Vec<(Id, String)>>(open_drags_id())).unwrap_or_default();
    for (widget, _) in open {
        if !ui.ctx().is_being_dragged(widget) {
            end_drag(ui, host, widget);
        }
    }
}

/// Where the window's open knob drags are kept: each widget's egui ID with
/// its parameter, from the drag's begin to its end.
fn open_drags_id() -> Id {
    Id::unique("panelwerk_open_drags")
}

/// A click on `response` toggles `param` (an Off/On or two-way parameter),
/// from its [`latest`] value, as one gesture.
///
/// Like [`knob`], it ends a knob drag lost when its knob was hidden,
/// through this call's `host`: every control in a window must be given the
/// same host ([`knob`] says why).
pub fn toggle_on_click(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str) {
    end_lost_drags(ui, host);
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
