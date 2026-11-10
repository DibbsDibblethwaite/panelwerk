# Panelwerk

The shared look and conventions of Ian Dilley's audio plugins (Rödalog, Kör, Fläns), beside DSPWerkbank, which holds their shared DSP. MIT OR Apache-2.0.

- `panelwerk-text`: the text and conventions. Std only: tempo-sync length names in Rödalog's words ("4 bars", "1/4", "1/4 dotted", "1/4 triplet"), value text for dB, %, Hz, ms and cents, list-ID conventions.
- `panelwerk-panel`: the panel, drawn with egui 0.37.0 (nice-plug-egui 0.6.0's): the palette and its contrast test, knobs and arcs, lights, switches, lists, the red title strip, the help line, the zoom menu.

Both are empty for now: `docs/proposal.md` is the plan, waiting on Ian. `CLAUDE.md` has the rules and the consumers.

## Licence

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your option.
