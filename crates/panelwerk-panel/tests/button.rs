//! Lights, button bodies, lit buttons and switches (Kör's, as Rödalog's
//! LEDs): lit red with a lighter ring, dimmed grey, off; outlined buttons
//! filled while hovered.

use egui::epaint::Shape;
use egui::{pos2, vec2, Id, Rect, Ui};
use panelwerk_panel::light::{self, Light};
use panelwerk_panel::palette::*;
use panelwerk_panel::{button, Control};
use panelwerk_testkit::{FakeHost, Window};

#[test]
fn a_light_lit_dimmed_or_off() {
    assert_eq!(
        [Light::new(true, false), Light::new(true, true), Light::new(false, false), Light::new(false, true)],
        [Light::Lit, Light::DimmedLit, Light::Off, Light::Off]
    );
    for (l, fill, ring) in
        [(Light::Lit, LIT, Some(LIT_RING)), (Light::DimmedLit, DIMMED_LIT, None), (Light::Off, LIGHT_OFF, None)]
    {
        let mut w = Window::new(vec2(40.0, 40.0));
        w.idle(&mut |ui: &mut Ui| light::paint(ui.painter(), pos2(20.0, 20.0), 5.0, l));
        let c = w.shapes().into_iter().find_map(|s| if let Shape::Circle(c) = s { Some(c) } else { None }).unwrap();
        assert_eq!((c.center, c.radius, c.fill), (pos2(20.0, 20.0), 5.0, fill), "{l:?}");
        match ring {
            Some(r) => assert_eq!((c.stroke.color, c.stroke.width), (r, light::RING_WIDTH)),
            None => assert_eq!(c.stroke.width, 0.0, "{l:?}"),
        }
    }
    assert_eq!(light::RING_WIDTH, 1.5);
}

fn rects(w: &Window) -> Vec<egui::epaint::RectShape> {
    w.shapes().into_iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r) } else { None }).collect()
}

#[test]
fn a_button_body_outlined_and_filled_while_hot() {
    let rect = Rect::from_min_size(pos2(5.0, 5.0), vec2(40.0, 20.0));
    for hot in [false, true] {
        let mut w = Window::new(vec2(60.0, 40.0));
        w.idle(&mut |ui: &mut Ui| button::body(ui.painter(), rect, hot));
        let r = rects(&w);
        let filled = r.iter().any(|s| s.fill == HOVER && s.rect == rect);
        let outlined = r.iter().any(|s| s.stroke.color == TRACK && s.stroke.width == 1.0 && s.rect == rect);
        assert_eq!((filled, outlined), (hot, true), "hot {hot}");
    }
}

#[test]
fn a_lit_button_its_light_at_the_left_its_name_beside() {
    let rect = Rect::from_min_size(pos2(12.0, 50.0), vec2(48.0, 26.0));
    let geometry = light::Geometry { radius: 5.0, inset: 12.0 };
    let mut w = Window::new(vec2(100.0, 100.0));
    let mut clicks = 0;
    let mut run = |ui: &mut Ui| {
        let r = button::lit(ui, rect, Id::unique("one"), "I", Light::Off, &geometry);
        clicks += r.clicked() as usize;
    };
    w.idle(&mut run);
    assert_eq!(w.rect(Id::unique("one")), Some(rect));
    assert!(w.circle_in(rect, LIGHT_OFF));
    let at = pos2(24.0, rect.center().y);
    assert!(w.shapes().iter().any(|s| matches!(s, Shape::Circle(c) if c.center == at && c.radius == 5.0)));
    let name = w.shapes().into_iter().find_map(|s| if let Shape::Text(t) = s { Some(t) } else { None }).unwrap();
    assert_eq!(name.galley.text(), "I");
    assert!((name.visual_bounding_rect().left() - (at.x + 5.0 + light::TEXT_GAP)).abs() < 1.5);
    assert_eq!(light::TEXT_GAP, 6.0);
    w.click(rect.center(), &mut run);
    assert_eq!(clicks, 1);
    assert!(rects(&w).iter().any(|s| s.fill == HOVER), "hovered after the click");
    let mut w = Window::new(vec2(100.0, 100.0));
    w.idle(&mut |ui: &mut Ui| {
        button::lit(ui, rect, Id::unique("one"), "I", Light::Lit, &geometry);
    });
    assert!(w.circle_in(rect, LIT));
}

#[test]
fn a_switch_its_name_above_a_lit_button_its_value_below() {
    let host = FakeHost::new(&[("noise", 0.0)]).stepped(&[("noise", 1)]);
    let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(72.0, 72.0));
    for dimmed in [false, true] {
        let c = Control { rect, id: Id::unique("noise"), param: "noise", label: "Noise", dimmed };
        let mut w = Window::new(vec2(120.0, 120.0));
        let mut run = |ui: &mut Ui| {
            button::switch(ui, &host, &c, vec2(40.0, 22.0), 5.0);
        };
        w.idle(&mut run);
        assert_eq!(w.texts_in(rect), ["Noise", "noise=0.000"]);
        let (text, _) = panelwerk_panel::colours(dimmed);
        assert_eq!(w.texts_coloured(text).len(), 2);
        let body = Rect::from_center_size(rect.center(), vec2(40.0, 22.0));
        assert!(rects(&w).iter().any(|s| s.rect == body && s.stroke.color == TRACK));
        assert!(w.circle_in(body, LIGHT_OFF));
        w.click(rect.center(), &mut run);
        assert_eq!(host.get("noise"), 1.0);
        assert_eq!(host.take(), ["begin noise", "set noise", "end noise"]);
        assert!(w.circle_in(body, if dimmed { DIMMED_LIT } else { LIT }));
        assert!(rects(&w).iter().any(|s| s.rect == body && s.fill == HOVER), "hovered");
        w.click(rect.center(), &mut run);
        assert_eq!(host.get("noise"), 0.0);
        host.take();
    }
}
