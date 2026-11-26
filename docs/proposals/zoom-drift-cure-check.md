# Fact-check: "The zoom drift: mechanism, options, cure and size"

**The fact-check of the approved design** (`zoom-drift-cure.md` beside it; Ian, 2026-11-26, card ZOOM-DRIFT-CURE, "As recommended: (a) in each of 1 to 7, and the split"), copied unchanged below this header from `~/code/proposals-pending/zoom-drift-cure-check.md`. Its findings were taken into the design (its "Changes after the fact-check"); where the two differ, the design records what was decided. The choices Ian made are listed at the head of the design. The scratch clones and probes it names were not kept in this repository.

Checked 2026-11-26 against the code, read-only. Scratch runs only, in `drift-check/` (clones of Panelwerk at v0.5.0 and Klinga at `m6-4` 87401d8, with their own target dirs). Nothing in any real tree was touched.

**Verdict: fit to put to Ian after corrections.** The diagnosis holds against the code. A+B is the right shape. The prototype's figures reproduce. But the prototype has one real defect that its own set-up changes hide: it loses drag steps. One risk is understated: a late answer is a runaway, not "one step". Four smaller statements are wrong. And one decision is missing: B has to go somewhere in Rödalog, and Rödalog does not use `panelwerk-nice`.

## Findings, most serious first

### 1. The prototype arms "the next frame is an answer" after every zoom change, including a followed drag step, so drag steps are lost (serious; reproduced)

- **What happens.** `follow` calls `ask(...)` whenever `before.zoom != now.zoom` (prototype `zoom.rs` diff, the `if before.zoom != now.zoom` branch). That includes the pass after a *followed* host size, where no request reaches the host:
  - The window already has the zoom's pixels, so egui-baseview's resize is a no-op.
  - Baseview's `WM_SIZE` returns early when the size is unchanged (`vendor/baseview/src/platform/win/window.rs:712`).
- **The effect.** The next host size after each followed drag step is taken as "the answer". The prototype then returns (`if before.zoom == now.zoom { return; }`). On the following pass `before.screen == now.screen`, so that size is never followed at all. The window stays at the drag's size with the panel drawn at the old zoom, permanently, until another resize.
- **The design's claim is wrong.** §2 (A) says: "On hosts that keep the shape, no request goes out during a drag, so nothing is swallowed". That is true of requests and false of the prototype's state. Under A+B it stays wrong: B has nothing outstanding and passes the size through, and A still swallows it.
- **Measured.** Scratch probe `probe_drag_second_step_on_the_answer_pass`: open, then a drag to 150 %, then a second step to 175 % on the pass after 150 % took effect. Result: zoom 1.5, window 1456×700, panel 1248×600, never corrected.
  - In a real drag, each host step makes a `WM_SIZE`, then an invalidate, then a frame, so steps landing on that pass are routine. The release step lands there often.
- **Why the suite didn't catch it.** The three Klinga set-up changes (`h.idle(1)` after open) move the single-step drags off the *opening's* answer pass. That is honest for the opening, since no user drags within a frame of opening. But it hides the same hazard after every zoom change, and no test drags in two steps.
- **Correction, verified.** Arm the answer only when the window must actually change size. In the zoom-change branch, if `near(now.screen, base)`, send `InnerSize`, tell a pending menu choice at once, and arm nothing. Otherwise `ask(...)`.
  - With that variant all 17 Klinga zoom tests pass (still with the diff's set-ups), and the drag probe ends with the window equal to the panel (1456×700 at 1.74996).
  - Add the two-step drag test to W-D1, and to Klinga/Rödalog.
  - Better still, once B exists, let B tell `follow` whether a request is outstanding (finding 4).

### 2. A late answer is a runaway, not "one drift step per answer" (serious; reproduced)

- **The claim.** §6 says that if a VST3 answer comes after the one-frame window, "the result would be one drift step per answer, still not saved".
- **What happens instead.** Each followed step changes the zoom. That arms `InnerSize`, which sends a new request, which brings a new late answer, and so on.
- **Measured.** Probe `probe_answer_one_pass_late_a_pixel_short` models the VST3 case: the child takes the asked size at once, as egui-baseview's own resize does, and the host's answer is one pixel short, one pass late. Result: 14 requests in 40 passes, zoom 1.5 down to 1.474, and every step passed to `zoom_chosen` through the default `zoom_followed` (so Kör and Fläns would save it).
- **Who is exposed.**
  - Under A+B, B catches this particular lag (its expiry is "before the frame after next").
  - A alone (Kör and Fläns until they wire B) and any answer later than B's window still run away.
- **Correction.** Reword the risk: "a host answering later than the window drifts as today, at half the rate". Under B, prefer pairing over counting frames (finding 4).

### 3. Missing decision: where B lives for Rödalog (moderate)

- **The facts.**
  - Rödalog does not depend on `panelwerk-nice`. Its `Cargo.toml:29-35` takes `panelwerk-panel` "for its zoom and settings file only", by Ian's PZ-RULES.
  - 683b805's `scripts/licences.sh` takes exactly "Panelwerk's text and panel layers".
  - Kör and Fläns don't use `panelwerk-nice` either (they pin only `panelwerk-panel`). Only Klinga does.
- **What follows.** "B in `panelwerk-nice`, wired by each consumer" means a new dependency for Rödalog, Kör and Fläns. For Rödalog that is outside Ian's approved scope, and it means licence-script work.
- **The alternative.** Rödalog already wraps both halves:
  - `Reported` wraps `HostCallbacks` (`crates/roedalog-plugin/src/window.rs:245-264, 306-309`).
  - `Open::set_size` wraps the handle (`window.rs:509`).
  - So B could be Rödalog's own code, with a copy in Klinga.
- **Correction.** Add this as a question for Ian, and size it into R-1 (licences, the dependency note).

### 4. Wrong or loose statements to fix (minor)

- **Two frames, not one, at a refused CLAP zoom.** Probe `probe_refusal_frames_at_the_refused_zoom` gives `[1.5, 1.5, 1.0, …]` for the passes after the click. The refused zoom is drawn on the pass where it took effect and on the next, because `follow`'s `set_zoom_factor(a.from)` only takes effect at the next `begin_pass` (egui-0.37.0 `context.rs:479-481`).
  - So §4's "One frame (about 16 ms)…" is wrong. The rule's "next frame" becomes "the second frame after the refusal".
  - It could be the next frame: B sees CLAP's `false` synchronously and could call `ctx.set_zoom_factor` before the next pass runs. That probably costs less than Q4's "further day".
- **"Before the first frame", not "before follow's second frame".** Face 1's explanation (§1, "When the answer lands before follow's second frame, no drift follows") should say *before the first frame*. At v0.5.0 `follow` reacts only to a change between passes (`zoom.rs:195-200` at v0.5.0, `before.screen != now.screen`). An answer that is already in place when the first pass looks is never followed. One that lands between the first and second passes is followed, and drifts. This is the precise reason (finding 8):
  - Rödalog's four whole-pixel cases and Klinga's `b_pnl_12_paint` adopt once: the show-time request is queued by patch 10 and answered inside the posted task before the first `WM_PAINT`.
  - The 90 % and 137.5 % cases run away: their zoom is set by a host drag after the open, so every answer lands between passes.
- **"Frame" is not a time bound.** Frames are on demand: `WM_PAINT` from the redraw timer or an invalidation (`window.rs:631-666`; `window_state.rs:136-146`).
  - When B swallows an answer, there is no `WM_SIZE` and so no frame. A's armed state can then stay open until the next input or repaint, possibly seconds later. The first host size after that is taken as an answer, so the first step of a drag after an idle pause is lost.
  - If a VST3 host never answers (refuses, or calls no `onSize`), B's "first `set_size` after the request" also stays armed until its frame-count expiry, and swallows a genuine size.
  - **Correction.** Bound B by request/answer pairing plus a time limit. Or post a marker after nice-plug's task: the posted queue is FIFO (`vendor/nice-plug/src/event_loop/windows.rs:89-104`), so when the marker arrives `resizeView` has returned. Say which in W-D2.
- **The header's commit.** It cites `pz` at 91c98cd. Head is 683b805, but `docs/behaviour/` is unchanged between them, so the rules quoted are current.

### 5. Where the diagnosis was checked and holds

| Claim | Evidence |
|---|---|
| Host-originated resize is guarded | `WindowHandle::resize` (`vendor/baseview/src/platform/win/window.rs:71-96`) sets `originate_host_resize`; `request_host_resize` returns early under it (`:306-311`) |
| Panel-originated resize is not guarded | egui-baseview's `draw` calls `WindowContext::resize` (egui-baseview-0.9.0 `window.rs:600-640`), which reaches `WindowState::resize` (`window_state.rs:93-103`) with no guard |
| `WM_SIZE` does `resized`, then `request_host_resize`; on an error it calls `resized(new)` again and reverts the window | `window.rs:704-746` (the "quirk" is real) |
| The show-time request | `window.rs:429-447`, `:750-760` |
| CLAP: the host's `request_resize` is synchronous inside `WM_SIZE`; `false` becomes `Err` | `vendor/nice-plug/src/wrapper/clap/wrapper.rs:2762-2785`. A nested `set_size` goes to `handle.set_size` (`:3053-3076`). The CLAP spec doesn't require it; on the main thread `true` means accepted |
| VST3: the request is only posted | Patch 10 (`vst3/view.rs:511-545`) returns `Ok` at once. `Task::RequestResize` later calls `WrapperView::request_resize`, then `IPlugFrame::resizeView` (`inner.rs:828-840`, `view.rs:253-290`), with the result discarded (`let _success`) |
| `onSize` always calls the editor handle's `set_size` when an editor is open | `view.rs:686-725` (and returns `kResultFalse` only on its error) |
| Klinga's paint-test frame calls `onSize` inside `resizeView` | `b_pnl_12_paint.rs:139-161`, m6-5 |
| The posted task runs before the next frame on a standard pump | Frames are `WM_PAINT`/`WM_TIMER`, the lowest-priority messages. Still supposed: a host that pumps with filters, or forces a synchronous paint, could reorder |
| (B) is possible without a patch | `HostMethods.callbacks: Box<dyn HostCallbacks>` is a public field of a public trait object (`vendor/nice-plug-core/src/editor.rs:38-48, 79-98`), passed to `Editor::spawn` (`:268-275`). Rödalog already replaces it. `EditorHandle::set_size` is wrappable (Rödalog `Open`, `panelwerk-nice` `Handle`). nice-plug-egui's `set_size` is only `window.resize` (nice-plug-egui-0.6.0 `editor.rs:332-338`), and `size()` comes from `resized` (`:124-136`), so a dropped answer leaves `getSize` at the panel's size, consistent with B-PNL-13/VST3-4 |
| A dropped VST3 answer is safe on Windows | A child larger than its parent's view is clipped by the parent; mouse coordinates are child-relative. The `WM_PAINT` loop such a child used to cause is cured by patch 13 (`window.rs:631-655`, `validate_window`), and Kör, Fläns and Klinga carry the same patch (KOER PATCH 6 at `vendor/baseview/src/platform/win/window.rs:638`). A *taller* answer leaves a gap under the panel, equal to the shortfall |
| (c) Button down | It would be read in the handle wrapper's `set_size`, on the GUI (main) thread. During a host's modal size loop the plugin's child gets no mouse messages, so egui can't know; only `GetAsyncKeyState` can. It reports *physical* buttons, so swapped buttons need `SM_SWAPBUTTON`. The flag must then be carried to the next frame's `zoom_followed` (plumbing, not in W-D2's text). It fails for a host that resizes the child only at `WM_EXITSIZEMOVE`, after release: drags would then not be saved, against B-PNL-41. Add this to Ian's check |
| (d) `zoom_followed` with a default is source-compatible | Implementors: Rödalog `Keeper` (`roedalog-plugin/src/zoom.rs:156`), Kör and Fläns `Frame<'_>` (`app.rs:138/143`), Klinga `Frame` (`app.rs:159`), and test fakes; none calls `zoom::choose` from the menu path. The behaviour change, "a menu choice told one frame later", hits each consumer's click-helper test, as the design says |

## What I ran and what I only read

**Ran (native, scratch):**
- Klinga `klinga-gui --test zoom --include-ignored`:
  - at v0.5.0: 12 pass, 5 fail (the five strict cases);
  - with the prototype: the five strict cases pass and the three named tests fail;
  - with the prototype and the Klinga set-up diff: 16/17, the scaling case failing as the design says.
- Klinga `klinga-plugin --test panel_zoom` with the prototype and the extra frame: 10/10.
- Panelwerk `cargo test --workspace` with the prototype diff: all green.
- Three probes of my own (drag second step, late answer, refusal frames), and a variant of `follow` (finding 1) that passes all 17 and fixes the drag probe.

**Not run:**
- Rödalog's, Kör's and Fläns's suites (heavy; the design's figures for them are unchecked).
- Panelwerk coverage.
- The Klinga plugin's other tests.
- Anything on Windows.
- B, which is not prototyped.

**Read only:** every framework path in finding 5; the rules (Rödalog B-PNL-41 to 44 on `pz`; Klinga B-PNL-6, 7 and 13 on `m6-5`. Klinga's B-PNL-11 is held edits and doesn't bear on this).

## Rules: does the design meet them?

- **B-PNL-43, "not the host's answer":**
  - met by A+B for answers paired within B's window;
  - not met for a later answer (finding 2);
  - the window keeping the zoom's size needs B. The design says so.
- **B-PNL-43, CLAP refusal:** met, except "the next frame". The design raises it (Q4), but under-counts by a frame (finding 4).
- **B-PNL-43, VST3 refusal:** met as approved: the panel keeps its size, and Klinga's refusal test is to be re-expressed. The design states this as Q2. Rödalog's held test `b_pnl_43_pz_built_5_…` (`panel_zoom_window.rs:434`) calls `keeper.zoom_chosen(0.8)` and expects the plugin layer to undo it, which A makes impossible by design. Re-expressing a held acceptance test needs its reviewer (rule 13). State that, not just "re-expressed".
- **B-PNL-41, "only the user's choice is saved":**
  - met only with B's button probe; A's default saves every host size;
  - the drag half rests on an unproved heuristic (Q1). Stated.
- **B-PNL-42, "CLAP fits again":** plausible as designed: zoom set at `set_parent`, then `follow` asks. It is not checked by any run.
- **Klinga B-PNL-6/7/13:** these rules are silent on answers today. The design's wording is offered as a proposal for Ian. Stated.

## Sizes

- **Comparable work.** About 4 developer-days is in line: PZ-c, "the window resizes", is 0.75, and the PZ total is 2.85.
- **Missing or thin:**
  - finding 1's fix and its tests (about 0.25);
  - B's frame/time expiry plumbing from the app into the handle;
  - Rödalog's B placement and dependency (finding 3, about 0.25 to 0.5);
  - PZ-g, folded into R-1's 0.75 though never sized;
  - the rules' history entries (Rödalog `history/panel-window.md`; Klinga);
  - Panelwerk's CHANGELOG, README and CLAUDE consumers table, and `scripts/consumers.sh` (named in W-D3, fine);
  - the family-update commits;
  - Kör's and Fläns's Windows runs when they move their pins (`cubase_k3_sizes`, `b_pnl_12_acceptance` comment on `follow`). Their released manuals ("the size you chose last") need no change under A's default;
  - sequencing with Klinga's in-flight `m6-7`.
- **Realistic total:** about 4.5 to 5.

## Alternatives

- **C (a vendored patch):** fairly judged. Note that it is exact for nesting hosts, where B's VST3 pairing is a heuristic window.
- **D (the panel's own grip):** fair.
- **Menu-only saving:** fairly offered (Q1). It answers point 2 only, not the drift.
- **Missed, and simpler:**
  - (i) arm the answer only when the window really changes size (finding 1, verified);
  - (ii) let B expose "request outstanding" to `follow` (for example a `ZoomHost::answering()` defaulting to A's frame rule), so consumers with B pair exactly instead of counting frames;
  - (iii) for VST3 expiry, a FIFO marker posted after nice-plug's task, or a time bound, instead of "the frame after next".
  - Comparing the host's size with the last requested size cannot by itself tell a one-pixel-short answer from a one-pixel drag step, so it is not a cure alone.

## Decisions that are Ian's

1. **The drag test (point 2).** Recommend accepting "main button down at the host's size", with `SM_SWAPBUTTON` handled. Add to his check that a drag in each host *is* saved, and fall back to menu-only saving if any host resizes after release.
2. **The VST3 refusal.** Recommend keeping B-PNL-43 as approved (no patch 14), with Klinga's refusal test re-expressed through its reviewer.
3. **Where B lives for Rödalog.** Recommend Rödalog's own `window.rs`, which already wraps both halves, rather than taking `panelwerk-nice` beyond PZ-RULES' "zoom and settings only". Klinga uses `panelwerk-nice`'s.
4. **The CLAP refusal's wording.** Recommend B undoing the zoom synchronously, so "the next frame" stands. Otherwise reword to "the second frame after".
5. **Kör and Fläns.** Recommend v0.6.0 at their next build (A only), knowing that a late answer still drifts there.
6. **The re-expressed held Rödalog CLAP-refusal test.** Recommend yes, reviewed under rule 13.
