# Changelog

All notable changes to Panelwerk. Versions follow `CLAUDE.md`, Versions; a change to what a consumer shows (a word, a colour, a size) lists what changes on each consumer's panel.

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
