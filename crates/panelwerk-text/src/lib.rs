//! Panelwerk's text layer: the conventions Ian Dilley's plugins (Rödalog,
//! Kör, Fläns) share in the words they show, so the family reads as one.
//!
//! - [`sync`]: the tempo-sync length names in Rödalog's words ("4 bars",
//!   "1/4", "1/4 dotted", "1/4 triplet"; FAM-1) and their list IDs, frozen,
//!   with the tables a consumer tests its own list against (PW-4).
//! - [`value`]: value text in the family's one format (PW-1): percentages,
//!   decibels, frequencies, times, cents and whole steps, and parsing typed
//!   text back.
//! - [`ids`]: the convention for parameter and list IDs.
//!
//! Std only, no `unsafe`, and never on the audio thread (`CLAUDE.md`
//! rule 3). Every name and every format is interface (rule 6): a change is
//! agreed with Ian and listed in `CHANGELOG.md` with what it changes on each
//! consumer's panel.
//!
//! ```
//! use panelwerk_text::{sync::NoteLength, value};
//!
//! assert_eq!(NoteLength::Triplet(8).name(), "1/8 triplet");
//! assert_eq!(value::percent(0.5, value::Sign::Unsigned), "50.0");
//! assert_eq!(value::hertz(1200.0), "1.20 kHz");
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ids;
pub mod sync;
pub mod value;
