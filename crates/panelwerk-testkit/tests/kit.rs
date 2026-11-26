//! The test kit itself: the fake host's record and lag, and the window's
//! input and readings.

use egui::{pos2, vec2, Align2, Color32, Id, Modifiers, PointerButton, Rect, Sense, Stroke, ViewportCommand};
use panelwerk_panel::{font, ParamHost};
use panelwerk_testkit::{text_width, FakeHost, Model, Window, MODELS};

#[test]
fn the_fake_host_records_gestures_and_shows_values() {
    let host = FakeHost::new(&[("mix", 0.5), ("on", 0.0)]).stepped(&[("on", 1)]).with(&[("mix", 0.2)]);
    assert_eq!((host.get("mix"), host.default_value("mix"), host.text("mix")), (0.2, 0.5, "mix=0.200".into()));
    assert!(host.take().is_empty(), "with() isn't a gesture");
    host.begin("on");
    host.set("on", 0.7);
    host.end("on");
    assert_eq!(host.get("on"), 1.0, "rounded to its steps");
    assert_eq!(host.take(), ["begin on", "set on", "end on"]);
    assert!(host.take().is_empty());
    host.lagging.set(true);
    host.set("mix", 0.9);
    assert_eq!(host.get("mix"), 0.2, "lagging: not shown");
}

#[test]
#[should_panic(expected = "no parameter nothing")]
fn the_fake_host_has_only_its_parameters() {
    FakeHost::new(&[]).get("nothing");
}

#[test]
fn the_window_clicks_drags_and_reads_what_was_drawn() {
    let button = Rect::from_min_size(pos2(10.0, 10.0), vec2(40.0, 20.0));
    let (mut clicks, mut seconds, mut doubles, mut dragged, mut hovered) = (0, 0, 0, 0.0, false);
    let mut w = Window::new(vec2(200.0, 100.0));
    let mut run = |ui: &mut egui::Ui| {
        let r = ui.interact(button, Id::unique("b"), Sense::click_and_drag());
        clicks += r.clicked() as usize;
        seconds += r.secondary_clicked() as usize;
        doubles += r.double_clicked() as usize;
        if ui.input(|i| i.modifiers.ctrl) {
            dragged += r.drag_delta().y;
        }
        hovered = r.hovered();
        let p = ui.painter();
        p.text(pos2(100.0, 50.0), Align2::CENTER_CENTER, "red", font(), Color32::RED);
        p.circle(pos2(150.0, 50.0), 5.0, Color32::GREEN, Stroke::NONE);
        if r.clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(vec2(1.0, 2.0)));
        }
    };
    w.click(button.center(), &mut run);
    w.click_with(button.center(), PointerButton::Secondary, &mut run);
    w.double_click(button.center(), &mut run);
    w.drag(button.center(), &[vec2(0.0, 5.0), vec2(0.0, 5.0)], Modifiers::CTRL, &mut run);
    w.hover(button.center(), &mut run);
    assert_eq!((clicks, seconds, doubles), (3, 1, 1), "a double-click is two clicks too");
    assert!(dragged > 0.0, "{dragged}");
    assert!(hovered);
    assert_eq!(w.rect(Id::unique("b")), Some(button));
    assert_eq!(w.rect(Id::unique("none")), None);
    assert_eq!(w.texts_in(Rect::EVERYTHING), ["red"]);
    assert_eq!(w.texts_in(button), Vec::<String>::new());
    assert_eq!(w.texts_coloured(Color32::RED), ["red"]);
    assert!(w.texts_coloured(Color32::BLUE).is_empty());
    assert!(w.circle_in(Rect::EVERYTHING, Color32::GREEN));
    assert!(!w.circle_in(button, Color32::GREEN));
    assert!(w.commands().is_empty(), "the hover pass sent none");
    assert!(w.time > 1.0);
}

#[test]
fn the_window_reports_the_commands_of_the_last_pass() {
    let mut w = Window::new(vec2(100.0, 100.0));
    // egui's first pass sets the theme.
    w.idle(&mut |_| {});
    w.idle(&mut |ui| ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(vec2(1.0, 2.0))));
    assert_eq!(w.commands(), [ViewportCommand::InnerSize(vec2(1.0, 2.0))]);
    let mut nested = |ui: &mut egui::Ui| {
        ui.painter().add(egui::Shape::Vec(vec![egui::Shape::circle_filled(pos2(5.0, 5.0), 2.0, Color32::RED)]));
    };
    w.idle(&mut nested);
    assert!(w.circle_in(Rect::EVERYTHING, Color32::RED), "nested shapes are flattened");
}

/// Ian's display scaling, 125 %, a scale not 1 so that one given shows.
const SCALE_125: f32 = 1.25;

#[test]
fn the_window_gives_egui_the_display_scale_it_is_set_to_and_none_by_default() {
    // ZOOM-DRAG-STEP, point 5 (Ian, 2026-11-26): egui-baseview gives egui
    // the display's scale as `native_pixels_per_point`, which the zoom's
    // sizes in pixels are worked from; by default none, as before.
    let mut w = Window::new(vec2(100.0, 100.0));
    assert_eq!(w.display_scale, None);
    w.idle(&mut |_| {});
    assert_eq!(w.ctx.native_pixels_per_point(), None, "none given by default");
    w.display_scale = Some(SCALE_125);
    let mut seen = None;
    w.idle(&mut |ui| seen = ui.ctx().native_pixels_per_point());
    assert_eq!(seen, Some(SCALE_125), "given in the pass it is set for");
    assert_eq!(w.ctx.pixels_per_point(), SCALE_125, "egui's points scaled by it, the zoom 1");
}

#[test]
#[should_panic(expected = "a pass was run")]
fn reading_before_a_pass_panics() {
    Window::new(vec2(1.0, 1.0)).shapes();
}

#[test]
fn text_width_measures_in_the_font() {
    assert!(text_width("WWWW", font()) > text_width("iiii", font()));
    assert_eq!(text_width("", font()), 0.0);
}

/// PW-HOLD: a gesture made in one instant, begin, set, end, as a click makes
/// it.
fn instant(host: &FakeHost, id: &str, to: f32) {
    host.begin(id);
    host.set(id, to);
    host.end(id);
}

#[test]
fn pw_hold_a_modelled_host_shows_a_set_only_at_its_next_process_step() {
    for model in MODELS {
        let host = FakeHost::new(&[("mix", 0.5)]).modelled(model);
        host.begin("mix");
        host.set("mix", 0.75);
        assert_eq!(host.get("mix"), 0.5, "{model:?}: not before the process step");
        host.process();
        assert_eq!(host.get("mix"), 0.75, "{model:?}: passed on while the gesture is open");
        host.end("mix");
        host.process();
        assert_eq!(host.get("mix"), 0.75, "{model:?}: the end changes nothing once the value has arrived");
        assert_eq!(host.take(), ["begin mix", "set mix", "end mix"], "{model:?}");
    }
}

#[test]
fn pw_hold_the_echoing_host_passes_on_an_instant_edit() {
    let host = FakeHost::new(&[("on", 0.0)]).modelled(Model::Echo);
    instant(&host, "on", 1.0);
    host.process();
    assert_eq!(host.get("on"), 1.0);
    // And a set outside any gesture, and an end with none open.
    host.set("on", 0.0);
    host.end("on");
    host.process();
    assert_eq!(host.get("on"), 0.0);
}

#[test]
fn pw_hold_the_read_back_host_undoes_an_instant_edit_with_the_value_it_reads_at_the_end() {
    let host = FakeHost::new(&[("on", 0.0)]).modelled(Model::ReadBack);
    instant(&host, "on", 1.0);
    host.process();
    assert_eq!(host.get("on"), 0.0, "the old value, read at the end, applied after the set");
}

#[test]
fn pw_hold_the_while_open_host_drops_a_set_whose_gesture_has_ended() {
    let host = FakeHost::new(&[("on", 0.0)]).modelled(Model::WhileOpen);
    instant(&host, "on", 1.0);
    host.process();
    assert_eq!(host.get("on"), 0.0, "ended before the process step: never passed on");
    host.set("on", 1.0);
    host.process();
    assert_eq!(host.get("on"), 0.0, "a set outside a gesture isn't passed on");
}

#[test]
fn pw_hold_a_modelled_host_passes_on_the_last_set_rounded_to_its_steps() {
    let host = FakeHost::new(&[("mode", 0.0)]).stepped(&[("mode", 4)]).modelled(Model::Echo);
    host.begin("mode");
    host.set("mode", 0.9);
    host.set("mode", 0.3);
    host.process();
    assert_eq!(host.get("mode"), 0.25);
    host.end("mode");
}

#[test]
fn pw_hold_the_fake_host_shows_a_value_as_its_steps_round_it() {
    let host = FakeHost::new(&[("mode", 0.0), ("mix", 0.5)]).stepped(&[("mode", 4)]).with(&[("mode", 0.25)]);
    assert!(host.shows("mode", 0.3), "0.3 rounds to 0.25");
    assert!(!host.shows("mode", 0.4));
    assert!(host.shows("mix", 0.5));
    assert!(!host.shows("mix", 0.51));
}
