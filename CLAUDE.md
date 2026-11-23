# CLAUDE.md — Panelwerk

Panelwerk is the shared look and conventions of Ian Dilley's audio plugins, MIT OR Apache-2.0, a separate library beside DSPWerkbank (`~/code/dspwerkbank`), which holds their shared DSP (Ian, 2026-11-10; Rödalog's `docs/spec.md` §1, "Family library: Panelwerk"). The plan, its steps and the open questions are in `docs/proposal.md`.

## Consumers

| Consumer | Where | What it uses | How it depends |
| --- | --- | --- | --- |
| **Rödalog** (Nord Lead 2X synth) | `~/code/roedalog`, branch `v3` | the text layer (step 1); the shared panel base, its Morph arcs, modulation rings and `Host` layered on top follow at its milestone P (step 3) | by pinned git tag, with a commented `[patch]` path entry in its `Cargo.toml`, as DSPWerkbank's consumers |
| **Kör** (Juno-60 chorus) | `~/code/koer`, `main` | the text layer; the panel layer, the test kit and held edits (`hold`), all v0.4.0 | the same |
| **Fläns** (Electric Mistress flanger) | `~/code/flaens` | the text layer; the panel layer, the test kit and held edits (`hold`), all v0.4.0 | the same |
| **Klinga** (diffuseur effect) | `~/code/klinga`, `main` | the text layer (value text, parsers, ID convention); its panel (M6) will take the panel layer and `panelwerk-nice` (W-2) | by pinned git tag (v0.4.0), with a commented `[patch]` path entry in its `Cargo.toml`, as the others |

**Four plugins depend on it, all pinning v0.4.0.** **A change must keep every consumer's displayed words and look as they are**, unless the change is meant to alter them (rule 6). Before committing a change to anything a consumer uses, run `./scripts/consumers.sh` (rule 14), which never touches a consumer's tree.

## Layout

- `crates/panelwerk-text`: the text and conventions. **Std only**: no dependencies (`tests/manifest.rs` checks).
- `crates/panelwerk-panel`: the panel. **egui**, at exactly the version every consumer's nice-plug-egui uses (`=0.37.0`, nice-plug-egui 0.6.0 on nice-plug 0.4.2; PW-13), and **serde_json** for the settings file (PW-8), and never nice-plug or nice-plug-egui, which each consumer vendors and patches for itself (`tests/manifest.rs` checks). It reaches the plugin only through the host traits the consumer implements (`ParamHost`, `zoom::ZoomHost`).
- `crates/panelwerk-nice` (task W-2b; Ian, 2026-11-23, PW-BUILT, point 1 (a)): the held edits' nice-plug side (the `ParamHost` adapter over `ParamPtr`, `shows` with plain values, the close-time hand-off), shared by the plugins instead of a copy in each. **The named exception to the two rules above, for this crate only:** it may depend on nice-plug (the panel crates still never do: `tests/manifest.rs` keeps checking them), and `unsafe` is allowed in the adapter's one module and nowhere else (rule 15). nice-plug at exactly `=0.4.2`, the version each consumer redirects to its own patched copy; the crate denies `unsafe_code` and allows it on `mod adapter` alone (`tests/manifest.rs` checks both, and the panel's and the test kit's dependency lists are exact, so neither takes nice-plug by accident).
- `crates/panelwerk-testkit`: dev-only (PW-9), for the panel's tests and each consumer's: a fake host and a headless window. egui (with its fonts) and the panel only.

## Commands

```bash
./scripts/coverage.sh          # all tests + 100 % coverage; --html for a report
cargo test --workspace --doc   # the API's examples
./scripts/consumers.sh         # rule 14: each consumer built against this checkout, its full hook; non-zero naming any that fail
./scripts/consumers.sh Klinga  # one consumer alone (names exact; one not in the list exits 2); its last line says a tag needs the run with none
./scripts/consumers.sh --pins  # each consumer's pinned tag; non-zero unless every consumer's pin is read and all are the same one (a pin it can't read fails, named)
```

`consumers.sh` works as DSPWerkbank's (its header says how): a clone of each consumer's HEAD under `target/consumers/`, this checkout patched in from a directory above, the consumer's tree never touched. A consumer is listed in it as it adopts Panelwerk.

## Rules

1. **Mechanisms and conventions here, each plugin's own choices in the plugin.** The library holds what the family shares: the words for a sync length, the form of value text, the palette, how a knob is drawn and dragged. What makes one plugin (its controls, parameter IDs, layout, sizes, zoom range, hints, its name, the order of its lists) is the consumer's, given as parameters. The library never names a consumer's parameter or knows a consumer's feature (Rödalog's Morph is drawn by Rödalog through the library's general arcs).
2. **The public API is what the consumers depend on: keep it small and stable, and document it.** `#![deny(missing_docs)]`; every public item has a doc comment, and every public type a doctest. A change to the public API is agreed with Ian before it's made and recorded in `CHANGELOG.md`. Dependencies as Layout says, checked by each crate's `tests/manifest.rs`; another needs Ian's agreement.
3. **Nothing here runs on the audio thread.** Value text, parsing and the panel run on the host's main or GUI thread; a consumer never calls the library from `process`. The settings file is read and written only on the GUI thread.
4. **Small steps, each through the hook.** Every commit and every merge passes the hook (`scripts/hooks/pre-commit`: all tests, 100 % coverage, doctests, rustdoc, clippy `-D warnings` native and `x86_64-pc-windows-gnu`, each step timed; `pre-merge-commit` runs the same for a merge). A commit of only Markdown skips the checks. Never bypass it. In a new clone, enable it with `git config core.hooksPath scripts/hooks`, and install `cargo install --locked cargo-nextest cargo-llvm-cov`.
5. **Determinism.** Same value, same text; same input, same drawing. No wall-clock time or randomness; the only time used is egui's input time.
6. **Words and looks are interface.** A shown name can be a saved key (Rödalog's presets save a list's choice by its name, its B-PRE-3), and every consumer's tests freeze its value text and its colours' contrast. So a change to a word, a value text's form, a colour or a control's geometry is agreed with Ian first, is a new minor version at least (see Versions), and its `CHANGELOG.md` entry names what changes on each consumer's panel.
7. **Strict test-driven development** (as on DSPWerkbank, Kör and Rödalog). Write tests before code, see them fail for the right reason, write the least code that passes, then refactor with the tests green. What moves here from a consumer brings its tests with it, green throughout. The lighter form (tests alongside the code, still to 100 %) for drawing-only code whose look Ian judges by eye, as Rödalog's rule 8; text, gestures and anything a consumer's behaviour rules name stay strict.
8. **Every threshold says where it came from.** In a comment by each test's limit: a consumer's rule and its number (for example Kör's B-PNL-9, 4.5:1, WCAG 2 AA), a published standard, or "as achieved", which needs Ian's agreement. A close margin between the limit and the measured value (a contrast just above 4.5:1, a text a point narrower than its room) is reported to Ian with its figure, so it's decided rather than accidental.
9. **Test coverage is 100 %** of lines, functions and regions in the four crates. Anything uncovered needs a very good reason agreed with Ian, listed in `scripts/coverage.sh`.
10. **Clean-room.** Ian gives his plugins away as donationware: no GPL code and no non-commercial content. Don't read or copy GPL code. The look is original: no manufacturer's artwork, names or logos (Clavia, Roland, Electro-Harmonix), as each consumer's own rules say.
11. **Ask Ian** when a boundary between library and consumer needs a design decision, rather than invent it, and record the answer here or in `docs/proposal.md`.
12. **Additions, off by default.** What a consumer needs is added as a new option, field or function whose default is today's behaviour, so every other consumer is unaffected even after it moves to the new version. A breaking change is the exception and needs Ian's agreement.
13. **One library change at a time**, each by its own agent, test first, through this hook. An agent working on a consumer never edits the library from inside that consumer's repository; it reports what it needs, and the change is scheduled here.
14. **The consumer check before every tag**: `scripts/consumers.sh` builds every consumer against this checkout and runs its full hook; a tag is cut only when every consumer passes, its words and look unchanged unless the change is meant to alter them (rule 6).
15. **`unsafe`**: none (`#![forbid(unsafe_code)]` in every crate). The one exception, once `panelwerk-nice` is built (Ian, 2026-11-23, PW-BUILT, point 1 (a)): the adapter's single module there, the same framework calls Kör and Fläns make in their own copies; no other module or crate.

## Tasks

The second review's tasks (Ian, 2026-11-22, REVIEW-2-FINDINGS) are in `docs/tasks-review-2.md`: read it for what is open and what comes next.

## Versions and the family update

Semantic versioning, one version for every crate, tags `vMAJOR.MINOR.PATCH` on `main`, cut only when the hook and the consumer check have passed; a tag is never moved or deleted. While the version is 0.x, a minor bump may break the API or change what a consumer shows (listed in `CHANGELOG.md`); a patch changes neither.

**One family update moves every consumer's pins together** (Ian, 2026-11-10): after a tag, one agent moves Rödalog's, Kör's, Fläns's and Klinga's pins to it, one consumer after another, each in its own commit through its own hook, and `./scripts/consumers.sh --pins` then shows one tag. The steps are in `docs/proposal.md` §5.

## Git

- Commit with Ian's noreply address, `116658505+DibbsDibblethwaite@users.noreply.github.com` (set in this repository).
- The GitHub repository, `DibbsDibblethwaite/panelwerk`, is Ian's to make, private unless he says otherwise; consumers name the library by that URL.

## Comments and tests (Ian, 2026-11-11)

Many tests and many comments are good. Only the cost of running tests (hook time) limits how many there should be, so never remove or discourage tests or comments merely for their number.

- **A comment that doesn't match the code is a defect,** worse than no comment. That includes doc comments, tolerance and limit notes, and As-built notes. Every reviewer checks that they match the code and the measured values.
- **Comments explain WHY** the code does what it does: the decision, the rejected alternative, the rule it serves. "What" comments are welcome too, as long as they're true.
- **A test is flagged** only if it is wrong, can't fail, or costs hook time out of proportion to what it guards.

Every code reviewer's brief includes this section.

### Guards against comment drift (Ian, 2026-11-11, COMMENT-DRIFT)

1. **Comments to re-read.** The hook prints, for information (it never fails a commit), every comment inside a function or block the change touched that wasn't itself edited (`scripts/comment-drift.sh`). The implementer's report and the reviewer's each say they re-read that list.
2. **Numbers live in one place.** A limit, size or count is a named constant. Its comment gives the source and the reason, and refers to the constant rather than copying the number.
3. **Examples that run.** A comment showing how to use something is a doctest, so the build checks it.
