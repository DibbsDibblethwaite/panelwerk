//! Value text in the family's one format (PW-1, Ian, 2026-11-10), and
//! parsing it back.
//!
//! # The format
//!
//! | Kind | Function | Text | Unit |
//! | --- | --- | --- | --- |
//! | Percentage | [`percent`] | one decimal: "50.0", "+12.5" | the framework's, " %" |
//! | Level | [`decibels`], [`decibels_floored`] | 0.1 dB: "+3.5", "-48.0"; [`FLOOR`] at a floor | the framework's, " dB", " dBFS" |
//! | Frequency | [`hertz`] | three significant figures, kHz from 1 kHz: "0.513 Hz", "440 Hz", "1.20 kHz" | **its own** (PW-1a) |
//! | Time | [`milliseconds`] | three significant figures: "0.450", "3.51", "350" | the framework's, " ms" |
//! | Fine tuning | [`cents`] | 0.1, signed: "+3.0" | the framework's, " cents" |
//! | Whole steps | [`integer`] | "+7", "-2" (semitones, octaves, whole cents); "3" (counts) | the framework's, " st", " oct" |
//! | Anything else | [`fixed`], [`significant`] | a rate "8.00", an angle "180" | the framework's |
//!
//! A parameter's unit is added by the framework (nice-plug's `with_unit`,
//! a space first), so these return the number alone; frequencies are the
//! one exception (PW-1a, Ian, 2026-11-10), since their unit changes from
//! Hz to kHz and a framework unit is one fixed string.
//!
//! The conventions every kind keeps: ASCII "-", so typed text reads back;
//! no negative zero; a "+" above zero only where the kind or the parameter
//! is signed; rounding half away from zero; and a format chosen by the value
//! *as rounded*, so text read back shows the same way ("999.5 Hz" reads
//! "1.00 kHz", and "1.00 kHz" read back reads the same).
//!
//! ```rust,ignore
//! // A consumer's parameter (nice-plug):
//! FloatParam::new("Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
//!     .with_unit(" %")
//!     .with_value_to_string(Arc::new(|v| value::percent(v, Sign::Unsigned)))
//!     .with_string_to_value(Arc::new(value::parse_percent))
//! ```
//!
//! Nothing here runs on the audio thread (CLAUDE.md rule 3): each function
//! allocates its `String`.

/// Whether a value above zero shows a "+": a parameter whose range runs
/// both sides of zero is signed, one that starts at zero isn't.
///
/// ```
/// use panelwerk_text::value::{decibels, Sign};
///
/// assert_eq!(decibels(3.5, Sign::Signed), "+3.5");
/// assert_eq!(decibels(3.5, Sign::Unsigned), "3.5");
/// assert_eq!(decibels(-3.5, Sign::Unsigned), "-3.5");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    /// No "+": "3.5", "0.0", "-3.5".
    Unsigned,
    /// "+" above zero (as rounded): "+3.5", "0.0", "-3.5".
    Signed,
}

/// The text of a level at its parameter's floor: silence (Rödalog's Gain,
/// its AMP-3). [`parse_decibels`] reads it back as the floor.
pub const FLOOR: &str = "-inf";

/// `x` to `places` decimal places: the building block of the fixed-place
/// kinds, and the format for any kind without its own (an angle, "180").
///
/// ```
/// use panelwerk_text::value::{fixed, Sign};
///
/// assert_eq!(fixed(180.0, 0, Sign::Unsigned), "180");
/// assert_eq!(fixed(-0.04, 1, Sign::Signed), "0.0");
/// assert_eq!(fixed(12.25, 1, Sign::Signed), "+12.3");
/// ```
pub fn fixed(x: f32, places: usize, sign: Sign) -> String {
    render(round_to(f64::from(x), places as i32), places, sign)
}

/// `x` to `figures` significant figures (at least one), its places chosen
/// by the value as rounded: the building block of frequencies and times,
/// and the format for any rate without its own ("8.00" steps/s).
///
/// ```
/// use panelwerk_text::value::significant;
///
/// assert_eq!(significant(0.513, 3), "0.513");
/// assert_eq!(significant(11.7, 3), "11.7");
/// assert_eq!(significant(9.996, 3), "10.0");
/// assert_eq!(significant(1234.0, 3), "1230");
/// ```
pub fn significant(x: f32, figures: usize) -> String {
    let (rounded, places) = round_significant(f64::from(x), figures);
    render(rounded, places, Sign::Unsigned)
}

/// A fraction (0 … 1, or −1 … +1) as a percentage to one decimal (PW-1),
/// without its unit.
///
/// ```
/// use panelwerk_text::value::{percent, Sign};
///
/// assert_eq!(percent(0.5, Sign::Unsigned), "50.0");
/// assert_eq!(percent(0.125, Sign::Signed), "+12.5");
/// assert_eq!(percent(-0.0004, Sign::Signed), "0.0");
/// ```
pub fn percent(fraction: f32, sign: Sign) -> String {
    render(round_to(f64::from(fraction) * 100.0, 1), 1, sign)
}

/// A level in dB to 0.1 dB, without its unit.
///
/// ```
/// use panelwerk_text::value::{decibels, Sign};
///
/// assert_eq!(decibels(-48.0, Sign::Unsigned), "-48.0");
/// assert_eq!(decibels(0.04, Sign::Signed), "0.0");
/// ```
pub fn decibels(db: f32, sign: Sign) -> String {
    fixed(db, 1, sign)
}

/// A level in dB as [`decibels`], but [`FLOOR`] ("-inf") where it rounds
/// to `floor` or below: for a parameter whose bottom is silence.
///
/// ```
/// use panelwerk_text::value::{decibels_floored, Sign};
///
/// assert_eq!(decibels_floored(-59.96, Sign::Signed, -60.0), "-inf");
/// assert_eq!(decibels_floored(-59.94, Sign::Signed, -60.0), "-59.9");
/// ```
pub fn decibels_floored(db: f32, sign: Sign, floor: f32) -> String {
    if round_to(f64::from(db), 1) <= f64::from(floor) {
        FLOOR.to_owned()
    } else {
        decibels(db, sign)
    }
}

/// A frequency to three significant figures (PW-1), in kHz from 1 kHz as
/// rounded, **with its unit** (PW-1a): the one kind whose parameter takes
/// no framework unit.
///
/// ```
/// use panelwerk_text::value::hertz;
///
/// assert_eq!(hertz(0.513), "0.513 Hz");
/// assert_eq!(hertz(440.0), "440 Hz");
/// assert_eq!(hertz(999.5), "1.00 kHz");
/// assert_eq!(hertz(1234.0), "1.23 kHz");
/// ```
pub fn hertz(hz: f32) -> String {
    let hz = f64::from(hz);
    if round_significant(hz, 3).0.abs() >= 1_000.0 {
        let (khz, places) = round_significant(hz / 1_000.0, 3);
        format!("{} kHz", render(khz, places, Sign::Unsigned))
    } else {
        let (hz, places) = round_significant(hz, 3);
        format!("{} Hz", render(hz, places, Sign::Unsigned))
    }
}

/// A time in ms to three significant figures, without its unit
/// (proposed for the family, PW-15).
///
/// ```
/// use panelwerk_text::value::milliseconds;
///
/// assert_eq!(milliseconds(0.45), "0.450");
/// assert_eq!(milliseconds(350.0), "350");
/// ```
pub fn milliseconds(ms: f32) -> String {
    significant(ms, 3)
}

/// A fine tuning in cents to 0.1, signed, without its unit.
///
/// ```
/// use panelwerk_text::value::cents;
///
/// assert_eq!(cents(3.0), "+3.0");
/// assert_eq!(cents(-12.34), "-12.3");
/// ```
pub fn cents(cents: f32) -> String {
    fixed(cents, 1, Sign::Signed)
}

/// A whole number: semitones, octaves and whole cents signed, counts not;
/// without its unit.
///
/// ```
/// use panelwerk_text::value::{integer, Sign};
///
/// assert_eq!(integer(7, Sign::Signed), "+7");
/// assert_eq!(integer(0, Sign::Signed), "0");
/// assert_eq!(integer(3, Sign::Unsigned), "3");
/// ```
pub fn integer(n: i32, sign: Sign) -> String {
    match sign {
        Sign::Signed if n > 0 => format!("+{n}"),
        _ => n.to_string(),
    }
}

/// A typed number, with or without one of `units` after it (any ASCII
/// case; list a longer unit before one it ends with, "dBFS" before "dB"),
/// a leading "+", spaces around. `None` unless it is a finite number.
///
/// ```
/// use panelwerk_text::value::parse;
///
/// assert_eq!(parse(" +180 °", &["°"]), Some(180.0));
/// assert_eq!(parse("8 Steps/s", &["steps/s"]), Some(8.0));
/// assert_eq!(parse("inf", &[]), None);
/// ```
pub fn parse(text: &str, units: &[&str]) -> Option<f32> {
    number(split_unit(text, units).0)
}

/// A typed whole number, as [`parse`], rounded half away from zero.
///
/// ```
/// use panelwerk_text::value::parse_integer;
///
/// assert_eq!(parse_integer("+7 st", &["st"]), Some(7));
/// assert_eq!(parse_integer("6.6", &["st"]), Some(7));
/// ```
pub fn parse_integer(text: &str, units: &[&str]) -> Option<i32> {
    parse(text, units).map(|x| x.round() as i32)
}

/// A typed percentage, with or without "%", as a fraction: the inverse of
/// [`percent`].
///
/// ```
/// use panelwerk_text::value::parse_percent;
///
/// assert_eq!(parse_percent("50.0 %"), Some(0.5));
/// assert_eq!(parse_percent("50"), Some(0.5));
/// ```
pub fn parse_percent(text: &str) -> Option<f32> {
    parse(text, &["%"]).map(|x| x / 100.0)
}

/// A typed level, with or without "dB" or "dBFS"; [`FLOOR`] ("-inf", any
/// case) reads as `floor` where the parameter has one.
///
/// ```
/// use panelwerk_text::value::parse_decibels;
///
/// assert_eq!(parse_decibels("-48.0 dBFS", None), Some(-48.0));
/// assert_eq!(parse_decibels("-inf dB", Some(-60.0)), Some(-60.0));
/// assert_eq!(parse_decibels("-inf dB", None), None);
/// ```
pub fn parse_decibels(text: &str, floor: Option<f32>) -> Option<f32> {
    let n = split_unit(text, &["dBFS", "dB"]).0;
    match floor {
        Some(floor) if n.eq_ignore_ascii_case(FLOOR) => Some(floor),
        _ => number(n),
    }
}

/// A typed frequency in Hz: with "Hz", "kHz" or "k" (any case), or none.
///
/// ```
/// use panelwerk_text::value::parse_hertz;
///
/// assert_eq!(parse_hertz("1.2 kHz"), Some(1200.0));
/// assert_eq!(parse_hertz("1.2k"), Some(1200.0));
/// assert_eq!(parse_hertz("440"), Some(440.0));
/// ```
pub fn parse_hertz(text: &str) -> Option<f32> {
    match split_unit(text, &["kHz", "k", "Hz"]) {
        (n, Some("Hz") | None) => number(n),
        (n, Some(_)) => number(n).map(|khz| khz * 1_000.0),
    }
}

/// A typed time in ms: with "ms", with "s" (seconds), or none (ms).
///
/// ```
/// use panelwerk_text::value::parse_milliseconds;
///
/// assert_eq!(parse_milliseconds("350 ms"), Some(350.0));
/// assert_eq!(parse_milliseconds("2.5 s"), Some(2500.0));
/// ```
pub fn parse_milliseconds(text: &str) -> Option<f32> {
    match split_unit(text, &["ms", "s"]) {
        (n, Some("s")) => number(n).map(|s| s * 1_000.0),
        (n, _) => number(n),
    }
}

/// `x` rounded half away from zero to `places` decimal places (tens,
/// hundreds for negative `places`).
fn round_to(x: f64, places: i32) -> f64 {
    let scale = 10_f64.powi(places.abs());
    if places >= 0 {
        (x * scale).round() / scale
    } else {
        (x / scale).round() * scale
    }
}

/// `x` rounded to `figures` significant figures (at least one), and the
/// decimal places it shows with. Its places are chosen by the value as
/// rounded: 9.996 is 10.0, not 10.00.
fn round_significant(x: f64, figures: usize) -> (f64, usize) {
    let figures = figures.max(1) as i32;
    if x == 0.0 || !x.is_finite() {
        return (x, (figures - 1) as usize);
    }
    let mut places = figures - 1 - x.abs().log10().floor() as i32;
    if round_to(x, places).abs() >= 10_f64.powi(figures - places) {
        places -= 1;
    }
    (round_to(x, places), places.max(0) as usize)
}

/// A rounded value's text: no negative zero; "+" above zero if signed.
fn render(rounded: f64, places: usize, sign: Sign) -> String {
    let r = if rounded == 0.0 { 0.0 } else { rounded };
    match sign {
        Sign::Signed if r > 0.0 => format!("+{r:.places$}"),
        _ => format!("{r:.places$}"),
    }
}

/// `text` trimmed and split from the first of `units` it ends with (any
/// ASCII case), the unit as listed; the number part trimmed.
fn split_unit<'a>(text: &'a str, units: &[&'a str]) -> (&'a str, Option<&'a str>) {
    let text = text.trim();
    for &unit in units {
        let start = text.len().checked_sub(unit.len());
        if let Some(start) = start.filter(|&s| text.get(s..).is_some_and(|tail| tail.eq_ignore_ascii_case(unit))) {
            return (text[..start].trim_end(), Some(unit));
        }
    }
    (text, None)
}

/// A finite number, with an optional leading "+".
fn number(text: &str) -> Option<f32> {
    let text = text.strip_prefix('+').unwrap_or(text);
    text.parse::<f32>().ok().filter(|x| x.is_finite())
}
