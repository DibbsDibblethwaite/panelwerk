//! Lights (Rödalog's LEDs): lit red with a lighter ring, lit grey while
//! the control is dimmed, or off.
//!
//! ```
//! use panelwerk_panel::light::Light;
//!
//! assert_eq!(Light::new(true, true), Light::DimmedLit);
//! ```

use egui::{Painter, Pos2, Stroke};

use crate::palette::{DIMMED_LIT, LIGHT_OFF, LIT, LIT_RING};

/// What a light shows.
///
/// ```
/// use panelwerk_panel::light::Light;
///
/// assert_eq!(Light::new(false, true), Light::Off);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    /// Off.
    Off,
    /// Lit red, ringed.
    Lit,
    /// Lit, on a dimmed control: grey.
    DimmedLit,
}

impl Light {
    /// The light of a control that is `lit`, drawn `dimmed` or not.
    pub fn new(lit: bool, dimmed: bool) -> Self {
        match (lit, dimmed) {
            (false, _) => Light::Off,
            (true, false) => Light::Lit,
            (true, true) => Light::DimmedLit,
        }
    }
}

/// A lit light's ring's width.
pub const RING_WIDTH: f32 = 1.5;
/// Between a light's edge and the name beside it.
pub const TEXT_GAP: f32 = 6.0;

/// Where a button's light goes, each consumer's: its radius (Kör's 5,
/// Rödalog's 5.5) and its centre's distance from the button's left edge.
///
/// ```
/// let koer = panelwerk_panel::light::Geometry { radius: 5.0, inset: 12.0 };
/// assert!(koer.inset > koer.radius);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// The light's radius.
    pub radius: f32,
    /// Its centre from the button's left edge.
    pub inset: f32,
}

/// Paints `light` at `at`.
pub fn paint(painter: &Painter, at: Pos2, radius: f32, light: Light) {
    match light {
        Light::Lit => painter.circle(at, radius, LIT, Stroke::new(RING_WIDTH, LIT_RING)),
        Light::DimmedLit => painter.circle_filled(at, radius, DIMMED_LIT),
        Light::Off => painter.circle_filled(at, radius, LIGHT_OFF),
    };
}
