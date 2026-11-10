//! Panelwerk's panel layer: the look and the controls Ian Dilley's plugins
//! (Rödalog, Kör, Fläns) share, drawn with egui 0.37.0.
//!
//! - [`palette`]: the family's colours and the contrast test.
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

use egui::{Color32, FontId};

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
