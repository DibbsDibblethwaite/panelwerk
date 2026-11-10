//! Panelwerk's text layer: the conventions Ian Dilley's plugins (Rödalog,
//! Kör, Fläns) share in the words they show, so the family reads as one.
//!
//! Std only, no `unsafe`. Empty until step 1 of `docs/proposal.md`: the
//! tempo-sync length names in Rödalog's words ("4 bars", "1/4",
//! "1/4 dotted", "1/4 triplet"; FAM-1), value text for dB, %, Hz, ms and
//! cents, and the list-ID conventions.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
