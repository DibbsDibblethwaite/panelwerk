//! The palette (Kör B-PNL-9, Rödalog B-PNL-4): Rödalog's colours under role
//! names, every value written out so a change fails here first (rule 6), and
//! every text colour readable on each background it is drawn on.

use egui::Color32;
use panelwerk_panel::palette::*;

#[test]
fn rule_6_the_family_colours_are_rodalogs_value_for_value() {
    let rgb = Color32::from_rgb;
    let want = [
        ("PANEL", PANEL, rgb(18, 18, 20)),
        ("SECTION", SECTION, rgb(38, 38, 43)),
        ("BORDER", BORDER, rgb(58, 58, 64)),
        ("HOVER", HOVER, rgb(48, 48, 54)),
        ("TEXT", TEXT, rgb(220, 220, 224)),
        ("DIMMED_TEXT", DIMMED_TEXT, rgb(155, 155, 160)),
        ("CHOSEN", CHOSEN, Color32::WHITE),
        ("CHOICE_TEXT", CHOICE_TEXT, rgb(170, 170, 176)),
        ("RED", RED, rgb(226, 56, 54)),
        ("BRIGHT_RED", BRIGHT_RED, rgb(240, 90, 90)),
        ("DIMMED_VALUE", DIMMED_VALUE, rgb(120, 120, 128)),
        ("LIT", LIT, rgb(255, 80, 70)),
        ("LIT_RING", LIT_RING, rgb(255, 160, 150)),
        ("DIMMED_LIT", DIMMED_LIT, rgb(170, 170, 176)),
        ("LIGHT_OFF", LIGHT_OFF, rgb(100, 100, 106)),
        ("TRACK", TRACK, rgb(84, 84, 92)),
        ("KNOB", KNOB, rgb(58, 58, 64)),
        ("RIM", RIM, rgb(120, 120, 128)),
        ("TITLE_STRIP", TITLE_STRIP, rgb(200, 40, 42)),
        ("TITLE_TEXT", TITLE_TEXT, Color32::WHITE),
    ];
    for (name, got, want) in want {
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn b_pnl_9_every_family_text_pair_is_at_least_4_5_to_1() {
    assert_eq!(TEXT_ON.len(), 11);
    for (text, background) in TEXT_ON {
        // Kör B-PNL-9, Rödalog B-PNL-4: 4.5:1, WCAG 2 AA for normal text.
        // The closest, reported to Ian (rule 8): DIMMED_TEXT on HOVER, 4.74:1
        // (Kör's and Rödalog's own tests have passed it since 2026-11-10).
        assert!(contrast(*text, *background) >= MIN_CONTRAST, "{text:?} on {background:?}");
    }
    assert!(unreadable(TEXT_ON).is_empty());
    assert_eq!(MIN_CONTRAST, 4.5);
}

#[test]
fn b_pnl_9_text_on_names_each_text_colour_on_its_backgrounds() {
    let want = [
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
    assert_eq!(TEXT_ON, want);
}

#[test]
fn contrast_is_wcag_2s_ratio() {
    // WCAG 2's own figures: black on white 21:1, a colour on itself 1:1,
    // either way round.
    assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 1e-3);
    assert!((contrast(Color32::WHITE, Color32::BLACK) - 21.0).abs() < 1e-3);
    let c = Color32::from_rgb(1, 2, 3);
    assert!((contrast(c, c) - 1.0).abs() < 1e-6);
    // Mid grey (119) on white is WCAG's often-quoted 4.48:1, just under AA.
    let grey = Color32::from_rgb(119, 119, 119);
    assert!((contrast(grey, Color32::WHITE) - 4.48).abs() < 0.01, "{}", contrast(grey, Color32::WHITE));
}

#[test]
fn unreadable_lists_the_pairs_under_4_5_with_their_ratios() {
    let grey = Color32::from_rgb(119, 119, 119);
    let pairs = [(Color32::BLACK, Color32::WHITE), (grey, Color32::WHITE)];
    let found = unreadable(&pairs);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].0, found[0].1), (grey, Color32::WHITE));
    assert!((found[0].2 - 4.48).abs() < 0.01);
}

#[test]
fn b_pnl_9_text_is_13_points() {
    // Kör B-PNL-9, Rödalog B-PNL-4.
    assert_eq!(panelwerk_panel::TEXT_SIZE, 13.0);
    assert_eq!(panelwerk_panel::font(), egui::FontId::proportional(13.0));
}

#[test]
fn colours_are_lit_or_dimmed() {
    // Kör B-PNL-5, Rödalog B-PNL-1: dimmed while the control does nothing.
    assert_eq!(panelwerk_panel::colours(false), (TEXT, RED));
    assert_eq!(panelwerk_panel::colours(true), (DIMMED_TEXT, DIMMED_VALUE));
}
