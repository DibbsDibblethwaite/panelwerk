//! The knob (Kör's look, PW-6's details from Rödalog; sizes per consumer):
//! the travel, the value arc, the body and its rim, the pointer; the whole
//! control with its name and value text; the small knob with its name
//! beside it.

use std::f32::consts::PI;

use egui::epaint::{ColorMode, PathShape, Shape};
use egui::{pos2, vec2, Color32, Id, Modifiers, Pos2, Rect, Ui, Vec2};
use panelwerk_panel::knob::{self, angle, Geometry, Look, RIM_WIDTH};
use panelwerk_panel::palette::*;
use panelwerk_panel::Control;
use panelwerk_testkit::{FakeHost, Window};

/// Kör's main knob (B-PNL-1): radius 15, its arc 30 % further out.
const KOER: Geometry =
    Geometry { radius: 15.0, arc_radius: 19.5, arc_width: 2.7, pointer: (4.5, 13.0), pointer_width: 2.0 };
const C: Pos2 = pos2(50.0, 50.0);

fn painted(look: Look) -> Window {
    let mut w = Window::new(vec2(100.0, 100.0));
    w.idle(&mut |ui: &mut Ui| knob::paint(ui.painter(), C, &KOER, &look));
    w
}

fn paths(w: &Window) -> Vec<PathShape> {
    w.shapes().into_iter().filter_map(|s| if let Shape::Path(p) = s { Some(p) } else { None }).collect()
}

fn colour(p: &PathShape) -> Color32 {
    match p.stroke.color {
        ColorMode::Solid(c) => c,
        _ => panic!("solid"),
    }
}

fn at(radius: f32, position: f32) -> Pos2 {
    C + Vec2::angled(angle(position)) * radius
}

fn near(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn the_travel_is_270_degrees_from_7_30_clockwise() {
    assert!((angle(0.0) - 0.75 * PI).abs() < 1e-6);
    assert!((angle(0.5) - 1.5 * PI).abs() < 1e-6, "12 o'clock");
    assert!((angle(1.0) - 2.25 * PI).abs() < 1e-6);
    assert_eq!(angle(-1.0), angle(0.0));
    assert_eq!(angle(2.0), angle(1.0));
    // 12 o'clock is straight up (egui's y is down).
    assert!((Vec2::angled(angle(0.5)) - vec2(0.0, -1.0)).length() < 1e-6);
}

#[test]
fn the_track_and_the_value_arc() {
    let w = painted(Look { value: 0.25, ..Look::default() });
    let p = paths(&w);
    assert_eq!(p.len(), 2);
    let (track, value) = (&p[0], &p[1]);
    assert_eq!((colour(track), track.stroke.width), (TRACK, 2.7));
    assert!(near(track.points[0], at(19.5, 0.0)) && near(*track.points.last().unwrap(), at(19.5, 1.0)));
    assert_eq!((colour(value), value.stroke.width), (RED, 2.7));
    assert!(near(value.points[0], at(19.5, 0.0)) && near(*value.points.last().unwrap(), at(19.5, 0.25)));
    // About a point every 4°.
    assert!(track.points.len() >= 60, "{}", track.points.len());
    // At its minimum, no value arc.
    assert_eq!(paths(&painted(Look::default())).len(), 1);
}

#[test]
fn the_body_rim_and_pointer_lit() {
    let w = painted(Look { value: 0.5, ..Look::default() });
    let circles: Vec<_> =
        w.shapes().into_iter().filter_map(|s| if let Shape::Circle(c) = s { Some(c) } else { None }).collect();
    assert_eq!(circles.len(), 1);
    let body = circles[0];
    assert_eq!(
        (body.center, body.radius, body.fill, body.stroke.color, body.stroke.width),
        (C, 15.0, KNOB, RIM, RIM_WIDTH)
    );
    assert_eq!(RIM_WIDTH, 1.5);
    let segments: Vec<_> = w
        .shapes()
        .into_iter()
        .filter_map(|s| if let Shape::LineSegment { points, stroke } = s { Some((points, stroke)) } else { None })
        .collect();
    assert_eq!(segments.len(), 1);
    let (points, stroke) = segments[0];
    // PW-6: the pointer white, as Rödalog's (Kör's was the text colour).
    assert_eq!((stroke.color, stroke.width), (CHOSEN, 2.0));
    assert!(near(points[0], at(4.5, 0.5)) && near(points[1], at(13.0, 0.5)));
}

#[test]
fn pw_6_hot_and_dimmed_rims_and_pointers() {
    let rim = |look| {
        painted(look)
            .shapes()
            .into_iter()
            .find_map(|s| if let Shape::Circle(c) = s { Some(c.stroke.color) } else { None })
            .unwrap()
    };
    let pointer = |look| {
        painted(look)
            .shapes()
            .into_iter()
            .find_map(|s| if let Shape::LineSegment { stroke, .. } = s { Some(stroke.color) } else { None })
            .unwrap()
    };
    let look = |hot, dimmed| Look { value: 0.5, hot, dimmed };
    // Lit: the rim red while hot. Dimmed: in the text colour while hot
    // (Rödalog's; Kör's was the dimmed value's grey), its pointer grey.
    assert_eq!(
        [rim(look(false, false)), rim(look(true, false)), rim(look(false, true)), rim(look(true, true))],
        [RIM, RED, RIM, TEXT]
    );
    assert_eq!([pointer(look(false, false)), pointer(look(false, true))], [CHOSEN, DIMMED_TEXT]);
    // A dimmed knob's value arc is grey.
    let p = paths(&painted(look(false, true)));
    assert_eq!(colour(&p[1]), DIMMED_VALUE);
}

const RECT: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(92.0, 92.0) };

fn control(dimmed: bool) -> Control<'static> {
    Control { rect: RECT, id: Id::unique("mix"), param: "mix", label: "Mix", dimmed }
}

#[test]
fn the_knob_control_its_name_above_its_value_below_and_its_gestures() {
    let host = FakeHost::new(&[("mix", 0.5)]);
    for dimmed in [false, true] {
        let mut w = Window::new(vec2(120.0, 120.0));
        let centre = RECT.center_top() + vec2(0.0, 37.0);
        let mut run = |ui: &mut Ui| {
            knob::knob(ui, &host, &control(dimmed), centre, &KOER);
        };
        w.idle(&mut run);
        assert_eq!(w.rect(Id::unique("mix")), Some(RECT));
        assert_eq!(w.texts_in(RECT), ["Mix", "mix=0.500"]);
        let (text, _) = panelwerk_panel::colours(dimmed);
        assert_eq!(w.texts_coloured(text), ["Mix", "mix=0.500"]);
        let y = |t: &str| {
            w.shapes().into_iter().find_map(|s| match s {
                Shape::Text(x) if x.galley.text() == t => Some(x.visual_bounding_rect()),
                _ => None,
            })
        };
        assert!(y("Mix").unwrap().top() >= RECT.top() - 0.5 && y("Mix").unwrap().bottom() < centre.y);
        assert!(y("mix=0.500").unwrap().bottom() <= RECT.bottom() + 0.5);
        assert!(w.shapes().iter().any(|s| matches!(s, Shape::Circle(c) if c.center == centre)));
        // Its gestures: 20 points up is +10 %; a double-click the default.
        w.drag(RECT.center(), &[vec2(0.0, -20.0)], Modifiers::NONE, &mut run);
        assert!((host.get("mix") - 0.6).abs() < 1e-5);
        w.double_click(RECT.center(), &mut run);
        assert_eq!(host.get("mix"), 0.5);
        // Hovered, its rim is hot.
        w.hover(RECT.center(), &mut run);
        let hot = if dimmed { TEXT } else { RED };
        assert!(w.shapes().iter().any(|s| matches!(s, Shape::Circle(c) if c.stroke.color == hot)));
    }
}

#[test]
fn the_inline_knob_its_name_at_the_left_and_no_value() {
    // Kör's title-row knobs (B-PNL-1): the value is in the help line.
    let host = FakeHost::new(&[("mix", 0.5)]);
    let rect = Rect::from_min_size(pos2(10.0, 4.0), vec2(96.0, 24.0));
    let small = Geometry { radius: 7.5, arc_radius: 9.75, arc_width: 2.0, pointer: (2.25, 5.5), pointer_width: 2.0 };
    let centre = pos2(rect.right() - 11.0, rect.center().y);
    let mut w = Window::new(vec2(120.0, 40.0));
    let mut run = |ui: &mut Ui| {
        knob::inline(ui, &host, &Control { rect, ..control(true) }, centre, &small);
    };
    w.idle(&mut run);
    assert_eq!(w.texts_in(rect), ["Mix"]);
    assert_eq!(w.texts_coloured(DIMMED_TEXT), ["Mix"]);
    let name = w
        .shapes()
        .into_iter()
        .find_map(|s| if let Shape::Text(t) = s { Some(t.visual_bounding_rect()) } else { None })
        .unwrap();
    assert!((name.left() - rect.left()).abs() < 1.5 && (name.center().y - rect.center().y).abs() < 1.5, "{name:?}");
    assert!(w.shapes().iter().any(|s| matches!(s, Shape::Circle(c) if c.center == centre && c.radius == 7.5)));
    w.drag(rect.center(), &[vec2(0.0, -20.0)], Modifiers::NONE, &mut run);
    assert!((host.get("mix") - 0.6).abs() < 1e-5);
    assert_eq!(host.take().first().map(String::as_str), Some("begin mix"));
}
