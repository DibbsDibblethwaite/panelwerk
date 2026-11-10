//! Panelwerk's panel layer: the look and the controls Ian Dilley's plugins
//! (Rödalog, Kör, Fläns) share, drawn with egui 0.37.0.
//!
//! - [`palette`]: the family's colours and the contrast test.
//! - [`gesture`]: how knobs and switches are worked: drag, fine drag,
//!   double-click, click, one host gesture each.
//! - [`knob`]: the knob, painted and as a whole control.
//! - [`light`] and [`button`]: lights, button bodies, lit buttons, switches.
//!
//! The panel reaches the plugin only through [`ParamHost`], which the
//! consumer implements; it never names a consumer's parameter or feature
//! (`CLAUDE.md` rule 1). No `unsafe`, and nothing here runs on the audio
//! thread (rule 3).
//!
//! ```
//! use panelwerk_panel::{colours, palette};
//!
//! assert_eq!(colours(false), (palette::TEXT, palette::RED));
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The egui the panel is drawn with: a consumer's window takes this one.
pub use egui;

use egui::{Color32, FontId, Id, Rect, Response};

pub mod button;
pub mod gesture;
pub mod knob;
pub mod light;
pub mod palette;

/// What every panel needs from its plugin, by parameter string ID. Values
/// are normalised, 0 … 1, as hosts see them. A consumer implements it on
/// its plugin's view (and on its tests' fake host); its own host trait can
/// add calls on top (`trait Host: ParamHost`), and every shared widget takes
/// `&dyn ParamHost`.
///
/// ```
/// use std::cell::Cell;
/// use panelwerk_panel::ParamHost;
///
/// /// One parameter, "mix".
/// struct One(Cell<f32>);
/// impl ParamHost for One {
///     fn value(&self, _: &str) -> f32 { self.0.get() }
///     fn default_value(&self, _: &str) -> f32 { 0.5 }
///     fn text(&self, _: &str) -> String { format!("{:.1} %", 100.0 * self.0.get()) }
///     fn begin(&self, _: &str) {}
///     fn set(&self, _: &str, normalized: f32) { self.0.set(normalized) }
///     fn end(&self, _: &str) {}
/// }
/// let host = One(Cell::new(0.25));
/// host.set("mix", host.default_value("mix"));
/// assert_eq!(host.text("mix"), "50.0 %");
/// ```
pub trait ParamHost {
    /// Parameter `id`'s value.
    fn value(&self, id: &str) -> f32;
    /// Parameter `id`'s default.
    fn default_value(&self, id: &str) -> f32;
    /// Parameter `id`'s value text with its unit, as the host shows it.
    fn text(&self, id: &str) -> String;
    /// Start one change: a drag, a click or a double-click is one host
    /// gesture, so one undo step.
    fn begin(&self, id: &str);
    /// Set parameter `id` within the gesture.
    fn set(&self, id: &str, normalized: f32);
    /// End the gesture.
    fn end(&self, id: &str);
}

/// One control on a consumer's panel: where it is (the consumer's layout),
/// its egui ID (so its tests can find it), the parameter it sets, its name,
/// and whether it is drawn dimmed while it does nothing (it still works).
///
/// ```
/// use panelwerk_panel::{egui::{pos2, vec2, Id, Rect}, Control};
///
/// let mix = Control {
///     rect: Rect::from_min_size(pos2(142.0, 44.0), vec2(72.0, 72.0)),
///     id: Id::unique("mix"),
///     param: "mix",
///     label: "Mix",
///     dimmed: false,
/// };
/// assert_eq!(mix.label, "Mix");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Control<'a> {
    /// Where it is drawn, in points at the panel's base size.
    pub rect: Rect,
    /// Its egui ID.
    pub id: Id,
    /// The parameter it sets, by string ID.
    pub param: &'a str,
    /// Its name on the panel.
    pub label: &'a str,
    /// Drawn in the dimmed colours.
    pub dimmed: bool,
}

/// The family's text size in points (Kör B-PNL-9, Rödalog B-PNL-4).
pub const TEXT_SIZE: f32 = 13.0;

/// The family's text font: proportional, [`TEXT_SIZE`] points.
pub fn font() -> FontId {
    FontId::proportional(TEXT_SIZE)
}

/// A control's text and value colours: lit, or dimmed while it does nothing
/// (Kör B-PNL-5, Rödalog B-PNL-1).
pub fn colours(dimmed: bool) -> (Color32, Color32) {
    if dimmed {
        (palette::DIMMED_TEXT, palette::DIMMED_VALUE)
    } else {
        (palette::TEXT, palette::RED)
    }
}

/// Whether a control is under the mouse or being dragged (dragged off it,
/// it stays hot).
pub fn hot(response: &Response) -> bool {
    response.hovered() || response.dragged()
}
