//! The title strip (Kör B-PNL-9, Rödalog's section titles) and the help
//! line (Kör B-PNL-8, 10).

use egui::epaint::Shape;
use egui::{pos2, vec2, Id, PointerButton, Rect, Ui};
use panelwerk_panel::palette::*;
use panelwerk_panel::{font, help, title};
use panelwerk_testkit::{text_width, Window};

#[test]
fn the_title_strip_white_on_red_filling_its_rectangle() {
    let rect = Rect::from_min_size(pos2(12.0, 4.0), vec2(44.0, 24.0));
    let mut w = Window::new(vec2(100.0, 40.0));
    let (mut clicks, mut seconds) = (0, 0);
    let mut run = |ui: &mut Ui| {
        let r = title::strip(ui, rect, Id::unique("name"), "Kör", 15.0);
        clicks += r.clicked() as usize;
        seconds += r.secondary_clicked() as usize;
    };
    w.idle(&mut run);
    let strips: Vec<_> = w
        .shapes()
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.fill == TITLE_STRIP => Some(r.rect),
            _ => None,
        })
        .collect();
    assert_eq!(strips, [rect]);
    let name = w.shapes().into_iter().find_map(|s| if let Shape::Text(t) = s { Some(t) } else { None }).unwrap();
    assert_eq!(name.galley.text(), "Kör");
    assert!(name
        .galley
        .job
        .sections
        .iter()
        .all(|s| s.format.color == TITLE_TEXT && s.format.font_id == egui::FontId::proportional(15.0)));
    assert!(rect.shrink(2.0).contains_rect(name.visual_bounding_rect()));
    assert!((name.visual_bounding_rect().center() - rect.center()).length() < 1.5);
    w.click(rect.center(), &mut run);
    w.click_with(rect.center(), PointerButton::Secondary, &mut run);
    assert_eq!((clicks, seconds), (1, 1), "its clicks are the caller's");
}

const LINE: Rect = Rect { min: pos2(0.0, 254.0), max: pos2(520.0, 280.0) };

fn line(hint: Option<&str>, value: Option<&str>) -> Window {
    let mut w = Window::new(vec2(520.0, 280.0));
    w.idle(&mut |ui: &mut Ui| help::line(ui.painter(), LINE, hint, value));
    w
}

#[test]
fn the_help_line_26_points_on_the_panels_dark() {
    // Kör B-PNL-8: 26 points; its margins 12, the gap before a value 12.
    assert_eq!((help::HEIGHT, help::MARGIN, help::VALUE_GAP), (26.0, 12.0, 12.0));
    let w = line(None, None);
    assert!(w.shapes().iter().any(|s| matches!(s, Shape::Rect(r) if r.fill == PANEL && r.rect == LINE)));
    assert!(w.texts_in(LINE).is_empty());
}

#[test]
fn a_hint_at_the_left_its_value_in_bright_red_at_the_right() {
    let w = line(Some("How loud the hiss is."), Some("-60.0 dBFS"));
    assert_eq!(w.texts_in(LINE), ["How loud the hiss is.", "-60.0 dBFS"]);
    assert_eq!(w.texts_coloured(TEXT), ["How loud the hiss is."]);
    assert_eq!(w.texts_coloured(BRIGHT_RED), ["-60.0 dBFS"]);
    let bounds = |text: &str| {
        w.shapes()
            .into_iter()
            .find_map(|s| match s {
                Shape::Text(t) if t.galley.text() == text => Some(t.visual_bounding_rect()),
                _ => None,
            })
            .unwrap()
    };
    assert!((bounds("How loud the hiss is.").left() - 12.0).abs() < 1.5);
    assert!((bounds("-60.0 dBFS").right() - 508.0).abs() < 1.5);
    assert!((bounds("-60.0 dBFS").center().y - LINE.center().y).abs() < 1.5);
    let w = line(Some("Mix"), None);
    assert_eq!(w.texts_coloured(TEXT), ["Mix"]);
}

#[test]
fn a_hint_is_clipped_before_the_value_rather_than_run_under_it() {
    let long =
        "A hint far too long for the line, going on and on past where the value is, and further still to the end";
    let value = "-96.0 dBFS";
    let w = line(Some(long), Some(value));
    let clip = w
        .output()
        .shapes
        .iter()
        .find(|c| matches!(&c.shape, Shape::Text(t) if t.galley.text() == long))
        .unwrap()
        .clip_rect;
    let right = LINE.right() - help::MARGIN - text_width(value, font()) - help::VALUE_GAP;
    assert!((clip.right() - right).abs() < 0.01, "{clip:?}, {right}");
    // Without a value, at the right margin.
    let w = line(Some(long), None);
    let clip = w
        .output()
        .shapes
        .iter()
        .find(|c| matches!(&c.shape, Shape::Text(t) if t.galley.text() == long))
        .unwrap()
        .clip_rect;
    assert!((clip.right() - (LINE.right() - help::MARGIN)).abs() < 0.01, "{clip:?}");
}
