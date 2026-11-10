//! The family's colours: Rödalog's palette under role names (Kör's
//! B-PNL-9 took it value for value, Ian, 2026-11-10), and WCAG 2's contrast
//! measure with the pairs every panel draws text in.
//!
//! No amber here: in Rödalog amber means Morph, which is Rödalog's own
//! colour and stays in Rödalog. Nothing of the original hardware's
//! (`CLAUDE.md` rule 10).
//!
//! ```
//! use panelwerk_panel::palette::{contrast, unreadable, TEXT, SECTION, TEXT_ON};
//!
//! assert!(contrast(TEXT, SECTION) > 4.5);
//! assert!(unreadable(TEXT_ON).is_empty());
//! ```

use egui::Color32;

/// Around the sections, and the help line: Rödalog's `BACKGROUND` (Kör's
/// `HELP_BACKGROUND`).
pub const PANEL: Color32 = Color32::from_rgb(18, 18, 20);
/// Where controls sit, and a list or menu's body: Rödalog's `SECTION`
/// (Kör's `BACKGROUND` and `MENU`).
pub const SECTION: Color32 = Color32::from_rgb(38, 38, 43);
/// A section's or a menu's outline: Rödalog's `BORDER` (Kör's
/// `MENU_BORDER`).
pub const BORDER: Color32 = Color32::from_rgb(58, 58, 64);
/// Behind a button, switch or menu row under the mouse.
pub const HOVER: Color32 = Color32::from_rgb(48, 48, 54);
/// Text: names, values and hints.
pub const TEXT: Color32 = Color32::from_rgb(220, 220, 224);
/// The text of a control that does nothing at the moment (dimmed), and a
/// dimmed knob's pointer: Rödalog's `DIMMED`.
pub const DIMMED_TEXT: Color32 = Color32::from_rgb(155, 155, 160);
/// A list's or menu's current choice, and a lit knob's pointer.
pub const CHOSEN: Color32 = Color32::WHITE;
/// A list's or menu's other choices.
pub const CHOICE_TEXT: Color32 = Color32::from_rgb(170, 170, 176);
/// Value arcs, and a lit knob's rim under the mouse.
pub const RED: Color32 = Color32::from_rgb(226, 56, 54);
/// A value shown on the panel's dark: the help line's value.
pub const BRIGHT_RED: Color32 = Color32::from_rgb(240, 90, 90);
/// A dimmed control's value arc.
pub const DIMMED_VALUE: Color32 = Color32::from_rgb(120, 120, 128);
/// A lit light.
pub const LIT: Color32 = Color32::from_rgb(255, 80, 70);
/// A lit light's ring.
pub const LIT_RING: Color32 = Color32::from_rgb(255, 160, 150);
/// A dimmed control's lit light.
pub const DIMMED_LIT: Color32 = Color32::from_rgb(170, 170, 176);
/// A light that is off: Rödalog's `OFF`.
pub const LIGHT_OFF: Color32 = Color32::from_rgb(100, 100, 106);
/// A knob's travel behind its value arc, a button's outline and a rule
/// between rows: Rödalog's `DIM` (Kör's `OUTLINE` too).
pub const TRACK: Color32 = Color32::from_rgb(84, 84, 92);
/// A knob's body.
pub const KNOB: Color32 = Color32::from_rgb(58, 58, 64);
/// A knob's rim.
pub const RIM: Color32 = Color32::from_rgb(120, 120, 128);
/// A title strip: Rödalog's section titles, Kör's name.
pub const TITLE_STRIP: Color32 = Color32::from_rgb(200, 40, 42);
/// A title strip's text.
pub const TITLE_TEXT: Color32 = Color32::WHITE;

/// The least contrast for any text: 4.5:1, WCAG 2's AA level for normal
/// text (Kör B-PNL-9, Rödalog B-PNL-4).
pub const MIN_CONTRAST: f32 = 4.5;

/// Every text colour the shared widgets draw, on each background they draw
/// it on, for each consumer's contrast test: a consumer adds its own pairs
/// (Rödalog's amber) and checks them with [`unreadable`].
pub const TEXT_ON: &[(Color32, Color32)] = &[
    (TEXT, SECTION),
    (TEXT, HOVER),
    (DIMMED_TEXT, SECTION),
    (DIMMED_TEXT, HOVER),
    (TITLE_TEXT, TITLE_STRIP),
    (TEXT, PANEL),
    (BRIGHT_RED, PANEL),
    (CHOSEN, SECTION),
    (CHOSEN, HOVER),
    (CHOICE_TEXT, SECTION),
    (CHOICE_TEXT, HOVER),
];

/// WCAG 2's contrast ratio between two colours, 1 … 21, either way round.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The pairs (text, background) under [`MIN_CONTRAST`], each with its
/// ratio; none for a readable palette.
pub fn unreadable(pairs: &[(Color32, Color32)]) -> Vec<(Color32, Color32, f32)> {
    pairs.iter().map(|&(t, b)| (t, b, contrast(t, b))).filter(|p| p.2 < MIN_CONTRAST).collect()
}

/// WCAG 2's relative luminance of an sRGB colour.
fn luminance(c: Color32) -> f32 {
    let linear = |v: u8| {
        let s = f32::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}
