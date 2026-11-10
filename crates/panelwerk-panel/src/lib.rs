//! Panelwerk's panel layer: the look and the controls Ian Dilley's plugins
//! (Rödalog, Kör, Fläns) share, drawn with egui.
//!
//! No `unsafe`, no plugin code: the panel reaches the plugin only through
//! the consumer's host trait. Empty until steps 2 and 3 of
//! `docs/proposal.md`: the palette and its contrast test, knobs and their
//! arcs, lights, switches, lists, the red title strip, the help line and
//! the zoom menu.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
