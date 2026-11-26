# The zoom drift: mechanism, options, cure and size

**Approved (Ian, 2026-11-26, 18:16, card ZOOM-DRIFT-CURE: "As recommended: (a) in each of 1 to 7, and the split", no note).** This is the design as approved, copied unchanged below this header from `~/code/proposals-pending/zoom-drift-cure.md`; its independent fact-check is beside it, `zoom-drift-cure-check.md`, and where the two differ this design (its "Changes after the fact-check") records what it was corrected to. The tasks are in `docs/tasks-v0.6.md`. The choices, the card's seven points (its numbering; the design's D1 to D6 are points 1 to 6):

1. **(a)** A size the host gives while the main mouse button is held is a drag, and is remembered for new instances (physical buttons, swapped buttons handled); fallback **(b)**, remember only menu choices, for a host that turns out to resize only after release.
2. **(a)** A VST3 host refusing a menu size: the approved rule stands; the panel keeps its new size and the host shows the part that fits (no framework patch); Klinga's test changes to match.
3. **(a)** Rödalog's part of the cure (the window part, (B)) lives in Rödalog's own window code, not `panelwerk-nice`.
4. **(a)** A CLAP host refusing a menu size: the plugin undoes it at once, so the very next frame shows the old size.
5. **(a)** Kör and Fläns take the shared part ((A)) at their next build; (B) only if a drift is seen in them.
6. **(a)** The two refusal tests (Rödalog's held one and Klinga's) are rewritten for the new order, each checked by a reviewer.
7. **(a)** Klinga's three notes for Panelwerk as its first user are recorded in Panelwerk's tasks for a later release, not v0.6.0.

**The split: yes**, as §5's table (W-D1, W-D3, W-D2 here in series; R-1 to R-3 in Rödalog; K-1 in Klinga; the Windows runs; Ian's check; Kör and Fläns at their next build). The prototype diffs this design names were scratch files, not kept in this repository.

For Ian. 2026-11-26. Research and a scratch prototype only; nothing in any repository was changed. The prototype's diffs are beside this file:
- `prototype-panelwerk.diff`: Panelwerk's `zoom.rs` with the fact-check's corrections, plus one test given an extra pass.
- `prototype-klinga-tests.diff`: Klinga test set-ups given extra passes, and the checker's three probes, for the experiment only.

The approved rules are on Rödalog `pz` at 683b805 (`docs/behaviour/` unchanged since 91c98cd): B-PNL-41 "Only the user's choice is saved", B-PNL-42 "CLAP fits again", and B-PNL-43's "A CLAP host that refuses" and "Not the host's answer to the panel's own request". B-PNL-43's Windows test says the panel's *size* stays the zoom's and the host's view "shows the part that fits", so the cure has to keep the **window** at the panel's size, not only the zoom.

## Changes after the fact-check (2026-11-26)

Each finding of `drift-check/drift-cure-check.md` was checked again natively, in my scratch clones.

1. **Drag steps were lost.** Reproduced with the checker's two-step drag probe against my first prototype: zoom 1.5, window 1456 × 700, panel 1248 × 600, never corrected. Fixed as the checker proposed: when the window already has the new zoom's size, no answer is awaited. Re-run: the window equals the panel (1456 × 700 at 1.74996). The 17 original Klinga zoom tests behave as before (16 pass; the scaling case fails as described below). The probe as written still fails, but only on its exact `1.75` comparison: the zoom is worked from a whole-pixel height. §2 and §4 are corrected, and a two-step drag test is added to W-D1.
2. **A late answer is a runaway, not one step.** Reproduced: a host answering one pixel short, one pass late, made 14 requests in 40 passes, zoom 1.5 down to 1.474, each step saved through the default. Today's code does 20 in 40, so it is the drift at about two-thirds of today's rate.
   - I also tried keeping (A) armed until the window moves or 0.25 s passes. That catches the late answer (1 request, zoom kept), but it swallows real drags soon after the opening: three Panelwerk and three Klinga tests fail. So late answers are (B)'s to catch, and (A) alone (Kör and Fläns) still drifts on them. §2, §4 and §6 are reworded.
3. **Frames are not a time bound.** Agreed.
   - (A)'s awaited answer now also expires by egui's input time, 0.25 s, which Panelwerk's rule 5 allows. Prototyped; nothing breaks.
   - (B) pairs each request with the next host size within 250 ms of wall-clock time, in plugin code.
   - Consumers with (B) tell `follow` so; `follow` then awaits nothing, since (B) keeps answers off the window.
   - Chosen: the time limit, not a marker posted after nice-plug's task. A marker would close the window as soon as `resizeView` returns, so a host answering asynchronously would run away again. The cost of the limit is one genuine host size swallowed if it comes within 250 ms of a request the host never answered. Section 4.
4. **Where (B) lives for Rödalog.** Confirmed: Rödalog wraps the host's callbacks (`Reported`, `window.rs` at 683b805) and the handle's `set_size` (`Open`), and doesn't use `panelwerk-nice`. It is now a decision for Ian (D3), recommended in Rödalog's own `window.rs`.
5. **The CLAP refusal is drawn on two frames.** Reproduced: `[1.5, 1.5, 1.0, …]`. Designed so that (B) sees CLAP's `false` at once and puts the zoom back before the next frame, so the approved "the next frame" stands for Rödalog and Klinga. Without (B), two frames.
6. **Face 1.** Corrected: a window adopts the host's view once when the answer lands **before the first frame** (v0.5.0 follows only a change between frames). The 90 % and 137.5 % cases run away because a host's drag set their zoom after the open, so each answer lands between frames.
7. **Button down.** Stated as the checker says (§2, §6, D1): read with `GetAsyncKeyState` in `set_size` on the GUI thread; physical buttons, so swapped buttons need handling; it fails for a host that resizes only after release. Fallback: save menu choices only.
8. **Held refusal tests.** Rödalog's `panel_zoom_window.rs` `b_pnl_43_pz_built_5_…` calls the plugin's "chosen" directly, an order the cure removes. It and Klinga's refusal test are re-expressed with a reviewer (rule 13). Header corrected to 683b805.
9. **Sizes.** Table redone (§5): 4.85 developer-days, 5.35 with Kör and Fläns.

**Disputed:** none of substance. One note on finding 1: the checker's "the drag probe ends with the window equal to the panel" is right, but the probe's own assertion (`zoom == 1.75` exactly) still fails, as above. A test written for W-D1 should compare sizes in pixels.

## 1. The mechanism, exactly

**The pieces.** The host's view is a window. The panel is a baseview child window inside it. egui-baseview draws a frame on demand (a repaint or input), and after it resizes that child itself if the zoom changed or `follow` sent `InnerSize`. Panelwerk's `follow` runs once per frame and sees only egui's screen size in points and the zoom.

**Two kinds of resize in baseview** (`vendor/baseview/src/platform/win/window.rs`):
- **Host-originated:** `set_size` → `baseview::Window::resize`. It sets the guard `resize_host_originated`, calls `SetWindowPos`, and gets a synchronous `WM_SIZE` → `resized`; the request back to the host is suppressed. An unchanged size returns early: no `WM_SIZE`, no frame.
- **Panel-originated:** egui-baseview → `WindowContext::resize`, with no guard → `WM_SIZE` → `resized` → `request_host_resize` → `HostCallbacks::request_resize`.
  - **VST3:** patch 10 posts `Task::RequestResize` and returns `Ok`. The task later calls `IPlugFrame::resizeView`, and the host calls `onSize` inside it (Klinga's and Kör's test frames do; patch 10's comment says a host does). The result is discarded.
  - **CLAP:** the host's `request_resize` is called synchronously inside that `WM_SIZE`. On `false`, baseview puts the window back (calling `resized` once more with the *new* size, a quirk). The next frame sees the old size.
  - **The show-time request:** at `WM_SHOWWINDOW`, with no `resized` before it. This is patch 10's queued request at open.

| Case | Calls, in order | What v0.5.0's `follow` does |
|---|---|---|
| (i) Corner drag | VST3: `checkSizeConstraint` (nice-plug keeps 380:137), then `onSize` and `set_size`. CLAP: `adjust_size`, then `set_size`. Per mouse step (supposed) | The zoom for the height, then `InnerSize`. On a shape-keeping host that is normally the window's exact pixels, so no request goes out |
| (ii) Menu | The zoom takes effect next frame, `InnerSize`, the child resized, a request | Asks once |
| (iii) The host answers | VST3: the posted task, `resizeView`, then a nested `onSize` and `set_size`, normally before the next frame (posted messages come before paint and timer; supposed for each host's pump). CLAP: a nested `set_size`, if any | **Same size:** nothing. **Different:** the child takes it. If it landed before the first frame, it is adopted once (face 1). If it landed between frames, it is read as a drag: a new zoom, saved, asked again, answered short again: the drift (face 2), one request per frame, starving frames (face 3) |
| (iv) The host's own layout | `set_size` with nothing outstanding | Followed and saved |
| (v) Refusal | VST3: `resizeView` false; the child is already new, the view old, the panel cut. CLAP: false; the window is put back | VST3: kept and saved. CLAP: the new zoom in the old window |
| (vi) Scaling change | Embedded: no `WM_DPICHANGED` reaches a child; the host resizes the view in new pixels (supposed) | Like a drag to zoom × the scales' ratio: right on screen, wrong when saved |

**Information that exists:**
- The guard flag, inside baseview only.
- Each request with its size, and CLAP's verdict synchronously: to whoever wraps the `HostCallbacks` given to `Editor::spawn`. No patch is needed; Rödalog's `Reported` already does.
- Every host size, exact answers included: to the handle's `set_size` wrapper (Rödalog `Open`, Klinga `Measured`, Kör's and Fläns's own).
- What the window became: `follow` sees it on the next frame, whenever that comes.

**Information that does not exist:**
- In egui, anything about a drag versus the host's own layout. Only `GetAsyncKeyState` in `set_size` can tell (physical buttons; useless if a host resizes after release).
- A VST3 refusal.
- An embedded window's new DPI.

**The coordinator's Windows evidence** is face 1 (Klinga `b_pnl_12_paint`: panel (1040, 499) in view (1040, 499); Rödalog's four whole-pixel cases, all answered before the first frame), face 2 (Rödalog 90 %: 99 requests; Klinga 137.5 % taller: 1430 × 688 climbing) and face 3 (8 frames for 20 moves, against the limit of 10).

## 2. Design options

**(A) Panelwerk's `follow` (prototyped, corrected).**
- **When it awaits an answer:** after the opening frame, and after a zoom change *when the window must change size*. On the next frame, within 0.25 s of egui's time, whatever the window is counts as the host's answer: never a new zoom, never asked again.
- **A menu choice** is told to the plugin on that frame, unless the window is back at its old pixels (CLAP refused). Then the zoom goes back and nothing is told.
- **Any other host size** is the host's own. It is followed and passed to `ZoomHost::zoom_followed`, whose default calls today's `zoom_chosen`.
- **Two further additions,** both defaulted:
  - `ZoomHost::answers_paired()`, default false. When true (a consumer has B), `follow` awaits nothing.
  - `zoom::refused(ctx)`, for B to call on CLAP's `false`.
- **Corner drags:** followed every step, verified with the two-step probe.
- **Limits:** it cannot keep the window at the panel's size (face 1 remains), cannot catch an answer later than the next frame (a drift at about two-thirds of today's rate), and cannot tell a drag from a layout.
- **Kind of change:** an API addition and a behaviour change, Panelwerk v0.6.0, agreed with Ian (rules 2 and 12).

**(B) The plugin's window code.**
- **What it wraps:** the `HostCallbacks` (each request) and `set_size` (each host size).
- **The answer:** the first `set_size` within 250 ms after a request, or nested in a CLAP request. It is **not applied**: the child keeps the panel's size, and the host's view shows what fits (B-PNL-43, and B-PNL-10's premise).
- **Other host sizes** pass through, with "main button down" recorded for `zoom_followed`.
- **CLAP `false`:** calls `zoom::refused(ctx)` at once, so the next frame is at the old zoom.
- **Where:** Rödalog's own `window.rs` (D3). For Klinga, `panelwerk-nice`, which it already uses, or its own `panel.rs`. Kör and Fläns: optional.
- **The time limit** is wall-clock, GUI thread, plugin code (not the engine), with an injectable clock for tests.
- **Rejected:** counting frames (frames come on demand), and a FIFO marker after nice-plug's task (it closes at `resizeView`'s return, so asynchronous answers would run away).

**(C) A vendored patch.** Not needed for points 1, 2, 5 and 6. The only thing it adds is VST3 refusal detection (patch 14, about 20 lines × 4 copies, about 0.5 d), which Ian's approved B-PNL-43 doesn't ask for.

**(D) No host following; a grip drawn by the panel.** It breaks the corner drag the approved rules and released manuals promise, and is about 2.5 to 3 d. Not recommended.

**Tests each turns green** (headless figures measured, Windows ones reasoned):

| Test | A | B | A+B |
|---|---|---|---|
| Klinga headless: 1 px short, 100 px short, 1 px taller, menu refused | green | red (no handle in the model) | green |
| Klinga headless: scaling change, pixels kept | red with the window settled | red | red (models a case I found no real path for) |
| Two-step drag (probe) | green (sizes) | — | green |
| Late answer, one frame (probe) | red: 14 requests | green (supposed: paired within 250 ms) | green (supposed) |
| Klinga Windows: 1 px or 100 px short | red | green (supposed) | green (supposed) |
| Klinga Windows: VST3 refusal, panel equal to view | red | red | red (re-expressed to B-PNL-43, D2) |
| Rödalog B-PNL-10's short, taller and 90 % cases, Klinga `b_pnl_12_*` | runaway gone, face 1 red | green (supposed) | green (supposed) |

## 3. The prototype

- **Set-up:** clones in `drift/` (Panelwerk `main` = v0.5.0 plus docs; Klinga `m6-4`; Rödalog `pz`; Kör, Fläns `main`), each consumer's commented `[patch]` path lines enabled, `cargo test --offline`, scratch target dirs.
- **Klinga zoom tests at v0.5.0:** 12 pass, 5 fail (the strict ones).
- **Klinga with the corrected prototype:**
  - The five strict cases pass.
  - The three tests that resize on the window's second frame need one idle frame in their set-up.
  - The scaling case fails once the window has settled (it only passed by timing).
  - `klinga-plugin`'s `panel_zoom` passes with one more frame in its click helper, since a choice is now told one frame later.
  - Panelwerk's workspace passes with one set-up frame.
  - The three probes behave as listed.
- **Earlier prototype, not re-run after the fact-check corrections:** Rödalog's gui and plugin zoom tests (12, 13, 8 of 8) passed. Kör and Fläns failed only their copy of the second-frame test.
- **Not prototyped:** (B).

## 4. Recommendation: (A) in Panelwerk, (B) in each plugin's window code

**What each case does under the cure:**
- **(i) Corner drag:**
  - Each step is followed; the window is asked back to the shape if the host ignored it.
  - It is saved when the main button was down at the host's size; otherwise it is treated as (iv).
  - Kör and Fläns without (B): saved as today.
- **(ii) Menu:**
  - Asked once.
  - Told to the plugin on the frame after it took effect, unless refused.
- **(iii) An answer** (after a menu choice, at open, after a fit or a snap):
  - **With (B):**
    - The window keeps the panel's size (cut, or a gap under a taller answer), the zoom stays, nothing is asked again, nothing is saved by it.
    - The three faces become: **face 1:** the panel stays larger than the view, B-PNL-10's premise; **face 2:** at most one request per zoom; **face 3:** no request load.
  - **Without (B)** (Kör, Fläns):
    - The window takes the answer, the zoom stays.
    - A late answer still drifts at about two-thirds of today's rate.
- **(iv) The host's own layout:**
  - Followed on screen and kept in the project.
  - Never written to the file (B-PNL-41).
- **(v) Refusal:**
  - **CLAP with (B):** the zoom is back at the next frame, and neither the project nor the file takes it.
  - **CLAP without (B):** the same, but the refused zoom is drawn on two frames.
  - **VST3:** as B-PNL-43 (the panel keeps its new size).
- **(vi) Scaling change:** embedded, it arrives as (iv): not saved to the file; the project takes zoom × the ratio. Unresolved (§6).
- **Point 6 (CLAP fits again):** Rödalog sets the fit's zoom at `set_parent`; `follow` asks for it as one of its own sizes, never as a choice.

**Proposed wording** (for Ian; nothing written):
- **Panelwerk, `zoom`:** "A size the host gives on the frame after the panel asked for one (its opening size, a new zoom's size), within a quarter second, is the host's answer: never a new zoom, never asked again. A plugin that pairs requests and answers itself says so, and then the panel awaits nothing. A menu choice is told to the plugin once the window has taken it; if the window is back at its old size, or the plugin reports a refusal, the zoom goes back and nothing is told. Any other size the host gives is the host's own: the zoom follows it."
- **Rödalog B-PNL-43,** under "Not the host's answer": "The panel's window keeps the zoom's size. A size the host gives within a quarter second after the panel's request, or inside it, is its answer; later ones are the host's own (task PZ-CURE)." **B-PNL-41**'s point 2 clause: "a drag is a host's own size given while the mouse's main button is held; any other is the host's layout."
- **Klinga B-PNL-6, 7, 13:** the same sentences. B-PNL-7: "only a menu size or a corner drag is written to the file". The VST3 refusal as Rödalog's B-PNL-43.

## 5. The work, in developer-days

| # | Repository | Task | Ends with | Size | After |
|---|---|---|---|---|---|
| W-D1 | Panelwerk | (A): the corrected `follow`, the time bound, the deferred choice, `zoom_followed`, `answers_paired`, `zoom::refused`. Tests first: Klinga's five strict models, two-step drag, late answer, refusal frames, opening answer, menu told once | hook, 100 % coverage | 1.0 | — |
| W-D3 | Panelwerk | `panelwerk-nice` (B) for Klinga: pairing, 250 ms limit with an injectable clock, button-probe hook, refusal call. Native stand-in tests for both formats | hook | 0.75 | W-D1 (one library change at a time) |
| W-D2 | Panelwerk | CHANGELOG, README, Consumers table, consumer check, tag **v0.6.0** | `consumers.sh` green, tag | 0.35 | W-D3 |
| R-1 | Rödalog `pz` | Pin v0.6.0 (family-update commit); (B) in `window.rs` (`Reported`, `Open::set_size`), `Keeper::zoom_followed`; native tests | hook | 0.75 | W-D2 |
| R-2 | Rödalog `pz` | PZ-g: the CLAP refusal (the held test re-expressed, reviewer, rule 13) and the CLAP fit again | held tests un-ignored | 0.5 | R-1 |
| R-3 | Rödalog | Rules and their history entries | docs | 0.15 | with R-1 |
| K-1 | Klinga | Pin, wire `panelwerk-nice` (B), `zoom_followed`, strict tests un-ignored, set-up idle frames and the click frame (reviewer), the refusal test to B-PNL-43's reading (reviewer), rules and history. After the builder now on `m6-7` merges (one task per lane) | hook | 0.75 | W-D2 |
| WIN | Rödalog, Klinga | Windows runs: `b_pnl_10_acceptance`, `pz_vst3_zoom`, `cubase_k3_sizes --ignored`, `b_pnl_12_*`; one sweep each | green | 0.5 | R-2, K-1 |
| IAN | — | Owner's check (below) | his card | 0.1 | WIN |
| KF | Kör, Fläns | At their next build: pin v0.6.0, one test's set-up, their Windows runs (`cubase_k3_sizes`, `b_pnl_12_acceptance`); (B) optional, +0.5 each | hooks, runs | 0.25 each | W-D2 |

- **Core total 4.85 d; 5.35 with Kör and Fläns** (agents usually beat this by two to three times).
- **In parallel:** R-1 to R-3 (lane 2) and K-1 (lane 3) once the tag exists; Kör and Fläns whenever they next build. Panelwerk's three tasks run in series.

**Ian's check (about 25 minutes):**
- In Reaper CLAP, Reaper VST3 and Cubase, at 125 %:
  - 80 % and 160 % from the menu;
  - drag the corner both ways;
  - three opens and closes;
  - a project saved and reopened.
- The settings file then holds the last menu or drag zoom: this proves the button test in each host.
- Dock in Reaper by its menu: the file is unchanged.
- Cubase: watch the panel for a minute.
- A second monitor at another scaling, if he has one.

## 6. Risks, and what I could not determine

- **Supposed, unproved on Windows:**
  - That each host's answer comes inside `resizeView` or within 250 ms. A later one drifts, at about two-thirds of today's rate, under (A) alone, and once per late answer under (B).
  - That hosts keep the shape during a drag.
  - That the main button reads as down at each host's `set_size` during a corner drag. If a host resizes only after release, its drags aren't saved: menu-only saving is then the fallback.
- **No record exists of a corner drag tried in any host** for any family plugin.
- **Embedded scaling change:** what Windows tells the child; whether a reopen on the new monitor opens too large; Klinga's headless scaling case models a path I found nowhere. Ian's two-monitor try decides.
- **The 250 ms limit swallows one genuine host size** if it comes that soon after an unanswered request.
- **A menu choice reaches the plugin a frame after it is drawn;** a window closed within that frame loses it.

## Decisions for Ian

- **D1. Telling a drag from the host's own resize.**
  - (a) A size the host gives while your main mouse button is held is a drag and is remembered for new instances; if a host turns out to resize only after release, fall back to (b).
  - (b) Remember only sizes chosen from the menu.
  - *Recommended: (a).*
- **D2. A VST3 host refusing a menu size.**
  - (a) Keep the approved rule: the panel keeps its new size and the host shows the part that fits; Klinga's test is changed to match.
  - (b) A small framework patch, about half a day in four copies, so it goes back to the old size as on CLAP.
  - *Recommended: (a).*
- **D3. Where Rödalog's part of the cure lives.**
  - (a) In Rödalog's own window code, which already intercepts both calls involved (about half a day).
  - (b) Take the family's shared framework helper library, widening the earlier "zoom and settings only" and the licence check.
  - *Recommended: (a).*
- **D4. A CLAP host refusing a menu size.**
  - (a) The plugin undoes the size at once, so the very next frame shows the old size, as approved.
  - (b) Reword the rule to "the second frame after".
  - *Recommended: (a); it is in the plan.*
- **D5. Kör and Fläns.**
  - (a) Take the shared part at their next build: it stops the drift from hosts that answer at once, not from hosts that answer late.
  - (b) Also add the window-code part then, half a day each, which stops both and keeps the panel's size.
  - *Recommended: (a), with (b) if anyone sees a drift in them.*
- **D6. Two refusal tests rewritten** for the new order (the plugin is told a size only once the window has taken it): Rödalog's held one and Klinga's, each checked by a reviewer.
  - (a) Yes.
  - (b) No.
  - *Recommended: (a).*
