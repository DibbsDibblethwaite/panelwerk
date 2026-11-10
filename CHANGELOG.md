# Changelog

All notable changes to Panelwerk. Versions follow `CLAUDE.md`, Versions; a change to what a consumer shows (a word, a colour, a size) lists what changes on each consumer's panel.

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
