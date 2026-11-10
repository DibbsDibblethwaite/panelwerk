//! The test kit itself: the fake host's record and lag, and the window's
//! input and readings.

use egui::{pos2, vec2, Align2, Color32, Id, Modifiers, PointerButton, Rect, Sense, Stroke, ViewportCommand};
use panelwerk_panel::{font, ParamHost};
use panelwerk_testkit::{text_width, FakeHost, Window};

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
