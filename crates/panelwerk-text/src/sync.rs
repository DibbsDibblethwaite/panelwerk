//! Tempo-sync length names and list IDs, in Rödalog's words (FAM-1), frozen
//! (PW-3).
//!
//! Names only: a length in beats is each engine's business (PW-5), and each
//! consumer maps its engine's type to [`NoteLength`]. Two kinds of length
//! are kept distinct (PW-2): **"N bars" and "1 bar" are always the host's
//! bar**, whatever its meter, and **"1/1" … "1/32" are always based on four
//! beats** (1/4 is one beat), in any meter.
//!
//! A name can be a saved key (Rödalog's presets save a list's choice by its
//! shown name, its B-PRE-3), and a list ID is one too (Kör's and Fläns's
//! states), so neither ever changes: Panelwerk's tests freeze every one,
//! and a change is a breaking change agreed with Ian (CLAUDE.md rule 6).
//!
//! # A consumer's list
//!
//! nice-plug's `#[derive(Enum)]` takes each name and ID as a literal, so a
//! consumer keeps its literals and proves them Panelwerk's with one line per
//! list (PW-4), its lengths in its own order (rule 1):
//!
//! ```rust,ignore
//! use panelwerk_text::sync::{ids, names, NoteLength::*};
//! const LIST: [NoteLength; 3] = [Bars(2), Note(4), Dotted(4)];
//! assert_eq!(SyncRate::variants(), names(LIST));
//! assert_eq!(SyncRate::ids(), Some(&ids(LIST)[..]));
//! ```

/// A tempo-synced length the family names: [`NoteLength::ALL`] lists them.
///
/// `Bars(n)` is `n` of the host's bars; `Note(n)` is 1/n of four beats, and
/// `Dotted(n)` and `Triplet(n)` that length × 3/2 and × 2/3. A value not in
/// [`ALL`](NoteLength::ALL) (`Note(3)`, `Bars(5)`) has no name.
///
/// ```
/// use panelwerk_text::sync::NoteLength;
///
/// let length = NoteLength::Dotted(4);
/// assert_eq!(length.name(), "1/4 dotted");
/// assert_eq!(length.id(), "4d");
/// assert_eq!(NoteLength::Bars(1).name(), "1 bar");
/// assert_eq!(NoteLength::from_id("16t"), Some(NoteLength::Triplet(16)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLength {
    /// `n` of the host's bars: 16, 8, 4, 2 or 1.
    Bars(u32),
    /// 1/n of four beats: n is 1, 2, 4, 8, 16 or 32.
    Note(u32),
    /// 1/n dotted, × 3/2.
    Dotted(u32),
    /// 1/n triplet, × 2/3.
    Triplet(u32),
}

use NoteLength::{Bars, Dotted, Note, Triplet};

impl NoteLength {
    /// Every length the family names: 16, 8, 4, 2 and 1 bars, then 1/1 …
    /// 1/32, each straight, dotted and triplet. Frozen (PW-3); a consumer's
    /// list is any of these in its own order.
    pub const ALL: [NoteLength; 23] = [
        Bars(16),
        Bars(8),
        Bars(4),
        Bars(2),
        Bars(1),
        Note(1),
        Dotted(1),
        Triplet(1),
        Note(2),
        Dotted(2),
        Triplet(2),
        Note(4),
        Dotted(4),
        Triplet(4),
        Note(8),
        Dotted(8),
        Triplet(8),
        Note(16),
        Dotted(16),
        Triplet(16),
        Note(32),
        Dotted(32),
        Triplet(32),
    ];

    /// The shown name (FAM-1): "4 bars", "1 bar", "1/4", "1/4 dotted",
    /// "1/4 triplet".
    ///
    /// # Panics
    ///
    /// For a length not in [`ALL`](NoteLength::ALL); in a `const` it fails
    /// to compile instead.
    pub const fn name(self) -> &'static str {
        match self {
            Bars(16) => "16 bars",
            Bars(8) => "8 bars",
            Bars(4) => "4 bars",
            Bars(2) => "2 bars",
            Bars(1) => "1 bar",
            Note(1) => "1/1",
            Dotted(1) => "1/1 dotted",
            Triplet(1) => "1/1 triplet",
            Note(2) => "1/2",
            Dotted(2) => "1/2 dotted",
            Triplet(2) => "1/2 triplet",
            Note(4) => "1/4",
            Dotted(4) => "1/4 dotted",
            Triplet(4) => "1/4 triplet",
            Note(8) => "1/8",
            Dotted(8) => "1/8 dotted",
            Triplet(8) => "1/8 triplet",
            Note(16) => "1/16",
            Dotted(16) => "1/16 dotted",
            Triplet(16) => "1/16 triplet",
            Note(32) => "1/32",
            Dotted(32) => "1/32 dotted",
            Triplet(32) => "1/32 triplet",
            _ => panic!("not a length the family names"),
        }
    }

    /// The list ID, Kör's and Fläns's: "16bar" … "1bar", then "1", "1d",
    /// "1t" … "32t".
    ///
    /// # Panics
    ///
    /// For a length not in [`ALL`](NoteLength::ALL), as [`name`](NoteLength::name).
    pub const fn id(self) -> &'static str {
        match self {
            Bars(16) => "16bar",
            Bars(8) => "8bar",
            Bars(4) => "4bar",
            Bars(2) => "2bar",
            Bars(1) => "1bar",
            Note(1) => "1",
            Dotted(1) => "1d",
            Triplet(1) => "1t",
            Note(2) => "2",
            Dotted(2) => "2d",
            Triplet(2) => "2t",
            Note(4) => "4",
            Dotted(4) => "4d",
            Triplet(4) => "4t",
            Note(8) => "8",
            Dotted(8) => "8d",
            Triplet(8) => "8t",
            Note(16) => "16",
            Dotted(16) => "16d",
            Triplet(16) => "16t",
            Note(32) => "32",
            Dotted(32) => "32d",
            Triplet(32) => "32t",
            _ => panic!("not a length the family names"),
        }
    }

    /// The length whose shown name is exactly `name` (a saved key read
    /// back, as Rödalog's presets do).
    ///
    /// ```
    /// use panelwerk_text::sync::NoteLength;
    ///
    /// assert_eq!(NoteLength::from_name("1/8 triplet"), Some(NoteLength::Triplet(8)));
    /// assert_eq!(NoteLength::from_name("1/8 Triplet"), None);
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }

    /// The length whose list ID is exactly `id`.
    ///
    /// ```
    /// use panelwerk_text::sync::NoteLength;
    ///
    /// assert_eq!(NoteLength::from_id("2bar"), Some(NoteLength::Bars(2)));
    /// ```
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.id() == id)
    }

    /// The length a user typed: its name in any case and spacing ("1/4
    /// Dotted", "1/4dotted"), a bar count singular or plural ("4 bar",
    /// "1 bars"), or Kör's former short forms ("1/4.", "1/4T", and "1/4d").
    /// A bare ID isn't taken ("4" could mean 1/4 or 4 bars). The caller
    /// finds the length in its own list, which may not hold it.
    ///
    /// ```
    /// use panelwerk_text::sync::NoteLength;
    ///
    /// assert_eq!(NoteLength::parse(" 1/16 Triplet"), Some(NoteLength::Triplet(16)));
    /// assert_eq!(NoteLength::parse("1/4."), Some(NoteLength::Dotted(4)));
    /// assert_eq!(NoteLength::parse("4 bar"), Some(NoteLength::Bars(4)));
    /// assert_eq!(NoteLength::parse("4"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let typed = key(text);
        Self::ALL.into_iter().find(|l| l.typed_forms().contains(&typed))
    }

    /// The forms [`parse`](NoteLength::parse) takes for this length, each
    /// as a [`key`].
    fn typed_forms(self) -> Vec<String> {
        let mut forms = vec![key(self.name())];
        match self {
            Bars(n) => forms.extend([format!("{n}bar"), format!("{n}bars")]),
            Dotted(n) => forms.extend([format!("1/{n}."), format!("1/{n}d")]),
            Triplet(n) => forms.push(format!("1/{n}t")),
            Note(_) => {}
        }
        forms
    }
}

/// `text` in lower case with its whitespace removed, for comparing typed
/// text.
fn key(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

/// The shown names of a consumer's list, in its order, at compile time
/// (PW-4): what its `Enum::variants()` must equal.
///
/// ```
/// use panelwerk_text::sync::{names, NoteLength::*};
///
/// const NAMES: [&str; 3] = names([Bars(2), Note(4), Triplet(8)]);
/// assert_eq!(NAMES, ["2 bars", "1/4", "1/8 triplet"]);
/// ```
pub const fn names<const N: usize>(list: [NoteLength; N]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = list[i].name();
        i += 1;
    }
    out
}

/// The list IDs of a consumer's list, in its order, at compile time (PW-4):
/// what its `Enum::ids()` must hold.
///
/// ```
/// use panelwerk_text::sync::{ids, NoteLength::*};
///
/// const IDS: [&str; 3] = ids([Bars(2), Note(4), Triplet(8)]);
/// assert_eq!(IDS, ["2bar", "4", "8t"]);
/// ```
pub const fn ids<const N: usize>(list: [NoteLength; N]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = list[i].id();
        i += 1;
    }
    out
}
