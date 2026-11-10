# Panelwerk: a proposal (for Ian)

*2026-11-10. Ian's decision of the same day (Rödalog's `docs/spec.md` §1, "Family library: Panelwerk") is the brief: a library beside DSPWerkbank, in two layers, text and conventions (`std` only) and the panel (egui); consumers pin tags; every release runs all three consumers' checks; one family update moves the three pins together; Rödalog's extras layer on top. The four steps are agreed (§7). This proposal says what goes where, sketches each layer's API, and asks what is still open (§8). Step 1, the text layer, is built (v0.1.0, §9).*

## 1. What the three plugins have today

Read from Rödalog `v3` (`45cfd01`), Kör `main` (`e97fc99`) and Fläns `m6-2` (`78185db`).

### Framework versions

| | nice-plug | vendored | nice-plug-egui | egui |
| --- | --- | --- | --- | --- |
| Rödalog | 0.4.2 | `nice-plug` and `nice-plug-core`: Kör's two patches and Rödalog's 5 and 6 | 0.6.0 (egui-baseview 0.9.0) | **0.37.0**, `default_fonts` |
| Kör | 0.4.2 | `nice-plug`: its two patches | `=0.6.0` | **`=0.37.0`**, `default_fonts` |
| Fläns | 0.4.2 | `nice-plug`: Kör's two patches | none yet (`flaens-gui` is empty until M7; the plugin has no `editor` feature) | none yet; M7 brings nice-plug-egui 0.6.0, so 0.37.0 |

They match: `panelwerk-panel` takes `egui = "=0.37.0"`. egui itself is from crates.io in all three, so cargo unifies it with the consumer's. The panel layer must never depend on nice-plug or nice-plug-egui: each consumer vendors and patches its own, and two copies would conflict. Its manifest test checks both. **One consequence** (question PW-13): with egui pinned exactly here, moving any consumer to a newer nice-plug-egui (so a newer egui) becomes a family-wide framework upgrade.

### Tempo-sync names (FAM-1)

| | Its list, in its order | List IDs | Saved by |
| --- | --- | --- | --- |
| Rödalog | 4 bars, 2 bars, 1 bar, 1/2 dotted, 1/2, 1/4 dotted, 1/2 triplet, 1/4, 1/8 dotted, 1/4 triplet, 1/8, 1/16 dotted, 1/8 triplet, 1/16, 1/16 triplet, 1/32 (16, by length) | none | the state by index; presets **by the shown name** (B-PRE-3) |
| Kör | 1/1, 1/1., 1/1T, 1/2 … 1/32T (18) | `1`, `1d`, `1t` … `32t` | its list ID |
| Fläns | 16 bars, 8 bars, 4 bars, 2 bars, then Kör's 18 (22) | `16bar` … `2bar`, then Kör's | its list ID |

So FAM-1 renames Kör's and Fläns's dotted and triplet entries ("1/1." to "1/1 dotted", "1/4T" to "1/4 triplet"); their IDs stay. Kör has it as its task M6.0, not yet done. Two things the words alone don't settle: **"1 bar" and "1/1"** are different lengths outside 4/4 (Rödalog's bar follows the host's meter, its B-SYN-2 … 4; Kör's and Fläns's 1/1 is always four beats, and Fläns has bars too), question PW-2; and in Rödalog a name is a **saved key**, so the names are frozen once shared, PW-3.

nice-plug's `#[derive(Enum)]` takes each name as a literal in a `#[name = "…"]` attribute, so a consumer can't write `name = panelwerk::…` there. Either each consumer keeps its literals and a test proves they are Panelwerk's, or it implements nice-plug's four-function `Enum` trait by hand from Panelwerk's tables (PW-4).

### Value text

| | Rödalog (`params.rs`) | Kör and Fläns (`params.rs`, Fläns's from Kör's) |
| --- | --- | --- |
| Unit | inside the text ("+3.5 dB") | nice-plug's `with_unit` (" dB", " dBFS", " ms", " Hz", " %") |
| Percent | 0.1 % ("50.0 %"); bipolar "+12.5 %", no "-0.0" | whole % (nice-plug's formatter), Fläns's Colour 0.1 % |
| dB | "+3.5 dB", "0.0 dB", "-inf dB" at its floor (−60, rounded) | 0.1 dB, signed or not by parameter, no "-0.0"; no floor text |
| Hz | "632 Hz", "1.2 kHz" (filter); "0.03 Hz", "2.00 Hz", "45.5 Hz", "523 Hz" (LFO) | three figures: "0.450", "1.23", "11.7" Hz |
| Time | chosen by size: "0.45 ms", "3.5 ms", "350 ms", "1.00 s", "45.0 s"; "Off" at 0 for some | ms at a fixed number of places per parameter |
| Cents, semitones | "+3.0 cents", "+5 cents", "+7 st", "+1 oct" | none |
| Parsing | with or without the unit, a leading "+", "-inf", "k"/"kHz", "ms"/"s", "off" | with or without a trailing unit |

The shared conventions underneath are already the same: a space before the unit, ASCII "-" (so typed text reads back), no negative zero, a "+" above zero only on signed values, and Rödalog's rule that a format is chosen by the value *as rounded*, so text read back shows the same way. The number of places and the style per parameter differ, and are each plugin's choice (rule 1); PW-1 asks whether to keep that.

### The panels

| | Kör (`koer-gui`, 1 000 lines + 1 150 of tests) | Rödalog (`roedalog-gui`, 2 700 lines + tests) |
| --- | --- | --- |
| Layout | fixed rectangles from a `Control` table; the tests check them exactly | egui's flow layout: each widget allocates its size at the cursor |
| Host trait | `value`, `default_value`, `text`, `begin`, `set`, `end`, `zoom_chosen`, `unsupported_rate` | the same first six, and `text_at`, `real`, `real_morphed`, `step`, keyboard, Morph audition, presets, AVX2 |
| Palette | Rödalog's, copied value for value (B-PNL-9), "until a shared panel crate is made" | `widgets.rs` and `panel.rs` constants; amber for Morph |
| Knob | 72-point slot, radius 15; pointer in the text colour; rim red on hover | 80-point slot, radius 16; bipolar arcs from 12 o'clock; a dead lower half; Morph's amber arc outside; pointer white; arrow keys |
| Gestures | drag from where the button went down, 200 points, Ctrl/Shift ×10, double-click default; the "pending" value for 0.5 s | the same rules (B-PNL-2), the same pending window (V3a.21), generalised to Morph's second value |
| Lights, buttons | lit red with a ring, dimmed grey, off; outlined buttons | the same LEDs, LED rows; amber LED for Morph Assign |
| Lists | none | drop-downs (B-PNL-5) |
| Title | "Kör" white on the red strip | section titles white on the red strip |
| Help | a 26-point help line from the manual (B-PNL-8) | tooltips (B-PNL-3); a help strip planned for P (panel-v3 §1) |
| Zoom | 75 … 300 %, menu 75 … 300 % in 7 sizes, saved with the project and in `%APPDATA%\Koer\panel.json` | none yet; decided for P: 60 … 200 %, menu 75 … 200 % |
| Contrast | `palette::contrast` and `TEXT_ON`, ≥ 4.5:1 | ≥ 4.5:1 (B-PNL-4), its own test |

Kör's widgets are the simpler (step 2's source); Rödalog's are the same family with more on top (step 3).

## 2. What goes where

| What | Layer | From | Step |
| --- | --- | --- | --- |
| Sync length names and list IDs | text | Rödalog's words, Kör's IDs | 1 |
| Value text and parsing: %, dB, Hz, time, cents, semitones | text | Rödalog's and Kör's, as styles | 1 |
| List- and parameter-ID checks (lower-case ASCII, digits, `_`) | text | Kör's and Fläns's frozen-ID tests | 1 |
| Palette (Rödalog's colours under role names), `contrast`, the family's text-on-background pairs | panel | Kör's `palette.rs` | 2 |
| `ParamHost`: the six calls every panel needs | panel | both | 2 |
| Gestures: drag, fine drag, double-click, the pending value, toggle on click, one change one gesture | panel | Kör's `widgets.rs` | 2 |
| Knob drawing: travel, value arc, body, pointer; geometry from the consumer | panel | Kör's | 2 |
| Lights, button bodies, lit buttons, slot switches | panel | Kör's | 2 |
| Title strip; help line; zoom (range from the consumer), its menu; the settings file | panel | Kör's | 2 |
| A test kit: a fake host that records gestures, a headless frame runner, finding a control's rectangle | panel (dev-only) | both consumers' `tests/common` | 2 |
| Bipolar arcs, the dead half, extra arcs at other radii, LED rows, drop-down lists | panel, as off-by-default options | Rödalog's | 3 |
| Morph (its amber, second values, Assign), modulation rings (cyan), graphs, the preset browser, Rödalog's `Host` additions, arrow keys | **stay in Rödalog** | | 3 |
| Each plugin's controls, layout, hints, name, the mode logic (Kör's I/II/Manual), Fläns's circuit switch | **stay in each plugin** | | |
| Note lengths in beats, the song clock | **DSPWerkbank** (`song_clock`), and each engine | | |

## 3. The text layer's API (sketch)

```rust
// panelwerk_text::sync — names only; lengths in beats are the engine's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLength { Bars(u32), Note(u32), Dotted(u32), Triplet(u32) }

impl NoteLength {
    /// Every length the family names, longest first: 16, 8, 4, 2 and 1
    /// bars, then 1/1 … 1/32 each straight, dotted and triplet.
    pub const ALL: [NoteLength; 23];
    /// "4 bars", "1 bar", "1/4", "1/4 dotted", "1/4 triplet" (FAM-1).
    /// Panics for a length not in ALL.
    pub const fn name(self) -> &'static str;
    /// "4bar", "1bar", "4", "4d", "4t": Kör's and Fläns's list IDs.
    pub const fn id(self) -> &'static str;
}
/// For a hand-written nice-plug `Enum` (PW-4): the names of a consumer's
/// list, in its order, at compile time.
pub const fn names<const N: usize>(list: [NoteLength; N]) -> [&'static str; N];
pub const fn ids<const N: usize>(list: [NoteLength; N]) -> [&'static str; N];

// panelwerk_text::value — the number and its unit together.
pub fn percent(fraction: f32, places: usize) -> String;          // "50 %", "50.0 %"
pub fn percent_signed(fraction: f32, places: usize) -> String;   // "+12.5 %", "0.0 %", "-3.0 %"
pub struct Decibels { pub places: usize, pub signed: bool, pub unit: &'static str, pub floor: Option<f32> }
pub fn decibels(db: f32, style: &Decibels) -> String;            // "+3.5 dB", "-48.0 dBFS", "-inf dB"
pub enum HertzStyle { Figures(usize), Kilo, BySize }
pub fn hertz(hz: f32, style: HertzStyle) -> String;              // "1.23 Hz" | "632 Hz", "1.2 kHz" | "2.00 Hz", "45.5 Hz", "523 Hz"
pub fn by_size(x: f32, unit: &str) -> String;                    // Rödalog's "8.0 steps/s", "40 steps/s"
pub enum TimeStyle { Milliseconds(usize), BySize }
pub fn time(seconds: f32, style: TimeStyle) -> String;           // "1.66 ms" | "350 ms", "1.00 s"
pub fn cents(cents: f32, places: usize) -> String;               // "+3.0 cents"
pub fn semitones(st: i32) -> String;                             // "+7 st"
pub fn parse(text: &str, units: &[&str]) -> Option<f32>;         // with or without a unit, "+", any case
pub fn parse_decibels(text: &str, floor: Option<f32>) -> Option<f32>;   // "-inf"
pub fn parse_hertz(text: &str) -> Option<f32>;                   // "1.2k", "1.2 kHz"
pub fn parse_time(text: &str) -> Option<f32>;                    // "350 ms", "2.5 s", "2.5"

// panelwerk_text::ids
pub fn is_id(text: &str) -> bool;   // a parameter or list ID: lower-case ASCII, digits, "_"
```

Each style is a choice of today's texts, so every consumer can keep exactly what it shows (its value-text tests unchanged); the conventions in §1 are the library's and not optional. A consumer's builders then read, for example, `.with_value_to_string(Arc::new(|v| value::percent(v, 0)))` in place of `with_unit(" %")` and nice-plug's formatter; hosts show the same text.

## 4. The panel layer's API (sketch), and how Rödalog layers on it

```rust
pub use egui;                      // the consumer's plugin window takes this one

pub mod palette {
    // Rödalog's colours under role names (Kör's BACKGROUND is SECTION, its
    // HELP_BACKGROUND is PANEL):
    pub const PANEL: Color32;      // 18, 18, 20: around sections; the help line
    pub const SECTION: Color32;    // 38, 38, 43: where controls sit
    pub const BORDER: Color32; pub const HOVER: Color32; pub const TEXT: Color32;
    pub const DIMMED_TEXT: Color32; pub const CHOSEN: Color32; pub const CHOICE_TEXT: Color32;
    pub const RED: Color32; pub const BRIGHT_RED: Color32; pub const DIMMED_VALUE: Color32;
    pub const LIT: Color32; pub const LIT_RING: Color32; pub const DIMMED_LIT: Color32; pub const LIGHT_OFF: Color32;
    pub const TRACK: Color32; pub const KNOB: Color32; pub const RIM: Color32;
    pub const TITLE_STRIP: Color32; pub const TITLE_TEXT: Color32;
    /// Every text colour above on each background it is drawn on.
    pub const TEXT_ON: &[(Color32, Color32)];
    pub fn contrast(a: Color32, b: Color32) -> f32;     // WCAG 2, 1 … 21
    /// The pairs under 4.5:1 (WCAG 2 AA; Kör B-PNL-9, Rödalog B-PNL-4).
    pub fn unreadable(pairs: &[(Color32, Color32)]) -> Vec<(Color32, Color32, f32)>;
}

/// What every panel needs from its plugin, by parameter ID, normalised.
pub trait ParamHost {
    fn value(&self, id: &str) -> f32;
    fn default_value(&self, id: &str) -> f32;
    fn text(&self, id: &str) -> String;
    fn begin(&self, id: &str);
    fn set(&self, id: &str, normalized: f32);
    fn end(&self, id: &str);
}

pub mod gesture {
    pub const DRAG_RANGE: f32 = 200.0; pub const FINE: f32 = 10.0;   // Rödalog B-PNL-2, Kör B-PNL-3
    /// What a knob's gestures change: `param`, shown at `at` (0 … 1 of the
    /// travel), set through `to_param`; `default` for a double-click.
    pub struct Target<'a> { pub param: &'a str, pub at: f32, pub to_param: &'a dyn Fn(f32) -> f32, pub default: f32 }
    impl<'a> Target<'a> { pub fn param(host: &dyn ParamHost, ui: &Ui, id: &'a str) -> Self; }
    /// Drag and double-click on `response`, one gesture each; where a drag
    /// ended, so a caller can act on it (Rödalog's Morph snap).
    pub fn knob(ui: &Ui, host: &dyn ParamHost, response: &Response, target: &Target) -> Option<f32>;
    pub fn toggle_on_click(ui: &Ui, host: &dyn ParamHost, response: &Response, param: &str);
    pub fn set(ui: &Ui, host: &dyn ParamHost, param: &str, value: f32);       // remembered until shown
    pub fn set_once(ui: &Ui, host: &dyn ParamHost, param: &str, value: f32);  // begin, set, end
    pub fn latest(ui: &Ui, host: &dyn ParamHost, param: &str) -> f32;         // the pending value or the host's
}

pub mod knob {
    /// The consumer's sizes (Kör's 15-point knob, Rödalog's 16).
    pub struct Geometry { pub radius: f32, pub arc_radius: f32, pub arc_width: f32, pub pointer_width: f32 }
    /// A further arc at its own radius, between two positions of the travel:
    /// the library draws it, the consumer says what it means.
    pub struct Arc { pub radius: f32, pub from: f32, pub to: f32, pub width: f32, pub colour: Color32, pub dimmed: Color32 }
    #[derive(Default)]
    pub struct Look<'a> {
        pub value: f32, pub pointer: Option<f32>, pub hot: bool, pub dimmed: bool,
        pub bipolar: bool,            // step 3, off by default: the value arc from 12 o'clock
        pub dead_lower_half: bool,    // step 3, off by default: below 12 o'clock worn and grey
        pub arcs: &'a [Arc],          // step 3, none by default: drawn outside the value arc
    }
    pub fn angle(position: f32) -> f32;                    // 7:30 … 4:30, 270°
    pub fn paint(painter: &Painter, centre: Pos2, geometry: &Geometry, look: &Look);
    /// The whole control: its name above, the knob, its value text below,
    /// in `rect`, which the consumer chose (Kör's table, Rödalog's cursor).
    pub fn knob(ui: &Ui, host: &dyn ParamHost, rect: Rect, id: &str, label: &str, geometry: &Geometry, dimmed: bool) -> Response;
}

pub mod light   { pub enum Light { Off, Lit, DimmedLit, Custom { fill: Color32, ring: Option<Color32> } }
                  pub fn paint(painter: &Painter, at: Pos2, radius: f32, light: Light); }
pub mod button  { pub fn body(ui: &Ui, rect: Rect, hot: bool);
                  pub fn lit(ui: &Ui, rect: Rect, id: Id, label: &str, light: Light) -> Response;
                  pub fn switch(ui: &Ui, host: &dyn ParamHost, rect: Rect, param: &str, label: &str, button: Vec2, dimmed: bool) -> Response; }
pub mod title   { pub fn strip(ui: &Ui, rect: Rect, id: Id, text: &str) -> Response; }  // secondary click: the zoom menu
pub mod help    { pub const HEIGHT: f32 = 26.0; pub fn line(ui: &Ui, rect: Rect, hint: &str, value: Option<&str>); }
pub mod zoom    { pub struct Range { pub min: f32, pub max: f32, pub menu: &'static [f32] }
                  pub trait ZoomHost { fn zoom_chosen(&self, zoom: f32); }
                  pub fn clamp(zoom: f32, range: &Range) -> f32;
                  pub fn follow(ctx: &Context, host: &dyn ZoomHost, base: Vec2, range: &Range);
                  pub fn menu(ui: &Ui, at: Pos2, host: &dyn ZoomHost, range: &Range); }
pub mod settings { pub fn file_in(appdata: Option<OsString>, plugin_folder: &str) -> Option<PathBuf>;
                   pub fn read(path: &Path, range: &zoom::Range) -> Option<f32>;
                   pub fn write(path: &Path, zoom: f32) -> std::io::Result<()>; }
pub mod list    { /* step 3: Rödalog's drop-down (B-PNL-5), choices and current from the caller */ }
```

**How Rödalog layers its extras without the library knowing Morph.** Its `Host` becomes `pub trait Host: panelwerk_panel::ParamHost` with its own calls added; every shared widget takes `&dyn ParamHost`, which a `&dyn Host` gives. Its knob keeps its own allocation (the flow layout) and its own Morph logic, and hands the library positions and arcs:

```rust
fn knob_in(ui: &mut Ui, host: &dyn Host, id: &str, label: &str, width: f32) -> Response {
    let (rect, response) = allocate(ui, size, Id::unique(id), Sense::click_and_drag());   // Rödalog's own
    let x = host.value(id);
    let assigning = morph_id(id).filter(|_| host.morph_audition());
    let target = match assigning {
        // Morph Assign edits the second value, stored as d (B-MOR-2, 7).
        Some(m) => Target { param: m, at: second_value(host, id, m), to_param: &|p| normalised(p - x), default: NO_MORPH },
        None => Target::param(host, ui, id),
    };
    if let (Some(m), Some(end)) = (assigning, gesture::knob(ui, host, &response, &target)) {
        if (end - x).abs() < SNAP { set_once(ui, host, m, NO_MORPH) }       // Morph's snap, Rödalog's
    }
    roedalog_keys(ui, host, &response, &target);                           // arrow keys, Rödalog's
    let second = morph_id(id).filter(|&m| morph_of(host, m) != 0.0).map(|m| second_value(host, id, m));
    let amber = second.map(|s| knob::Arc { radius: 24.5, from: x, to: s, width: 2.5, colour: AMBER, dimmed: DIMMED_AMBER });
    // P's modulation ring would be one more Arc, cyan, further out.
    knob::paint(ui.painter(), centre(rect), &RODALOG_KNOB, &knob::Look {
        value: x, pointer: assigning.map(|m| second_value(host, id, m)),
        hot: response.hovered() || response.dragged(), dimmed: !is_active(host, id),
        bipolar: is_bipolar(id), dead_lower_half: lower_half_dimmed(host, id),
        arcs: amber.as_slice(),
    });
    // The label and the value text (amber in Assign) are Rödalog's to draw.
    response
}
```

The library knows "a further arc at a radius", "a bipolar knob" and "a dead half"; Rödalog knows they mean Morph, Mod Envelope on Amp and so on. The same for lights (`Light::Custom` for its amber LED) and lists (the drop-down takes a per-choice marker for the second waveform). Each of those additions is off by default (rule 12), so Kör and Fläns are unaffected.

## 5. The family update

1. **The change**, in Panelwerk, by its own agent (rule 13), test first, through the hook; its `CHANGELOG.md` entry says what changes on each consumer's panel, if anything (rule 6); an API change agreed with Ian (rule 2).
2. **The consumer check**: `./scripts/consumers.sh`, every consumer's full hook against this checkout, in clones (rule 14). A failure is fixed here, or the consumer's side is scheduled in that consumer, before the tag.
3. **The tag**, `vX.Y.Z` on `main`, pushed to GitHub.
4. **The family update**, one agent, the three consumers one after another (Rödalog on `v3`, Kör on `main`, Fläns on its current branch), in each:
   - the tag in `Cargo.toml` for both crates, then `cargo update -p panelwerk-text -p panelwerk-panel`;
   - its licence file regenerated (`scripts/licences.sh`, which lists Panelwerk's notice as it does DSPWerkbank's);
   - its full hook; a commit naming only those files (`Family update: Panelwerk vX.Y.Z`), never `git commit -a` (Kör's and Fläns's agents share one tree);
   - where the change shows on screen, a screenshot (each has a screenshot example) for Ian.
5. **The proof**: `./scripts/consumers.sh --pins` shows one tag for all three. If one consumer's hook fails at step 4, the others aren't held back from their commits, but the update isn't done until all three are on the tag; the failing one is fixed the same day or reported to Ian.

A consumer in a release freeze is the only reason to lag (PW-11).

## 6. How the tests prove consistency

In **Panelwerk**:

- **Frozen tables.** Every sync name and ID written out literally in a test, so any change to a word fails (rule 6); every value-text style at a table of values, including the edges (−0.0, rounding up into the next format, the floor), and each text read back to the value that shows the same.
- **The palette**: every pair in `TEXT_ON` at least 4.5:1 (WCAG 2 AA; Kör B-PNL-9, Rödalog B-PNL-4), and `contrast` against WCAG's own figures (black on white 21).
- **Widgets** headless, as Kör's tests are now (`egui::Context::run_ui`): each gesture is one host gesture, drags by distance, the pending value, the zoom's following, the help line's clipping; each option off draws as before.
- **The manifests**: the text layer std only; the panel egui `=0.37.0` only, never nice-plug.

In **each consumer** (written in its adoption step, in its own repository):

- **Names come from Panelwerk**: `fam_1_sync_names_are_panelwerks` — each variant's name in nice-plug's `Enum::variants()` equals `panelwerk_text::sync::NoteLength::name` of that variant's length, and its ID `NoteLength::id` (Kör, Fläns). By construction instead, if Ian prefers (PW-4).
- **Value text comes from Panelwerk**: each parameter builder calls a `panelwerk_text::value` function; its existing value-text tests stay and freeze what it shows.
- **No colour of its own outside its extras**: a test reads the GUI crate's sources (`include_str!`) and finds no `Color32::from_rgb` outside the one file that holds its own colours (Rödalog's amber and cyan; none in Kör or Fläns), and its own pairs pass `palette::unreadable`.
- **The pins**: `consumers.sh --pins` in Panelwerk, run after every family update.

And `consumers.sh` runs every consumer's tests before every tag, so a library change that would alter a consumer's words or look fails there first.

## 7. The four steps

Sizes in developer-days (a person's time). Agents have run far faster: SIMD-0 … 6, about 15 developer-days, took about a day.

| Step | What | Ends with | Size |
| --- | --- | --- | --- |
| **1. The text layer** | `sync` (names, IDs, the const tables), `value` (the styles in §3, the parsers), `ids`; frozen tables; `consumers.sh` and the licence notices; tag v0.1.0. Then the family update: **Kör** (its M6.0 becomes this: "1/1 dotted" …, IDs unchanged), **Fläns** (the same names, once `m6-2` is merged), **Rödalog** (its names unchanged; its formatters swapped, every value-text test unchanged) | every consumer's names and value text from Panelwerk, their tests green, one pin | library 1.5, adoptions 0.5 each: **3.0** |
| **2. Palette and basic widgets, from Kör's panel** | `palette`, `ParamHost`, `gesture`, `knob` (Kör's look, Rödalog's details if PW-6), `light`, `button`, `title`, `help`, `zoom`, `settings`, the test kit; Kör's panel tests moved with them, green throughout; v0.2.0. **Kör adopts**: its `palette.rs`, `widgets.rs`, `zoom.rs`, `settings.rs` replaced by the library, its layout, hints and mode logic kept | Kör's panel on Panelwerk, every Kör panel test green, a screenshot for Ian | library 3.0, Kör 1.0: **4.0** |
| **3. Rödalog's extras, Rödalog adopts** | the off-by-default options (bipolar, dead half, further arcs, custom lights, LED rows, the drop-down list with per-choice markers); v0.3.0. **Rödalog adopts**: its `Host` on `ParamHost`, its knob, switch and list on the shared ones, Morph drawn through arcs, its palette constants replaced | Rödalog's panel on Panelwerk, every `roedalog-gui` test green, a screenshot for Ian | library 2.0, Rödalog 2.5: **4.5** |
| **4. Fläns M7 on Panelwerk** | Fläns's M7 (3.0 days planned) builds on the shared panel rather than copying Kör's: M7.1 shrinks to the layout and its tests; anything it finds missing reported and added here first (rule 13) | Fläns's panel, M7's own acceptance | M7 itself about **2.5** (from 3.0), and **0.5** in the library for what it finds |

About **14.5 developer-days** in all, of which 2.5 is Fläns's M7 already planned. Steps 2 and 4 depend on step 1 only through the tag; step 3 can come after step 4 (PW-10).

## 8. Questions for Ian

Each with a recommendation; "as recommended" answers all.

- **PW-1 Value text: shared conventions, each plugin's style?** The conventions (§1) are already the same; the places and styles differ (Kör's whole %, Rödalog's 0.1 %; three-figure Hz against kHz). *Recommend:* the library fixes the conventions and offers today's styles, each plugin keeping exactly what it shows now; a single family style later only if you want one (it would change two plugins' texts).
- **PW-2 "1 bar" and "1/1".** Rödalog's bars follow the host's meter; Kör's and Fläns's 1/1 is four beats in any meter. *Recommend:* both names, kept distinct: "N bars" / "1 bar" always the host's bar, "1/1" always four beats; written into the family rule beside FAM-1.
- **PW-3 Names frozen.** Rödalog's presets save a list's choice by its shown name (B-PRE-3), so a renamed length would break its presets. *Recommend:* every name and ID is frozen by Panelwerk's tests, and a change is a breaking change needing your agreement (rule 6), as a parameter ID is.
- **PW-4 How a consumer's list takes the names.** (a) Keep `#[derive(Enum)]` and its literals, and a test that each equals Panelwerk's; or (b) implement nice-plug's `Enum` by hand (four functions) from `sync::names` and `sync::ids`, so the names are Panelwerk's by construction. *Recommend (a):* the derive keeps each list's IDs readable beside its variants, and the test is one line per list.
- **PW-5 One length type with DSPWerkbank?** Each engine has its own (`NoteValue`, `SyncRate`, Kör's `beats`); DSPWerkbank has `bar_beats`. *Recommend:* no; Panelwerk's `NoteLength` names lengths and nothing more, and each plugin maps its engine's type to it, as Fläns's `SyncRateParam::engine` does now. The two libraries stay independent.
- **PW-6 The knob's details.** Kör's pointer is in the text colour, and a dimmed knob's rim on hover is the dimmed-value grey; Rödalog's pointer is white (grey when dimmed), and a dimmed knob's rim on hover is the text colour. *Recommend:* Rödalog's details for the family (its look is the family's, Kör B-PNL-9), sizes per plugin; Kör's knob changes slightly, shown to you in a screenshot at step 2.
- **PW-7 Zoom ranges.** Kör 75 … 300 %; Rödalog's P decided 60 … 200 %; Kör found its 13-point text under 10 points below 75 %. *Recommend:* the range and the menu's sizes per plugin (rule 1); the mechanism shared. Whether Rödalog keeps 60 % is for P.
- **PW-8 The settings file** (`%APPDATA%\<plugin>\panel.json`, the last zoom, Kör's B-PNL-7). It reads JSON with `serde_json`. *Recommend:* into the panel layer with `serde_json` as its second dependency (MIT/Apache, already in every consumer's build through nice-plug's state), the folder name the consumer's.
- **PW-9 A shared test kit.** Kör and Rödalog each have a fake host and frame helpers. *Recommend:* a third, dev-only crate, `panelwerk-testkit` (as DSPWerkbank's), made in step 2; consumers take it as a dev-dependency.
- **PW-10 When Rödalog adopts (step 3).** Its panel is about to be rebuilt in milestone P (panel-v3, option B). *Recommend:* steps 1, 2 and 4 now; step 3's library half and Rödalog's adoption as P's first task, so P's prototype is built on the shared base rather than today's panel being moved and then rebuilt. Until then Rödalog takes only the text layer.
- **PW-11 A consumer in a release freeze.** Kör's next milestone is its 1.0 release candidate (M6). *Recommend:* lockstep always, except that a consumer you name as frozen may stay one version behind until its release, `consumers.sh --pins` reporting it.
- **PW-12 Licence and notices.** *Recommend:* MIT OR Apache-2.0, as DSPWerkbank; each consumer's `scripts/licences.sh` checks Panelwerk at its tag and carries its MIT notice, as it does DSPWerkbank's.
- **PW-13 egui pinned exactly.** With `egui = "=0.37.0"` here, no consumer can move to a newer nice-plug-egui alone. *Recommend:* accept it: a framework upgrade becomes a family one, a Panelwerk minor version moving egui with all three consumers' nice-plug-egui in one family update, each with its offline-render test and validator run as each project's rules say.
- **PW-14 Hints and the manual.** Kör reads its hints from its `manual.md` at test time. *Recommend:* the help line is shared, the hints and the test that reads the manual stay each plugin's (the manual is the plugin's).

### For Ian to do

- Make the private GitHub repository `DibbsDibblethwaite/panelwerk` and push `main` (`git remote add origin …; git push -u origin main`). Consumers name the library by that URL, and `consumers.sh` patches it.

## Answers (Ian, 2026-11-10, on the Decisions board)

Every question as recommended, with one change to PW-1:

- **PW-1, value text: one family format** (Ian's note): percentages to **1 decimal**; frequencies to **3 significant figures, then kHz** (from 1 kHz); **units added by the framework** (nice-plug's `with_unit`), not written into the text. So Kör and Fläns change their percentages (whole → 1 decimal) and frequencies (gain kHz); Rödalog changes how its units are attached. Open detail (PW-1a): a framework unit is one fixed string, so a frequency that switches Hz → kHz needs the unit in the text; see the board.
- **PW-1a, frequency units** (Ian, 2026-11-10): **frequencies write their own unit**, "440 Hz", "1.20 kHz" (three significant figures, kHz from 1 kHz as rounded), the one exception; every other kind takes its unit from the framework (`with_unit`). Built in step 1 (`value::hertz`).
- PW-2 … PW-14 as recommended: "N bars" (host's bar) and "1/1" (four beats) distinct; sync names and IDs frozen; literal names plus a test per list; no shared note-length type with DSPWerkbank; Rödalog's knob details, sizes per plugin (a screenshot for Ian before Kör's merge); zoom mechanism shared, ranges per plugin; the settings file in the panel layer (serde_json); a test-only `panelwerk-testkit`; steps 1, 2 and 4 now, step 3 as the first task of Rödalog's milestone P; pins move together, a frozen plugin may lag one version until its release; MIT OR Apache-2.0; egui pinned exactly; the help line shared, hints per plugin.
- PW-REPO: Ian created `DibbsDibblethwaite/panelwerk` (private); `main` pushed.

## 9. Step 1 as built (v0.1.0)

`panelwerk-text` v0.1.0, 2026-11-10. The API, exactly:

```rust
pub mod sync {
    pub enum NoteLength { Bars(u32), Note(u32), Dotted(u32), Triplet(u32) }   // Clone, Copy, Debug, PartialEq, Eq
    impl NoteLength {
        pub const ALL: [NoteLength; 23];
        pub const fn name(self) -> &'static str;          // panics for a length not in ALL
        pub const fn id(self) -> &'static str;            // the same
        pub fn from_name(name: &str) -> Option<Self>;     // exactly a shown name (a saved key)
        pub fn from_id(id: &str) -> Option<Self>;         // exactly a list ID
        pub fn parse(text: &str) -> Option<Self>;         // typed text
    }
    pub const fn names<const N: usize>(list: [NoteLength; N]) -> [&'static str; N];
    pub const fn ids<const N: usize>(list: [NoteLength; N]) -> [&'static str; N];
}
pub mod value {
    pub enum Sign { Unsigned, Signed }                    // Clone, Copy, Debug, PartialEq, Eq
    pub const FLOOR: &str = "-inf";
    pub fn fixed(x: f32, places: usize, sign: Sign) -> String;
    pub fn significant(x: f32, figures: usize) -> String;
    pub fn percent(fraction: f32, sign: Sign) -> String;
    pub fn decibels(db: f32, sign: Sign) -> String;
    pub fn decibels_floored(db: f32, sign: Sign, floor: f32) -> String;
    pub fn hertz(hz: f32) -> String;                      // with its unit (PW-1a)
    pub fn milliseconds(ms: f32) -> String;
    pub fn cents(cents: f32) -> String;
    pub fn integer(n: i32, sign: Sign) -> String;
    pub fn parse(text: &str, units: &[&str]) -> Option<f32>;
    pub fn parse_integer(text: &str, units: &[&str]) -> Option<i32>;
    pub fn parse_percent(text: &str) -> Option<f32>;      // the fraction
    pub fn parse_decibels(text: &str, floor: Option<f32>) -> Option<f32>;
    pub fn parse_hertz(text: &str) -> Option<f32>;
    pub fn parse_milliseconds(text: &str) -> Option<f32>;
}
pub mod ids { pub fn is_id(text: &str) -> bool; }
```

Changed from §3's sketch, by PW-1's one format: no per-plugin styles (`HertzStyle`, `TimeStyle`, `Decibels { places, unit }` gone); no unit in the text except frequencies'; `percent` takes a `Sign` in place of `percent_signed`; `by_size` is `significant`; times are in ms, the framework's unit; `fixed`, `integer`, `parse_integer`, `parse_percent`, `from_name`, `from_id` and `NoteLength::parse` added.

### The sync tables (frozen, PW-3)

| `NoteLength` | Name | ID | | `NoteLength` | Name | ID |
| --- | --- | --- | --- | --- | --- | --- |
| `Bars(16)` | 16 bars | `16bar` | | `Triplet(4)` | 1/4 triplet | `4t` |
| `Bars(8)` | 8 bars | `8bar` | | `Note(8)` | 1/8 | `8` |
| `Bars(4)` | 4 bars | `4bar` | | `Dotted(8)` | 1/8 dotted | `8d` |
| `Bars(2)` | 2 bars | `2bar` | | `Triplet(8)` | 1/8 triplet | `8t` |
| `Bars(1)` | 1 bar | `1bar` | | `Note(16)` | 1/16 | `16` |
| `Note(1)` | 1/1 | `1` | | `Dotted(16)` | 1/16 dotted | `16d` |
| `Dotted(1)` | 1/1 dotted | `1d` | | `Triplet(16)` | 1/16 triplet | `16t` |
| `Triplet(1)` | 1/1 triplet | `1t` | | `Note(32)` | 1/32 | `32` |
| `Note(2)` | 1/2 | `2` | | `Dotted(32)` | 1/32 dotted | `32d` |
| `Dotted(2)` | 1/2 dotted | `2d` | | `Triplet(32)` | 1/32 triplet | `32t` |
| `Triplet(2)` | 1/2 triplet | `2t` | | | | |
| `Note(4)` | 1/4 | `4` | | | | |
| `Dotted(4)` | 1/4 dotted | `4d` | | | | |

`ALL` is that order. `1bar` is the one new ID (Rödalog has no IDs, and neither Kör nor Fläns has 1 bar). Each consumer's list is these in its own order; `tests/sync.rs` writes out Rödalog's 16, Kör's 18 and Fläns's 22 as they show today, which are the tables their one-line tests (PW-4) compare with: `assert_eq!(SyncRate::variants(), names(LIST))` and `assert_eq!(SyncRate::ids(), Some(&ids(LIST)[..]))`. nice-plug's own parse of an enum's typed text takes a variant's exact name only; `NoteLength::parse` is for a consumer's panel, and takes a name in any case and spacing, "4 bar" or "1 bars", and Kör's former "1/4.", "1/4T" (and "1/4d"); a bare ID isn't taken ("4" could be 1/4 or 4 bars).

### The value formats, and what each consumer would show

Conventions throughout: ASCII "-", no negative zero, "+" above zero only when signed, rounding half away from zero, the format chosen by the value as rounded. The number alone, the consumer's `with_unit` adding the unit, except frequencies (PW-1a).

| Kind | Format | Rödalog | Kör | Fläns |
| --- | --- | --- | --- | --- |
| Percentage (PW-1) | one decimal: "50.0", "+12.5" | same text; unit moves to `with_unit`. Fixed: "-0.0 %" between −0.05 % and 0 (its bipolar and Morph %) now "0.0 %" | Mix, Width, Drive, Manual Depth "50 %" → "50.0 %" | Range, Mix "75 %" → "75.0 %"; Colour unchanged |
| dB | 0.1 dB, signed by parameter; "-inf" at a floor as rounded | same; fixed: Gain "-0.0 dB" between −0.05 and 0 now "0.0 dB" | unchanged | unchanged |
| Frequency (PW-1, 1a) | three figures, kHz from 1 kHz, own unit | Filter Frequency 20 … 99.9 Hz gains a decimal ("45 Hz" → "45.0 Hz"), 1 … 9.99 kHz a figure ("1.2 kHz" → "1.20 kHz"); LFO Rates below 1 Hz gain a figure ("0.45 Hz" → "0.450 Hz", "0.03 Hz" → "0.0300 Hz") | Manual Rate unchanged on screen ("0.513 Hz"); `with_unit(" Hz")` dropped | Rate below 0.1 Hz gains a figure ("0.045 Hz" → "0.0450 Hz"); `with_unit(" Hz")` dropped |
| Time (**proposed, PW-15**) | three figures, in ms | below 100 ms gains a figure ("0.45 ms" → "0.450 ms", "3.5 ms" → "3.50 ms", "45 ms" → "45.0 ms"); **from 1 s: "1.00 s" → "1000 ms", "45.0 s" → "45000 ms"**; a bare typed number is ms, not s | Fade In below 10 ms gains a figure ("5.0" → "5.00 ms"); Fade Out below 100 ms gains one ("45" → "45.0 ms"), from 1 s loses one ("1234" → "1230 ms"); Manual Delay loses one ("3.505" → "3.51 ms") | Fade In, Fade Out as Kör's |
| Cents | 0.1, signed | Fine Tune same; fixed: "+0.0 cents" between 0 and 0.05 now "0.0 cents" | none | none |
| Whole steps | "+7", "0", "-2" | semitones, octaves, Master Tune unchanged | none | none |
| Ring Mod Tune (`fixed`, 1 place, signed) | "+7.0" | "+0.0 st" at 0 → "0.0 st" | none | none |
| Steps per second (**proposed, PW-16**: `significant`, 3) | "0.500", "8.00", "40.0" | Arp Rate, Echo Rate gain a figure ("8.0 steps/s" → "8.00 steps/s", "40 steps/s" → "40.0 steps/s") | none | none |
| Angle (`fixed`, 0 places) | "180" | none | Manual Phase unchanged ("180°") | none |

Each consumer's own words stay its own (rule 1): Rödalog's "Off", "Exponential", "Linear", "Slow start"; Kör's and Fläns's Bypass. Typed text: each parser takes the number with or without its unit, any ASCII case, a leading "+"; frequencies "k", "kHz", "Hz"; times "ms", or "s" for seconds; levels "-inf" where the parameter has a floor. Kör's and Fläns's parsers took any letters after the number; Panelwerk's take only the kind's own units.

### Questions for Ian (step 1)

- **PW-15 Times.** PW-1a's answer gives a time the framework's one unit, so a time is always in ms. Rödalog's envelope times run to 45 s, which would read "45000 ms", and its times from 1 s read "1.00 s" today. *Recommend:* times as frequencies, the second exception: their own unit, three figures, **s from 1 s** ("350 ms", "1.00 s", "45.0 s"), which leaves Rödalog's long times as they are and Kör's and Fläns's Fade Out from 1 s reading "1.23 s". Built as asked for now (ms, framework unit, three figures, `milliseconds`); a change is a minor version (v0.2.0) before any consumer adopts. Either way, three figures changes Kör's and Fläns's fixed places as the table says (Kör's Manual Delay "3.505" → "3.51 ms").
- **PW-16 Rates in steps per second** (Rödalog's Arp and Echo Rates, "0.5", "8.0", "40" today, two figures). *Recommend:* three figures, as frequencies ("8.00 steps/s"); or keep two (`significant(x, 2)`), which the library offers as well.

### v0.1.0's follow-ups (Ian, 2026-11-10)

- **PW-15:** times are a second exception, writing their own unit like frequencies: "350 ms", then seconds from 1 s ("1.00 s", "45.0 s"), 3 significant figures (v0.2.0).
- **PW-16:** steps per second (Arp Rate) to 3 significant figures.
- **PW-API:** v0.1.0's API approved as built (rule 2).
