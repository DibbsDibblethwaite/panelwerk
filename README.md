# Panelwerk

The shared look and conventions of Ian Dilley's audio plugins (Rödalog, Kör, Fläns, Klinga), beside DSPWerkbank, which holds their shared DSP. MIT OR Apache-2.0.

- `panelwerk-text`: the text and conventions. Std only: tempo-sync length names in Rödalog's words ("4 bars", "1/4", "1/4 dotted", "1/4 triplet"), value text in one family format for %, dB, Hz, ms, cents and whole steps, and its parsing; the list-ID convention.
- `panelwerk-panel`: the panel, drawn with egui 0.37.0 (nice-plug-egui 0.6.0's): the palette and its contrast test, gestures, knobs, lights, buttons and switches, the red title strip, the help line, zoom and its menu (the host's answer to the panel's own resize request never taken for a new zoom, v0.6.0), the settings file, held edits (a host's gesture kept open until the plugin has the value). Lists, bipolar and further arcs come in step 3.
- `panelwerk-nice`: held edits' nice-plug side, for a plugin on nice-plug 0.4.2: the panel's `ParamHost` over the plugin's parameters (`shows` comparing plain values), a window's held edits each frame, and the edits still open as the window closes or hides handed to the main thread; and the zoom's window part (v0.6.0): the host's callbacks and the window's handle wrapped, so the host's answer to the panel's resize request is kept off the window, a refusal is reported to the zoom, and a size given with the main button held (read by the plugin) is a drag. The one crate here that depends on nice-plug, and the one with `unsafe` (its adapter module only).
- `panelwerk-testkit`: dev-only, for the panel's and its consumers' tests: a fake host that records gestures (and can model three hosts' handling of them), a headless window (with an optional display scale, v0.6.0).

Built so far: `panelwerk-text` (v0.1.0, v0.2.0), `panelwerk-panel`'s step 2 (v0.3.0) and its held edits (v0.4.0), `panelwerk-nice` (v0.5.0, W-2b, with the second review's other changes), and the zoom drift's cure, in `panelwerk-panel`'s zoom and `panelwerk-nice`'s window part (v0.6.0, `docs/tasks-v0.6.md`). `docs/proposal.md` is the plan, §9 the text layer as built, §10 the panel's step 2 and §12 where v0.4.0's API is. `CHANGELOG.md` says what each version changed. `CLAUDE.md` has the rules and the consumers.

## Licence

Licensed under either of the Apache License, Version 2.0 (`LICENSE-APACHE`) or the MIT licence (`LICENSE-MIT`), at your option.
