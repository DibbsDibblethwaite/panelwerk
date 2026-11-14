//! Held edits (PW-HOLD): the panel keeps a host's gesture open until the
//! plugin has the value set in it, then ends it, so an edit made in one
//! instant survives every conforming host (Kör's and Fläns's B-PNL-11;
//! Rödalog's B-PAR-11, where the mechanism was proved in Cubase on v2).
//!
//! **Why.** While audio runs, nice-plug's VST3 layer doesn't apply a panel's
//! set itself, and reports the old value until the host passes the change
//! back at its next process call. A host that hands the value back (Reaper)
//! is fine. A host that reads the plugin's value back when the edit ends, or
//! passes a change on only while its edit is open (Cubase, it seems), undoes
//! every edit begun, set and ended in one instant: a click on a switch or a
//! mode button, a double-click for the default. Keeping the gesture open
//! until the plugin has the value cures both, in the panel, with no change
//! to the framework; making the framework report the panel's value at once
//! would cure only the first. A drag already holds its gesture open while
//! the host processes, so it ends as before, a frame later.
//!
//! **How.** [`Held`] sits between the panel's widgets and the plugin's own
//! host ("raw", the [`ParamHost`] that talks to the framework):
//! - [`Held::begin`] tells the host, unless that parameter's gesture is
//!   still open or waiting: then it continues it, so the host never sees a
//!   gesture begun twice (a second click while the first waits, a drag on a
//!   knob whose double-click waits). Begins nest: an open gesture counts
//!   them, and waits only at the last one's end, so an instant edit during a
//!   drag (a consumer's key) doesn't cut the drag off.
//! - [`Held::set`] passes the set on, and remembers it as the value the
//!   gesture waits for.
//! - [`Held::end`] tells the host nothing: the gesture waits, from the
//!   panel's clock at that frame. An end with nothing open, or one too
//!   many, does nothing.
//! - [`Held::frame`], at the start of each frame, ends for the host the
//!   waiting gestures whose value the plugin now has
//!   ([`ParamHost::shows`]), or that have waited [`PENDING_FOR`] (the
//!   panel's own limit for going by the value it set, so the two agree), in
//!   the order begun. Gestures ended at one moment are one batch, ended
//!   together once all their values have arrived or the limit has passed,
//!   so edits the panel ended together reach the host together.
//! - [`Held::close`], as the window goes, ends every gesture still open or
//!   waiting, once, in the order begun: no frame will come to end them.
//!
//! A gesture is still one begin–end pair for the host (Kör's and Fläns's
//! B-PNL-3), the end only later. A plugin that doesn't wire `Held` in is
//! unchanged. The panel's thread only (`CLAUDE.md` rule 3); its clock is
//! egui's input time (rule 5).
//!
//! **Wiring it in** (a consumer's editor, a few lines):
//! - the `App` that lives as long as the window owns a `Held`;
//! - the `ParamHost` the panel is given passes `begin`, `set` and `end`
//!   through it to the adapter that talks to the framework, and may
//!   override [`ParamHost::shows`] to compare plain values;
//! - the `App` calls [`Held::frame`] with egui's input time before it draws
//!   the panel, and [`Held::close`] when the window goes (its `Drop`, since
//!   nice-plug-egui 0.6 drops the app with its window);
//! - a panel that draws only when asked asks for another frame while
//!   `frame` says an edit waits (`ctx.request_repaint()`), or the edit ends
//!   only at the next input.
//!
//! ```
//! use std::cell::Cell;
//! use panelwerk_panel::{hold::Held, ParamHost};
//!
//! /// The adapter that talks to the framework: here, one parameter whose
//! /// sets the plugin has only at the next process call.
//! struct Raw { value: Cell<f32>, sent: Cell<f32>, told: Cell<&'static str> }
//! impl ParamHost for Raw {
//!     fn value(&self, _: &str) -> f32 { self.value.get() }
//!     fn default_value(&self, _: &str) -> f32 { 0.0 }
//!     fn text(&self, _: &str) -> String { String::new() }
//!     fn begin(&self, _: &str) { self.told.set("begun") }
//!     fn set(&self, _: &str, normalized: f32) { self.sent.set(normalized) }
//!     fn end(&self, _: &str) { self.told.set("ended") }
//! }
//!
//! /// What the panel is given: the held edits in between.
//! struct Frame<'a> { raw: &'a Raw, held: &'a Held }
//! impl ParamHost for Frame<'_> {
//!     fn value(&self, id: &str) -> f32 { self.raw.value(id) }
//!     fn default_value(&self, id: &str) -> f32 { self.raw.default_value(id) }
//!     fn text(&self, id: &str) -> String { self.raw.text(id) }
//!     fn begin(&self, id: &str) { self.held.begin(self.raw, id) }
//!     fn set(&self, id: &str, normalized: f32) { self.held.set(self.raw, id, normalized) }
//!     fn end(&self, id: &str) { self.held.end(id) }
//! }
//!
//! let raw = Raw { value: Cell::new(0.0), sent: Cell::new(0.0), told: Cell::new("") };
//! let held = Held::new();
//! let frame = Frame { raw: &raw, held: &held };
//! // A frame at 1 s: a click sets "on" in one instant.
//! held.frame(&raw, 1.0);
//! frame.begin("on");
//! frame.set("on", 1.0);
//! frame.end("on");
//! // The next frame: the plugin hasn't the value, so the edit stays open.
//! assert!(held.frame(&raw, 1.02), "an edit waits: ask for another frame");
//! assert_eq!(raw.told.get(), "begun");
//! // The process call brings it; the frame after ends the edit.
//! raw.value.set(raw.sent.get());
//! assert!(!held.frame(&raw, 1.04));
//! assert_eq!(raw.told.get(), "ended");
//! ```

use std::cell::RefCell;

use crate::gesture::PENDING_FOR;
use crate::ParamHost;

/// The panel's gestures the host has been told have begun and not yet that
/// they've ended (see the [module](self) doc). One per panel window, on its
/// thread; `Send`, so a consumer's app that owns it can be.
///
/// ```
/// use panelwerk_panel::hold::Held;
/// use panelwerk_testkit::FakeHost;
///
/// let host = FakeHost::new(&[("on", 0.0)]);
/// let held = Held::new();
/// held.frame(&host, 0.0);
/// held.begin(&host, "on");
/// held.set(&host, "on", 1.0);
/// held.end("on");
/// assert_eq!(host.take(), ["begin on", "set on"]);
/// held.frame(&host, 0.02);
/// assert_eq!(host.take(), ["end on"], "the value had arrived");
/// ```
#[derive(Debug, Default)]
pub struct Held {
    state: RefCell<State>,
}

#[derive(Debug, Default)]
struct State {
    /// The panel's clock: the time of the frame being drawn, in seconds.
    now: f64,
    /// Each gesture the host knows of, by parameter, in the order begun, so
    /// they end in that order.
    gestures: Vec<(String, Gesture)>,
}

/// A gesture the host knows of.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    /// Begun, not yet ended by the panel: how many of the panel's begins
    /// await their end (nested begins: an instant edit inside a drag), and
    /// the value last set in it, if any.
    Open { depth: u32, last: Option<f32> },
    /// Ended by the panel at `at` (the panel's clock), waiting for the
    /// plugin to have `value` (none set: nothing to wait for), with every
    /// gesture ended at the same moment, or for [`PENDING_FOR`].
    Waiting { value: Option<f32>, at: f64 },
}

impl Held {
    /// No gesture open.
    pub fn new() -> Self {
        Held::default()
    }

    /// The panel begins a gesture on `id`: `raw` is told, unless the host
    /// already has one open for it, still open or waiting, which this then
    /// continues (one more begin counted, if it's open).
    pub fn begin(&self, raw: &dyn ParamHost, id: &str) {
        {
            let mut state = self.state.borrow_mut();
            if let Some((_, gesture)) = state.gestures.iter_mut().find(|(p, _)| p == id) {
                *gesture = match *gesture {
                    Gesture::Open { depth, last } => Gesture::Open { depth: depth + 1, last },
                    // Its value carried over: a gesture that sets nothing
                    // more still waits for it.
                    Gesture::Waiting { value, .. } => Gesture::Open { depth: 1, last: value },
                };
                return;
            }
            state.gestures.push((id.to_owned(), Gesture::Open { depth: 1, last: None }));
        }
        // Told with the state released, so nothing the host does as a
        // gesture begins can find it borrowed.
        raw.begin(id);
    }

    /// The panel sets `id`: passed on to `raw`, and, in an open gesture, the
    /// value it will wait for.
    pub fn set(&self, raw: &dyn ParamHost, id: &str, normalized: f32) {
        raw.set(id, normalized);
        let mut state = self.state.borrow_mut();
        if let Some((_, Gesture::Open { last, .. })) = state.gestures.iter_mut().find(|(p, _)| p == id) {
            *last = Some(normalized);
        }
    }

    /// The panel ends its gesture on `id`: at the last nested begin's end it
    /// waits, from this frame's time, and a later [`Held::frame`] tells the
    /// host. An end with no gesture open does nothing.
    pub fn end(&self, id: &str) {
        let mut state = self.state.borrow_mut();
        let at = state.now;
        if let Some((_, gesture)) = state.gestures.iter_mut().find(|(p, _)| p == id) {
            if let Gesture::Open { depth, last } = *gesture {
                *gesture = match depth {
                    1 => Gesture::Waiting { value: last, at },
                    _ => Gesture::Open { depth: depth - 1, last },
                };
            }
        }
    }

    /// A frame starts at `now` seconds (egui's input time): the waiting
    /// gestures whose batch is ready are ended for `raw`, in the order
    /// begun. A batch is every gesture the panel ended at one moment; it is
    /// ready once `raw` shows each one's value, or it has waited
    /// [`PENDING_FOR`] (or the clock is before it: a new window's, started
    /// again). Returns whether a gesture still waits, so a panel that draws
    /// only when asked can ask for another frame.
    pub fn frame(&self, raw: &dyn ParamHost, now: f64) -> bool {
        let mut ended = Vec::new();
        let waiting = {
            let mut state = self.state.borrow_mut();
            state.now = now;
            let waited = |at: f64| !(0.0..PENDING_FOR).contains(&(now - at));
            // The moments whose gestures aren't all ready.
            let unready: Vec<f64> = state
                .gestures
                .iter()
                .filter_map(|(id, gesture)| match *gesture {
                    Gesture::Waiting { value, at } if !value.is_none_or(|v| raw.shows(id, v)) && !waited(at) => {
                        Some(at)
                    }
                    _ => None,
                })
                .collect();
            state.gestures.retain(|(id, gesture)| match *gesture {
                Gesture::Waiting { at, .. } if !unready.contains(&at) => {
                    ended.push(id.clone());
                    false
                }
                _ => true,
            });
            !unready.is_empty()
        };
        // Told with the state released, as in begin.
        for id in ended {
            raw.end(&id);
        }
        waiting
    }

    /// The window closes: every gesture still open or waiting is ended for
    /// `raw` now, once, in the order begun, whether or not its value has
    /// arrived; no frame will come to end it.
    pub fn close(&self, raw: &dyn ParamHost) {
        let gestures = std::mem::take(&mut self.state.borrow_mut().gestures);
        for (id, _) in gestures {
            raw.end(&id);
        }
    }
}
