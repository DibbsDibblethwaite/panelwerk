# Changelog

All notable changes to Panelwerk. Versions follow `CLAUDE.md`, Versions; a change to what a consumer shows (a word, a colour, a size) lists what changes on each consumer's panel.

## Unreleased (v0.5.0)

*The second review's tasks (`docs/tasks-review-2.md`), approved by Ian on card REVIEW-2-FINDINGS (2026-11-22), each a minor-version change under rule 2.*

- **W-3, a host gesture can't be left open.** `gesture::knob` ends a drag's gesture when egui stops the drag even if the knob is no longer drawn then (a preset, automation or a mode change hid it mid-drag): the next `gesture::knob` or `gesture::toggle_on_click` drawn in the window ends it, through its own `host`, once. Before, the host's gesture stayed open (egui 0.37 reports no `drag_stopped` for a widget not drawn in the release pass: the new test showed it), and a host in Touch or Latch kept writing automation. `Held::frame` is `#[must_use]`: ignoring its answer leaves a waiting edit open until the next input. `Held::end` panics in debug builds if `Held::frame` has never run. What each consumer's panel shows: nothing changes. What each must change at its pin move: a call of `frame` whose answer is dropped no longer passes clippy `-D warnings` (`let _ =`, or ask for a repaint on `true`); Kör's and Fläns's apps use it already.
- **W-3b, a drag keeps setting the parameter it began with** (Ian, 2026-11-23, PW-BUILT, point 4 (a)). `gesture::knob` sends a drag's sets and its end to the parameter it began on, even if the same widget is given another parameter mid-drag (Kör's Rate knob drawing Sync Rate once Sync is on). Before, the host got `begin rate, set rate, set sync_rate, end rate`: a value for a parameter with no gesture open. What each consumer's panel shows: nothing changes. What each must change at its pin move: nothing.
- **W-6, a bad zoom range can't panic.** `zoom::Range::new(min, max, menu)`, a `const fn` that refuses (panics; in a `const`, fails to compile) unless `min <= max`, both finite, and the menu has at least one size, strictly smallest first. Before, an empty menu underflowed in `zoom::menu` (a panic in debug builds, a huge rectangle in release), and `min > max` or a NaN bound panicked in `f32::clamp`, which every zoom path calls, on the GUI thread. The fields stay public, so a struct literal still compiles unchecked (rule 12: no consumer breaks); `zoom::menu` now draws nothing for an empty menu rather than underflow. `zoom`'s module doc names the three framework patches a VST3 consumer needs for its size in Cubase (Rödalog's 10, 12, 13; W-12). What each consumer's panel shows: nothing changes. At its pin move a consumer may write its `RANGE` with `Range::new` (Kör's and Fläns's `zoom.rs`, one line each); nothing obliges it.

- **W-2b, `panelwerk-nice`: held edits' nice-plug side, once for the family** (Ian, 2026-11-23, PW-BUILT, point 1 (a)). A new crate, an addition (rule 12): no consumer depends on it yet, and none changes until its own task moves it on (Klinga's panel in M6; Kör and Fläns with the family's Cubase fixes; Rödalog at P). It holds what Kör's and Fläns's `app.rs` and `lib.rs` each carry, behaviour for behaviour:
  - `ParamMap` (a plugin's parameters by string ID, kept alive with the map) and `Adapter`, the panel's `ParamHost` over them and the window's `GuiContext`: unmodulated values, the framework's value text with its unit, gestures through `raw_begin_set_parameter`, `raw_set_parameter_normalized` and `raw_end_set_parameter`, and `shows` comparing plain values (`unmodulated_plain_value() == preview_plain(normalized)`).
  - `HeldEdits`, one window's held edits, owned by its app: `new`, `leaving`, `open`, and `frame(ui, draw)`, `ui` an `&mut egui::Ui` handed on to `draw(ui, host)` (so a panel taking `&mut Ui`, as Kör's and Fläns's do, can be drawn in it), which ends the edits whose values have arrived, draws the panel with a host whose begins, sets and ends go through `Held`, asks for another frame while an edit waits, and draws nothing before `open`.
  - `Leaving` and `Handle<H>`: the window's handle wrapped, every other call passed straight through, its drop and its `hide` taking the edits still open, in the order begun, as an `EndEdits` task given to `Ends`.
  - `EndEdits` (`ids`, `run`), and `queue_later(executor)`, `Ends` that gives each task to `execute_gui` from a scoped thread of its own, so nice-plug queues it for the main thread's next turn rather than running it inside the host's close. The plugin's `BackgroundTask` takes it with one `From<EndEdits>`, and its executor calls `run`.

  One difference from the copies, not in behaviour: they relied on the plugin outliving its window and its queued tasks for the parameter pointers to stay valid; `ParamMap` keeps an `Arc<dyn Params>` instead, so the crate's safe API is sound on its own (one reference count more). Dependencies: `nice-plug = "=0.4.2"` (no default features, `editor`), which every consumer redirects to its own patched copy, and the panel; its tests also use nice-plug's `vst3` wrapper, crates.io's copy, unpatched. `unsafe` in its `adapter` module only (rule 15's exception; `tests/manifest.rs` checks it, and that every block has a `// SAFETY:` comment). The panel's manifest check now also requires its one dev-dependency to be the test kit. What each consumer's panel shows: nothing changes. Adopting it: the crate's doc ("Adopting it", with its example); a consumer's own `Raw`, `Frame`'s held-edit routing, `Leaving`, `Handle`, `MainTask::EndEdits`'s loop and `queue_later` go, and its `Frame` keeps only its own traits (zoom, its extras) over the host `frame` gives: `edits.frame(ui, |ui, host| panel(ui, &Frame { host, … }))`.

## v0.4.0 (2026-11-14)

*API agreed by Ian, 2026-11-14 (rule 2): in outline on board card CUBASE-FAMILY-PLAN (D1, "All six as recommended"), and as built, with `frame`'s answer and `take_open` added, on card PW-API-3 ("Agreed: tag v0.4.0"). The outline is Kör's `docs/proposals/cubase-family.md` §1.*

PW-HOLD: held edits, the cure for hosts that undo an edit the panel makes in one instant (a click on a switch or a mode button, a double-click), moved from Rödalog v2's `preset.rs` (B-PAR-11, proved in Cubase LE 16 on 2026-11-14), keyed by parameter string ID. The consumers' rule is their B-PNL-11. An addition (rule 12): a plugin that doesn't wire it in is unchanged.

- `hold::Held`, with `new`, `begin`, `set`, `end`, `frame`, `close` and `take_open`. The panel keeps a host's gesture open until the plugin has the value last set in it, then ends it at the next `frame`. Gestures ended at one moment end together, once all their values have arrived. If its value never arrives, an edit is ended at the first `frame` after `gesture::PENDING_FOR`: the limit runs only while frames run, so a hidden window keeps a waiting edit open until it draws again or closes. `close` ends every open edit; `take_open` hands them to the caller instead, telling the host nothing, for a consumer to end them later from a safe place (below). If the host stops calling `process` for longer than `PENDING_FOR` without telling the plugin it has stopped processing (a stopped, idle Cubase, say), an edit is ended at the limit before its value arrives, and under a read-back or while-open host it is then lost (Rödalog v2's caveat; Ian found idle Cubase fine with v2 on 2026-11-14). A second begin continues an open or waiting gesture, so the host never sees one begun twice. Begins nest: an instant edit inside a drag doesn't cut the drag off. An end too many does nothing. `frame` returns whether an edit still waits.
- `ParamHost::shows(id, normalized)`: whether the plugin has the value, which is what a held edit waits for. It has a default (`value(id) == normalized`), so existing implementations are unchanged. A consumer on nice-plug should override it to compare plain values: without that, a stepped parameter's edit, and often a float's, ends only at `PENDING_FOR`, and a new edit of it before then (a second drag) joins the first in one host gesture, one undo step.
- `panelwerk-testkit`: `Model` (`Echo`, `ReadBack`, `WhileOpen`) and `MODELS`, the three hosts of B-PNL-11, after Rödalog's `tests/vst3_host/model.rs`; `FakeHost::modelled` and `FakeHost::process`, a set reaching the plugin only as the model passes it on at a process step; `FakeHost`'s `shows` compares as its steps round. A `FakeHost` not modelled is unchanged.

Wiring it in, for a consumer's editor (`hold`'s module doc has the example):

- The `App` that lives as long as the window owns a `Held`.
- The `ParamHost` given to the panel passes `begin`, `set` and `end` through `Held` to the adapter that talks to the framework. It should override `shows` with the plain comparison (`unmodulated_plain_value() == preview_plain(normalized)`), as Rödalog v2's does.
- `App::ui` calls `held.frame(&adapter, ui.input(|i| i.time))` before it draws the panel.
- The edits still open end when the window goes. Where the app is made in each `Editor::spawn`, as Kör's and Fläns's are, it is dropped with its window, so its `Drop` is the place; nice-plug-egui 0.6 itself keeps the app in its `EguiEditor` and never calls `editor_closed`. A consumer that makes one editor for the plugin's life needs something else dropped with the window.
- **Under nice-plug's VST3 wrapper, never call into the host from the app's `Drop`.** Call `held.take_open()` there and end the edits from a task queued to the main thread. The window is dropped inside the host's `IPlugView::removed`, while the wrapper holds its editor handle borrowed; an `endEdit` there, answered by a host with `setParamNormalized` while audio isn't processing, has the wrapper borrow it again in place, and the process aborts (found in Rödalog v3, CUBASE-1b, 2026-11-14). Rödalog v3 hands the taken edits, with the GUI context, to a `Plugin::BackgroundTask` that ends them through `param_setter()`. It gives the task to the `AsyncExecutor` from a thread of its own, joined at once: nice-plug runs a task given on the main thread in place, and queues one given from another thread for the main thread's next turn, after `removed()` has returned. It does the same under CLAP, whose window closes inside `gui.destroy`. `held.close(&adapter)` is for a caller not inside a framework's call (a standalone, a test).
- `frame`'s ends come from the panel's `ui`, inside the window's paint, not inside a host's call. As far as nice-plug 0.4.2 and nice-plug-egui 0.6.0 were read (not run), the wrapper holds no borrow then, and a host's answering `setParamNormalized` only asks the editor for a repaint. The app's own lock is held during `ui`, though, so a consumer must check that nothing the host's `endEdit` sets off calls back into its app (nice-plug-egui 0.6's `track_info_updated` locks it, and would deadlock). The wrapper does hold its editor handle mutably while it opens the window (`attached`), but a frame drawn then has no edit to end where the app, and its `Held`, are made in each `spawn`.
- A panel that draws only when asked must ask for another frame (`ui.ctx().request_repaint()`) while `frame` returns `true`. Otherwise a waiting edit ends only at the next input. Rödalog v3 needed exactly this. In the frame where a click makes an edit, `frame` (run before the panel draws) still returns `false`; egui draws again straight after a frame with input, and that frame sees the wait. An edit made without input (from a timer or a message) must ask for its next frame itself.

What each consumer's panel shows on adopting: nothing changes (no word, colour or geometry). Once wired in, an edit's end reaches the host at the first frame at which the plugin shows its value: a frame or more after a click or a double-click, usually a frame after a drag's release (with `shows` comparing plain values). Each is still one begin–end pair.

- **Kör**, **Fläns**: their tasks CUBASE-K2 and CUBASE-F2 wire it in.
- **Rödalog**: nothing (it uses only `panelwerk-text`; v3 keeps its own port of v2's code).

## v0.3.0 (2026-11-10)

*API approved by Ian, 2026-11-10 (rule 2, PW-API-2), as built; Kör's panel adopted it after Ian's look at the screenshots (PW-6-LOOK).*

Step 2 of `docs/proposal.md`: `panelwerk-panel`'s palette and basic widgets, from Kör's panel with Rödalog's knob details (PW-6), and `panelwerk-testkit` (PW-9). §10 has the API. `panelwerk-text` unchanged.

- `palette`: Rödalog's colours under role names, `contrast` (WCAG 2), `unreadable`, `MIN_CONTRAST` (4.5:1) and `TEXT_ON`, the shared widgets' text pairs, each at least 4.5:1 (the closest `DIMMED_TEXT` on `HOVER`, 4.74:1).
- `ParamHost`, `Control`, `TEXT_SIZE` (13), `font`, `colours`, `hot`.
- `gesture`: Kör's drag (200 points, ×10 finer with Ctrl or Shift, from where the button went down, stopping at the ends), double-click for the default, click to toggle, each one host gesture; the pending value for 0.5 s.
- `knob`: travel, value arc, body, rim and pointer, the sizes the consumer's (`Geometry`); `knob` (name above, value below) and `inline` (name beside). PW-6: the pointer white, grey while dimmed; the rim red under the mouse, the text colour while dimmed.
- `light`, `button`: lights lit, dimmed and off; button bodies, lit buttons, switches.
- `title::strip`, `help::line`, `zoom` (the range and menu sizes the consumer's, PW-7), `settings` (`panel.json` in the consumer's folder, with `serde_json`, PW-8).
- `panelwerk-testkit`: `FakeHost`, `Window`, `text_width`.
- Dependencies: `panelwerk-panel` takes `serde_json = "1"` beside `egui = "=0.37.0"` (PW-8, PW-13).

What each consumer's panel shows on adopting:

- **Kör**: a lit knob's pointer white (was 220, 220, 224); a dimmed knob's rim under the mouse in the text colour (was 120, 120, 128, no change from its rim at rest). Nothing else.
- **Rödalog**: nothing yet (step 3).
- **Fläns**: nothing (no panel until M7).

## v0.2.0 (2026-11-10)

- **PW-API** (Ian, 2026-11-10): v0.1.0's API approved as built (rule 2).
- **PW-15** (Ian, 2026-11-10): times write their own unit, as frequencies: three significant figures, ms below 1 s and s from 1 s as rounded ("350 ms", "1.00 s", "45.0 s"; 999.6 ms reads "1.00 s"). Breaking: `value::milliseconds` (number only) is now `value::time` (with its unit), and `parse_milliseconds` is `parse_time` (unchanged: "ms", "s", or a bare number as ms). A consumer's time parameter takes no `with_unit`.
- **PW-16** (Ian, 2026-11-10): steps per second to three significant figures, `significant(x, 3)`, as built.

What each consumer would show on adopting, for times (in place of v0.1.0's):

- **Rödalog**: below 100 ms gains a figure ("0.45 ms" → "0.450 ms", "3.5 ms" → "3.50 ms", "45 ms" → "45.0 ms"); 100 ms and up unchanged ("350 ms", "1.00 s", "45.0 s"); a bare typed time is ms, not s. Arp Rate and Echo Rate gain a figure ("8.0 steps/s" → "8.00 steps/s", "40 steps/s" → "40.0 steps/s").
- **Kör**: Fade In below 10 ms gains a figure ("5.0 ms" → "5.00 ms"); Fade Out below 100 ms gains one ("45 ms" → "45.0 ms") and from 1 s reads in s ("1234 ms" → "1.23 s"); Manual Delay loses one ("3.505 ms" → "3.51 ms").
- **Fläns**: Fade In and Fade Out as Kör's.

## v0.1.0 (2026-11-10)

Step 1 of `docs/proposal.md`: `panelwerk-text`, the text layer (§9 has the API, the tables and the formats). No consumer depends on it yet; each adopts it in its own family-update task.

- `sync`: `NoteLength`, its 23 lengths (`ALL`: 16, 8, 4, 2, 1 bars; 1/1 … 1/32, each straight, dotted, triplet), their names in Rödalog's words (FAM-1) and list IDs (Kör's and Fläns's; `1bar` new), frozen (PW-3); `names` and `ids` at compile time for each consumer's one-line test (PW-4); `from_name`, `from_id`, `parse` for typed text.
- `value`: the family's one format (PW-1): `percent` to one decimal, `decibels` to 0.1 dB with `decibels_floored`'s "-inf", `hertz` to three figures and kHz from 1 kHz with its own unit (PW-1a), `milliseconds` to three figures (PW-15 proposed), `cents`, `integer`, and `fixed` and `significant` beneath them; the number alone, the unit the framework's, except frequencies'; parsers for each.
- `ids`: `is_id`, the ID convention.

What each consumer's panel and host text would show on adopting (proposal §9's table):

- **Rödalog**: sync names unchanged. Units move to `with_unit`, the host text the same. Filter Frequency 20 … 99.9 Hz and 1 … 9.99 kHz gain a figure ("45.0 Hz", "1.20 kHz"); LFO Rates below 1 Hz gain one ("0.450 Hz"); times below 100 ms gain one ("3.50 ms") and from 1 s read in ms ("45000 ms") unless PW-15 says otherwise; Arp and Echo Rates gain a figure if PW-16 as recommended; Ring Mod Tune at 0 reads "0.0 st" (was "+0.0 st"). Negative zero fixed: "-0.0 %", "-0.0 dB" and "+0.0 cents" near zero now "0.0".
- **Kör**: Sync Rate's dotted and triplet names in words ("1/4 dotted", "1/4 triplet"; IDs unchanged; its M6.0). Percentages to one decimal ("50.0 %"). Fade In below 10 ms and Fade Out below 100 ms gain a figure, Fade Out from 1 s and Manual Delay lose one ("1230 ms", "3.51 ms"). dB, Manual Rate and Phase unchanged on screen.
- **Fläns**: sync names unchanged. Range and Mix to one decimal ("75.0 %"); Rate below 0.1 Hz gains a figure ("0.0450 Hz"); Fade In and Fade Out as Kör's.

## Unreleased

- The repository: the workspace, two empty crates (`panelwerk-text`, std only; `panelwerk-panel`, egui `=0.37.0`), the hook, the coverage and consumer-check scripts, the licences, and `docs/proposal.md` (Ian's decision of 2026-11-10, Rödalog's `spec.md` §1, "Family library: Panelwerk").
