//! Value text, the family's one format (PW-1, Ian, 2026-11-10), frozen
//! (CLAUDE.md rule 6): every kind at a table of values, the edges included
//! (negative zero, rounding up into the next format, the floor), and every
//! text read back to a value that shows the same.
//!
//! The units: every kind but frequency returns the number alone, and the
//! consumer's parameter adds its unit (nice-plug's `with_unit`, a space
//! first: " %", " dB"); frequencies write their own, "440 Hz", "1.20 kHz"
//! (PW-1a, Ian, 2026-11-10). `shown` below is what a host shows.

use panelwerk_text::value::{self, Sign::*};

/// What a host shows for a number text and its framework unit.
fn shown(text: String, unit: &str) -> String {
    format!("{text}{unit}")
}

#[test]
fn fixed_rounds_half_away_from_zero_with_no_negative_zero() {
    let table = [
        (0.0, 1, Unsigned, "0.0"),
        (-0.0, 1, Unsigned, "0.0"),
        (-0.04, 1, Unsigned, "0.0"),
        (-0.04, 1, Signed, "0.0"),
        (0.04, 1, Signed, "0.0"),
        (0.05, 1, Signed, "+0.1"),
        (-0.05, 1, Signed, "-0.1"),
        (12.25, 1, Unsigned, "12.3"),
        (-12.25, 1, Unsigned, "-12.3"),
        (3.5, 0, Unsigned, "4"),
        (180.0, 0, Unsigned, "180"),
        (3.505, 3, Unsigned, "3.505"),
        (7.0, 2, Signed, "+7.00"),
    ];
    for (x, places, sign, text) in table {
        assert_eq!(value::fixed(x, places, sign), text, "{x} to {places}");
    }
}

#[test]
fn significant_chooses_its_places_by_the_rounded_value() {
    let table = [
        (0.03, 3, "0.0300"),
        (0.045, 3, "0.0450"),
        (0.0999, 3, "0.0999"),
        (0.09996, 3, "0.100"),
        (0.513, 3, "0.513"),
        (0.9996, 3, "1.00"),
        (1.234, 3, "1.23"),
        (9.996, 3, "10.0"),
        (11.7, 3, "11.7"),
        (99.96, 3, "100"),
        (632.4, 3, "632"),
        (999.5, 3, "1000"),
        (1234.0, 3, "1230"),
        (20_000.0, 3, "20000"),
        (-1.234, 3, "-1.23"),
        (8.0, 2, "8.0"),
        (40.0, 2, "40"),
        (0.5, 2, "0.50"),
        (0.0, 3, "0.00"),
        (-0.0, 3, "0.00"),
        // No figures is taken as one.
        (4.4, 0, "4"),
        (f32::INFINITY, 3, "inf"),
    ];
    for (x, figures, text) in table {
        assert_eq!(value::significant(x, figures), text, "{x} to {figures} figures");
    }
}

/// PW-1: percentages to one decimal, signed or not by parameter.
#[test]
fn pw_1_percentages_to_one_decimal() {
    let table = [
        (0.0, Unsigned, "0.0"),
        (0.5, Unsigned, "50.0"),
        (1.0, Unsigned, "100.0"),
        (0.12345, Unsigned, "12.3"),
        (0.9996, Unsigned, "100.0"),
        (0.125, Signed, "+12.5"),
        (-0.03, Signed, "-3.0"),
        (-0.0004, Signed, "0.0"),
        (0.0, Signed, "0.0"),
        (-1.0, Signed, "-100.0"),
    ];
    for (fraction, sign, text) in table {
        assert_eq!(value::percent(fraction, sign), text, "{fraction}");
    }
    assert_eq!(shown(value::percent(0.5, Unsigned), " %"), "50.0 %");
}

#[test]
fn percentages_read_back_as_fractions() {
    for text in ["50.0 %", "50.0%", " 50 ", "+50", "50.0 %  "] {
        assert_eq!(value::parse_percent(text), Some(0.5), "{text:?}");
    }
    assert_eq!(value::parse_percent("-3.0 %"), Some(-0.03));
    for fraction in [0.0, 0.123, 0.5, 0.999, 1.0, -0.25] {
        let text = shown(value::percent(fraction, Signed), " %");
        let back = value::parse_percent(&text).unwrap();
        assert_eq!(value::percent(back, Signed), value::percent(fraction, Signed), "{text}");
    }
}

/// Decibels to 0.1 dB, signed or not by parameter, and "-inf" at a floor
/// for a parameter that has one (Rödalog's Gain, −60 dB, its AMP-3).
#[test]
fn decibels_to_a_tenth() {
    let table = [
        (0.0, Signed, "0.0"),
        (-0.04, Signed, "0.0"),
        (0.04, Signed, "0.0"),
        (3.5, Signed, "+3.5"),
        (3.5, Unsigned, "3.5"),
        (-6.0, Signed, "-6.0"),
        (-60.0, Unsigned, "-60.0"),
        (-95.96, Unsigned, "-96.0"),
    ];
    for (db, sign, text) in table {
        assert_eq!(value::decibels(db, sign), text, "{db}");
    }
    assert_eq!(shown(value::decibels(-48.0, Unsigned), " dBFS"), "-48.0 dBFS");
}

#[test]
fn decibels_at_or_below_the_floor_as_rounded_read_minus_inf() {
    let table = [(-60.0, "-inf"), (-75.0, "-inf"), (-59.96, "-inf"), (-59.94, "-59.9"), (6.0, "+6.0"), (0.0, "0.0")];
    for (db, text) in table {
        assert_eq!(value::decibels_floored(db, Signed, -60.0), text, "{db}");
    }
    assert_eq!(value::FLOOR, "-inf");
    assert_eq!(shown(value::decibels_floored(-80.0, Signed, -60.0), " dB"), "-inf dB");
}

#[test]
fn decibels_read_back() {
    let table = [
        ("+3.5 dB", Some(3.5)),
        ("3.5dB", Some(3.5)),
        ("-48.0 dBFS", Some(-48.0)),
        ("-48 dbfs", Some(-48.0)),
        ("0", Some(0.0)),
        ("-inf dB", None),
        ("inf", None),
        ("nan", None),
        ("", None),
        ("loud", None),
    ];
    for (text, db) in table {
        assert_eq!(value::parse_decibels(text, None), db, "{text:?}");
    }
    for text in ["-inf", "-INF dB", " -inf  dB "] {
        assert_eq!(value::parse_decibels(text, Some(-60.0)), Some(-60.0), "{text:?}");
    }
    assert_eq!(value::parse_decibels("-12.5 dB", Some(-60.0)), Some(-12.5));
    for db in [-60.0, -59.94, -12.25, 0.0, 0.04, 5.96] {
        let text = shown(value::decibels_floored(db, Signed, -60.0), " dB");
        let back = value::parse_decibels(&text, Some(-60.0)).unwrap();
        assert_eq!(value::decibels_floored(back, Signed, -60.0), value::decibels_floored(db, Signed, -60.0), "{text}");
    }
}

/// PW-1 and PW-1a: frequencies to three significant figures, in kHz from
/// 1 kHz (as rounded), writing their own unit.
#[test]
fn pw_1a_frequencies_three_figures_then_khz_with_their_own_unit() {
    let table = [
        (0.03, "0.0300 Hz"),
        (0.045, "0.0450 Hz"),
        (0.513, "0.513 Hz"),
        (2.0, "2.00 Hz"),
        (11.7, "11.7 Hz"),
        (20.0, "20.0 Hz"),
        (45.5, "45.5 Hz"),
        (440.0, "440 Hz"),
        (523.0, "523 Hz"),
        (999.4, "999 Hz"),
        (999.5, "1.00 kHz"),
        (1_200.0, "1.20 kHz"),
        (1_234.0, "1.23 kHz"),
        (9_996.0, "10.0 kHz"),
        (20_000.0, "20.0 kHz"),
        (99_960.0, "100 kHz"),
    ];
    for (hz, text) in table {
        assert_eq!(value::hertz(hz), text, "{hz}");
    }
}

#[test]
fn frequencies_read_back() {
    let table = [
        ("440 Hz", Some(440.0)),
        ("440hz", Some(440.0)),
        ("440", Some(440.0)),
        ("+440", Some(440.0)),
        ("1.2 kHz", Some(1_200.0)),
        ("1.2k", Some(1_200.0)),
        ("1.2 K", Some(1_200.0)),
        ("1.2KHZ", Some(1_200.0)),
        ("kHz", None),
        ("fast", None),
    ];
    for (text, hz) in table {
        assert_eq!(value::parse_hertz(text), hz, "{text:?}");
    }
    for hz in [0.03, 0.0451, 0.513, 9.996, 999.4, 999.5, 1_234.0, 20_000.0] {
        let text = value::hertz(hz);
        let back = value::parse_hertz(&text).unwrap();
        assert_eq!(value::hertz(back), text, "{text}");
    }
}

/// PW-15 (Ian, 2026-11-10): times to three significant figures, in s from
/// 1 s (as rounded), writing their own unit, as frequencies do.
#[test]
fn pw_15_times_three_figures_then_seconds_with_their_own_unit() {
    let table = [
        (0.45, "0.450 ms"),
        (1.66, "1.66 ms"),
        (3.505, "3.51 ms"),
        (5.0, "5.00 ms"),
        (9.996, "10.0 ms"),
        (50.0, "50.0 ms"),
        (350.0, "350 ms"),
        (999.4, "999 ms"),
        (999.6, "1.00 s"),
        (1_234.0, "1.23 s"),
        (2_000.0, "2.00 s"),
        (9_996.0, "10.0 s"),
        (45_000.0, "45.0 s"),
        (99_960.0, "100 s"),
    ];
    for (ms, text) in table {
        assert_eq!(value::time(ms), text, "{ms}");
    }
}

#[test]
fn times_read_back_in_milliseconds() {
    let table = [
        ("350 ms", Some(350.0)),
        ("350ms", Some(350.0)),
        ("350", Some(350.0)),
        ("+350", Some(350.0)),
        ("2.5 s", Some(2_500.0)),
        ("2.5S", Some(2_500.0)),
        ("0.45 MS", Some(0.45)),
        ("ms", None),
        ("s", None),
        ("slow", None),
    ];
    for (text, ms) in table {
        assert_eq!(value::parse_time(text), ms, "{text:?}");
    }
    for ms in [0.45, 3.505, 9.996, 350.0, 999.4, 999.6, 1_999.0, 45_000.0] {
        let text = value::time(ms);
        let back = value::parse_time(&text).unwrap();
        assert_eq!(value::time(back), text, "{text}");
    }
}

/// Cents to 0.1, signed (Rödalog's Osc 2 Fine Tune).
#[test]
fn cents_to_a_tenth_signed() {
    let table = [(3.0, "+3.0"), (0.0, "0.0"), (-0.04, "0.0"), (-50.0, "-50.0"), (12.34, "+12.3")];
    for (c, text) in table {
        assert_eq!(value::cents(c), text, "{c}");
    }
    assert_eq!(shown(value::cents(3.0), " cents"), "+3.0 cents");
}

/// Whole numbers: semitones, octaves, whole cents (signed), counts.
#[test]
fn integers_signed_or_not() {
    let table = [(7, Signed, "+7"), (0, Signed, "0"), (-2, Signed, "-2"), (7, Unsigned, "7"), (-2, Unsigned, "-2")];
    for (n, sign, text) in table {
        assert_eq!(value::integer(n, sign), text, "{n}");
    }
    assert_eq!(shown(value::integer(7, Signed), " st"), "+7 st");
}

#[test]
fn integers_read_back_rounded() {
    let table = [("+7 st", Some(7)), ("-2 oct", Some(-2)), ("7", Some(7)), ("6.6 st", Some(7)), ("-0.4", Some(0))];
    for (text, n) in table {
        assert_eq!(value::parse_integer(text, &["st", "oct"]), n, "{text:?}");
    }
    assert_eq!(value::parse_integer("seven", &["st"]), None);
}

/// The general parse: with or without one of the units (any ASCII case,
/// longest listed first), a leading "+", spaces around; finite numbers
/// only.
#[test]
fn parse_takes_a_number_with_or_without_its_unit() {
    let table = [
        ("12.5", Some(12.5)),
        (" +12.5 ", Some(12.5)),
        ("-3", Some(-3.0)),
        ("180°", Some(180.0)),
        ("180 °", Some(180.0)),
        ("8.0 steps/s", Some(8.0)),
        ("8 STEPS/S", Some(8.0)),
        ("12.5 %%", None),
        ("inf", None),
        ("NaN", None),
        ("", None),
        ("°", None),
        // A tail that isn't a whole character of the text isn't a unit.
        ("é", None),
    ];
    for (text, x) in table {
        assert_eq!(value::parse(text, &["steps/s", "°", "%"]), x, "{text:?}");
    }
    assert_eq!(value::parse("3 dB", &[]), None);
}
