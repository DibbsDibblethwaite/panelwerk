//! Panelwerk's test kit (PW-9): what the panel's tests and each consumer's
//! panel tests share. Dev-only, never shipped.
//!
//! - [`FakeHost`]: a host that records every gesture and every zoom chosen,
//!   and shows what is set, or lags as a real host does, or passes a set on
//!   at its next process step as one of three modelled hosts does
//!   ([`Model`]).
//! - [`Window`]: a headless egui window run a pass at a time
//!   (`egui::Context::run_ui`), with clicks, drags and hovers, and what the
//!   last pass drew: shapes, texts, lights, the window's commands.
//!
//! ```
//! use panelwerk_panel::{egui, ParamHost};
//! use panelwerk_testkit::{FakeHost, Window};
//!
//! let host = FakeHost::new(&[("mix", 0.5)]);
//! let mut w = Window::new(egui::vec2(200.0, 100.0));
//! w.idle(&mut |ui| {
//!     ui.painter().text(egui::pos2(20.0, 20.0), egui::Align2::LEFT_TOP, host.text("mix"), panelwerk_panel::font(), egui::Color32::WHITE);
//! });
//! assert_eq!(w.texts_in(egui::Rect::EVERYTHING), ["mix=0.500"]);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use egui::epaint::Shape;
use egui::{
    Color32, Context, Event, FontId, FullOutput, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui, Vec2,
    ViewportCommand, ViewportId,
};
use panelwerk_panel::zoom::ZoomHost;
use panelwerk_panel::ParamHost;

/// A fake host: parameters by ID with their defaults, every gesture
/// recorded as `"begin id"`, `"set id"`, `"end id"`. A set value is shown at
/// once, rounded to its parameter's steps if it has them (as nice-plug's
/// stepped parameters are), unless the host is lagging, or is modelled
/// ([`FakeHost::modelled`]): then it is shown only as the [`Model`] passes
/// it on at a [`FakeHost::process`] step.
///
/// ```
/// use panelwerk_panel::ParamHost;
/// use panelwerk_testkit::FakeHost;
///
/// let host = FakeHost::new(&[("mode", 0.0)]).stepped(&[("mode", 4)]);
/// host.begin("mode");
/// host.set("mode", 0.3);
/// host.end("mode");
/// assert_eq!(host.get("mode"), 0.25);
/// assert_eq!(host.take(), ["begin mode", "set mode", "end mode"]);
/// ```
#[derive(Default)]
pub struct FakeHost {
    /// The values set, by ID; a parameter not here shows its default.
    pub values: RefCell<HashMap<String, f32>>,
    /// The gestures so far ([`FakeHost::take`] clears them).
    pub gestures: RefCell<Vec<String>>,
    /// Hold the values: a set is recorded but not shown (a host's lag).
    pub lagging: Cell<bool>,
    /// Every zoom the user chose, in order.
    pub zooms: RefCell<Vec<f32>>,
    defaults: Vec<(String, f32)>,
    steps: HashMap<String, usize>,
    model: Option<Model>,
    /// The modelled host's gestures open, by ID.
    open: RefCell<Vec<String>>,
    /// What the modelled host passes on at its next process step, in order.
    next: RefCell<Vec<(String, f32)>>,
}

/// What a host does with the panel's gestures before its next process step
/// (PW-HOLD), the three hosts Kör's and Fläns's B-PNL-11 and Rödalog's
/// B-PAR-11 name, as Rödalog's `tests/vst3_host/model.rs` models them at the
/// VST3 level. Why it matters: while audio runs, nice-plug's VST3 layer
/// doesn't apply a panel's set itself and reports the old value until the
/// host passes the change back, so what the host passes back, and when,
/// decides whether an edit made in one instant survives.
///
/// ```
/// use panelwerk_panel::ParamHost;
/// use panelwerk_testkit::{FakeHost, Model};
///
/// let host = FakeHost::new(&[("on", 0.0)]).modelled(Model::ReadBack);
/// host.begin("on");
/// host.set("on", 1.0);
/// host.end("on");
/// host.process();
/// assert_eq!(host.get("on"), 0.0, "read back at the end: the click undone");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// Passes each parameter's last set on at the next process step,
    /// whether or not its gesture has ended (Reaper).
    Echo,
    /// Passes the sets on as [`Model::Echo`] does, and at a gesture's end
    /// reads the plugin's value back and passes that on too, after them
    /// (reported of Cubase in Latch mode).
    ReadBack,
    /// Passes a parameter's last set on only if its gesture is still open at
    /// the next process step; a set outside a gesture is never passed on
    /// (reported of some VST3 hosts; Cubase likely).
    WhileOpen,
}

/// Every [`Model`], for a test that runs under each.
pub const MODELS: [Model; 3] = [Model::Echo, Model::ReadBack, Model::WhileOpen];

impl FakeHost {
    /// A host with these parameters and their defaults, normalised. Any
    /// other ID panics, so a test finds a panel naming a parameter that
    /// doesn't exist.
    pub fn new(defaults: &[(&str, f32)]) -> Self {
        let defaults = defaults.iter().map(|(id, v)| ((*id).to_owned(), *v)).collect();
        FakeHost { defaults, ..FakeHost::default() }
    }

    /// The same, the parameters named each with `n` steps (`n + 1` values,
    /// a list's or a switch's), so a set value is shown rounded to k / n.
    pub fn stepped(mut self, steps: &[(&str, usize)]) -> Self {
        self.steps.extend(steps.iter().map(|(id, n)| ((*id).to_owned(), *n)));
        self
    }

    /// The same, as a host of `model`: a set is shown only when the model
    /// passes it on at a [`FakeHost::process`] step (audio running). A host
    /// not modelled stands for one not processing: nice-plug applies a set
    /// at once then.
    pub fn modelled(mut self, model: Model) -> Self {
        self.model = Some(model);
        self
    }

    /// The modelled host's next process step: what it has gathered reaches
    /// the plugin, in order, each value rounded to its parameter's steps.
    pub fn process(&self) {
        for (id, v) in std::mem::take(&mut *self.next.borrow_mut()) {
            self.put(&id, self.rounded(&id, v));
        }
    }

    /// The same, with these values set (not as gestures).
    pub fn with(self, values: &[(&str, f32)]) -> Self {
        values.iter().for_each(|(id, v)| self.put(id, *v));
        self
    }

    /// Parameter `id`'s value.
    pub fn get(&self, id: &str) -> f32 {
        self.value(id)
    }

    /// Sets parameter `id` from outside the panel (automation, a preset).
    pub fn put(&self, id: &str, v: f32) {
        self.values.borrow_mut().insert(id.to_owned(), v);
    }

    /// The gestures recorded so far, cleared.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.gestures.borrow_mut())
    }

    /// `normalized` as parameter `id` holds it: rounded to its steps, if it
    /// has them.
    fn rounded(&self, id: &str, normalized: f32) -> f32 {
        self.steps.get(id).map_or(normalized, |&n| (normalized * n as f32).round() / n as f32)
    }
}

impl ParamHost for FakeHost {
    fn value(&self, id: &str) -> f32 {
        self.values.borrow().get(id).copied().unwrap_or_else(|| self.default_value(id))
    }
    fn default_value(&self, id: &str) -> f32 {
        self.defaults.iter().find(|d| d.0 == id).unwrap_or_else(|| panic!("no parameter {id}")).1
    }
    /// `"id=0.500"`: the ID and the value to three places.
    fn text(&self, id: &str) -> String {
        format!("{id}={:.3}", self.value(id))
    }
    fn begin(&self, id: &str) {
        self.gestures.borrow_mut().push(format!("begin {id}"));
        self.open.borrow_mut().push(id.to_owned());
    }
    fn set(&self, id: &str, normalized: f32) {
        self.gestures.borrow_mut().push(format!("set {id}"));
        match self.model {
            None if !self.lagging.get() => self.put(id, self.rounded(id, normalized)),
            None => {}
            Some(Model::WhileOpen) if !self.open.borrow().iter().any(|o| o == id) => {}
            Some(_) => {
                // The last value: a later one replaces one not yet passed on.
                let mut next = self.next.borrow_mut();
                next.retain(|(p, _)| p != id);
                next.push((id.to_owned(), normalized));
            }
        }
    }
    fn end(&self, id: &str) {
        self.gestures.borrow_mut().push(format!("end {id}"));
        let mut open = self.open.borrow_mut();
        if let Some(at) = open.iter().position(|o| o == id) {
            open.remove(at);
        }
        match self.model {
            Some(Model::ReadBack) => self.next.borrow_mut().push((id.to_owned(), self.value(id))),
            Some(Model::WhileOpen) => self.next.borrow_mut().retain(|(p, _)| p != id),
            _ => {}
        }
    }
    /// The plugin's value is `normalized` as its steps round it, as a
    /// consumer comparing plain values sees it.
    fn shows(&self, id: &str, normalized: f32) -> bool {
        self.value(id) == self.rounded(id, normalized)
    }
}

impl ZoomHost for FakeHost {
    fn zoom_chosen(&self, zoom: f32) {
        self.zooms.borrow_mut().push(zoom);
    }
}

/// What a test runs each pass: the panel, drawn into the window's `Ui`.
pub type Run<'a> = dyn FnMut(&mut Ui) + 'a;

/// A headless window: one egui context, its clock (a tenth of a second a
/// pass), its screen in points, and the last pass's output.
///
/// ```
/// use panelwerk_panel::egui::{self, Sense};
/// use panelwerk_testkit::Window;
///
/// let mut w = Window::new(egui::vec2(100.0, 100.0));
/// let mut clicks = 0;
/// let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(20.0, 20.0));
/// w.click(rect.center(), &mut |ui| {
///     clicks += ui.interact(rect, egui::Id::unique("b"), Sense::click()).clicked() as usize;
/// });
/// assert_eq!(clicks, 1);
/// ```
pub struct Window {
    /// The window's egui context: zoom, memory, a consumer's setup.
    pub ctx: Context,
    /// The input time of the last pass, in seconds.
    pub time: f64,
    /// The screen in points, as the host's window gives it.
    pub screen: Vec2,
    /// The last pass's output.
    pub last: Option<FullOutput>,
}

impl Window {
    /// A window `screen` points in size, no pass run yet.
    pub fn new(screen: Vec2) -> Self {
        Window { ctx: Context::default(), time: 0.0, screen, last: None }
    }

    /// One pass with `events` and `modifiers` held.
    pub fn pass(&mut self, mut events: Vec<Event>, modifiers: Modifiers, run: &mut Run) -> &FullOutput {
        self.time += 0.1;
        // egui 0.37 takes the modifiers as an event, not a field.
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| run(ui));
        output.textures_delta.clear();
        self.last.insert(output)
    }

    /// One pass with no input.
    pub fn idle(&mut self, run: &mut Run) -> &FullOutput {
        self.pass(vec![], Modifiers::NONE, run)
    }

    /// A primary click at `at`, then two passes for what it changed to show.
    pub fn click(&mut self, at: Pos2, run: &mut Run) {
        self.click_with(at, PointerButton::Primary, run);
    }

    /// A click of `button` at `at`, then two passes.
    pub fn click_with(&mut self, at: Pos2, button: PointerButton, run: &mut Run) {
        let press = |pressed| Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE };
        self.pass(vec![Event::PointerMoved(at)], Modifiers::NONE, run);
        self.pass(vec![press(true)], Modifiers::NONE, run);
        self.pass(vec![press(false)], Modifiers::NONE, run);
        self.idle(run);
        self.idle(run);
    }

    /// A double-click at `at`, then a pass.
    pub fn double_click(&mut self, at: Pos2, run: &mut Run) {
        let press = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.pass(vec![Event::PointerMoved(at)], Modifiers::NONE, run);
        for pressed in [true, false, true, false] {
            self.pass(vec![press(pressed)], Modifiers::NONE, run);
        }
        self.idle(run);
    }

    /// A drag from `from` by `moves` (a pass each), `modifiers` held, then a
    /// pass after the release.
    pub fn drag(&mut self, from: Pos2, moves: &[Vec2], modifiers: Modifiers, run: &mut Run) {
        let button = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers };
        self.pass(vec![Event::PointerMoved(from)], modifiers, run);
        self.pass(vec![button(from, true)], modifiers, run);
        let mut at = from;
        for m in moves {
            at += *m;
            self.pass(vec![Event::PointerMoved(at)], modifiers, run);
        }
        self.pass(vec![button(at, false)], modifiers, run);
        self.idle(run);
    }

    /// The pointer moved to `at`, and a pass more.
    pub fn hover(&mut self, at: Pos2, run: &mut Run) -> &FullOutput {
        self.pass(vec![Event::PointerMoved(at)], Modifiers::NONE, run);
        self.idle(run)
    }

    /// Where the widget with egui ID `id` was in the last pass, if it was.
    pub fn rect(&self, id: Id) -> Option<Rect> {
        self.ctx.read_response(id).map(|r| r.rect)
    }

    /// The last pass's output. Panics before the first pass.
    pub fn output(&self) -> &FullOutput {
        self.last.as_ref().expect("a pass was run")
    }

    /// Every shape the last pass drew, nested ones flattened.
    pub fn shapes(&self) -> Vec<Shape> {
        fn flatten(s: Shape, out: &mut Vec<Shape>) {
            match s {
                Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
                s => out.push(s),
            }
        }
        let mut out = vec![];
        for c in &self.output().shapes {
            flatten(c.shape.clone(), &mut out);
        }
        out
    }

    /// The texts drawn with their middle inside `area` in the last pass, in
    /// the order drawn.
    pub fn texts_in(&self, area: Rect) -> Vec<String> {
        self.shapes()
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) if area.contains(t.visual_bounding_rect().center()) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// The texts drawn wholly in `colour` in the last pass.
    pub fn texts_coloured(&self, colour: Color32) -> Vec<String> {
        self.shapes()
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) if t.galley.job.sections.iter().all(|s| s.format.color == colour) => {
                    Some(t.galley.text().to_owned())
                }
                _ => None,
            })
            .collect()
    }

    /// Whether a circle filled with `fill` (a light) was drawn with its
    /// centre inside `area`.
    pub fn circle_in(&self, area: Rect, fill: Color32) -> bool {
        self.shapes().into_iter().any(|s| matches!(s, Shape::Circle(c) if area.contains(c.center) && c.fill == fill))
    }

    /// The commands the last pass sent the window (a resize).
    pub fn commands(&self) -> Vec<ViewportCommand> {
        self.output().viewport_output.get(&ViewportId::ROOT).map_or(vec![], |v| v.commands.clone())
    }
}

/// How wide `text` is drawn in `font`, in points.
///
/// ```
/// let w = panelwerk_testkit::text_width("100.0 %", panelwerk_panel::font());
/// assert!(w > 30.0 && w < 72.0);
/// ```
pub fn text_width(text: &str, font: FontId) -> f32 {
    let ctx = Context::default();
    let mut w = 0.0;
    let mut out = ctx.run_ui(RawInput::default(), |ui| {
        w = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x;
    });
    out.textures_delta.clear();
    w
}
