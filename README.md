# Panelwerk

The shared look and conventions of Ian Dilley's audio plugins (Rödalog, Kör, Fläns), beside DSPWerkbank, which holds their shared DSP. MIT OR Apache-2.0.

- `panelwerk-text`: the text and conventions. Std only: tempo-sync length names in Rödalog's words ("4 bars", "1/4", "1/4 dotted", "1/4 triplet"), value text in one family format for %, dB, Hz, ms, cents and whole steps, and its parsing; the list-ID convention.
- `panelwerk-panel`: the panel, drawn with egui 0.37.0 (nice-plug-egui 0.6.0's): the palette and its contrast test, gestures, knobs, lights, buttons and switches, the red title strip, the help line, zoom and its menu, the settings file. Lists, bipolar and further arcs come in step 3.
- `panelwerk-testkit`: dev-only, for the panel's and its consumers' tests: a fake host that records gestures, a headless window.

`panelwerk-text` is built (v0.1.0, v0.2.0), and `panelwerk-panel`'s step 2 (v0.3.0). `docs/proposal.md` is the plan, §9 the text layer as built and §10 the panel's step 2. `CLAUDE.md` has the rules and the consumers.

## Licence

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your option.
