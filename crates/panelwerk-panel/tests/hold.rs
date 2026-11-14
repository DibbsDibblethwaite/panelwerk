//! PW-HOLD: held edits (`hold::Held`), the cure for hosts that undo an edit
//! the panel makes in one instant (Kör's and Fläns's B-PNL-11, Rödalog's
//! B-PAR-11), and `ParamHost::shows`, what a held edit waits for.

use std::cell::Cell;

use panelwerk_panel::ParamHost;

/// A host that doesn't override `shows`.
struct Plain(Cell<f32>);

impl ParamHost for Plain {
    fn value(&self, _: &str) -> f32 {
        self.0.get()
    }
    fn default_value(&self, _: &str) -> f32 {
        0.0
    }
    fn text(&self, _: &str) -> String {
        String::new()
    }
    fn begin(&self, _: &str) {}
    fn set(&self, _: &str, normalized: f32) {
        self.0.set(normalized)
    }
    fn end(&self, _: &str) {}
}

#[test]
fn pw_hold_shows_by_default_compares_the_normalised_value() {
    let host = Plain(Cell::new(0.25));
    assert!(host.shows("mix", 0.25));
    assert!(!host.shows("mix", 0.2500001));
    host.set("mix", 0.5);
    assert!(host.shows("mix", 0.5));
}
