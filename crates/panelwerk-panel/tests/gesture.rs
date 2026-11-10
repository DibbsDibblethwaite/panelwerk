//! The gestures (Kör B-PNL-3, Rödalog B-PNL-2), moved from Kör's
//! `tests/controls.rs`: a drag by distance from where the button went down,
//! finer with Ctrl or Shift, stopping at the ends; a double-click for the
//! default; a click toggling; each one host gesture; and the panel going by
//! the value it set while the host lags, for at most half a second.

use egui::{pos2, vec2, Id, Modifiers, Rect, Sense, Ui, Vec2};
use panelwerk_panel::gesture::{self, DRAG_RANGE, FINE, PENDING_FOR};
use panelwerk_panel::hot;
use panelwerk_testkit::{FakeHost, Window};

const KNOB: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(92.0, 92.0) };
const SWITCH: Rect = Rect { min: pos2(120.0, 20.0), max: pos2(192.0, 92.0) };

fn host() -> FakeHost {
    FakeHost::new(&[("mix", 0.5), ("rate", 0.0), ("on", 0.0)]).stepped(&[("rate", 17), ("on", 1)])
}

/// A knob for `param` and a switch for "on", as a panel would have them.
fn panel<'a>(host: &'a FakeHost, param: &'a str) -> impl FnMut(&mut Ui) + 'a {
    move |ui: &mut Ui| {
        let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
        gesture::knob(ui, host, &r, param);
        let s = ui.interact(SWITCH, Id::unique("switch"), Sense::click());
        gesture::toggle_on_click(ui, host, &s, "on");
    }
}

fn window(host: &FakeHost) -> Window {
    let mut w = Window::new(vec2(300.0, 120.0));
    w.idle(&mut panel(host, "mix"));
    w
}

/// Exactly one begin–set–end of `id` (one host gesture, one undo step).
fn one_gesture(gestures: &[String], id: &str) {
    let want = [format!("begin {id}"), format!("set {id}"), format!("end {id}")];
    assert_eq!(gestures, want, "{gestures:?}");
}

/// A drag of `id`: one begin, sets, one end.
fn one_drag(gestures: &[String], id: &str) {
    assert_eq!(gestures.first(), Some(&format!("begin {id}")), "{gestures:?}");
    assert_eq!(gestures.last(), Some(&format!("end {id}")), "{gestures:?}");
    assert_eq!(gestures.iter().filter(|g| g.starts_with("begin")).count(), 1, "{gestures:?}");
    assert_eq!(gestures.iter().filter(|g| g.starts_with("end")).count(), 1, "{gestures:?}");
    assert!(gestures.iter().filter(|g| g.starts_with("set")).all(|g| *g == format!("set {id}")), "{gestures:?}");
}

#[test]
fn b_pnl_3_the_drag_range_and_fine_factor() {
    // Kör B-PNL-3, Rödalog B-PNL-2: 200 points, ten times finer.
    assert_eq!((DRAG_RANGE, FINE), (200.0, 10.0));
    // Kör's and Rödalog's (V3a.21) pending window.
    assert_eq!(PENDING_FOR, 0.5);
}

#[test]
fn b_pnl_3_a_drag_moves_by_the_distance_dragged() {
    let up: Vec<Vec2> = (0..10).map(|_| vec2(0.0, -5.0)).collect();
    // From 50 %: 50 points up is 75 %, with Ctrl or Shift 52.5 %.
    for (modifiers, want) in [(Modifiers::NONE, 0.75), (Modifiers::CTRL, 0.525), (Modifiers::SHIFT, 0.525)] {
        let host = host();
        let mut w = window(&host);
        w.drag(KNOB.center(), &up, modifiers, &mut panel(&host, "mix"));
        assert!((host.get("mix") - want).abs() < 1e-5, "{modifiers:?}: {}", host.get("mix"));
        one_drag(&host.take(), "mix");
    }
}

#[test]
fn b_pnl_3_past_the_end_a_knob_stops_whatever_the_speed() {
    let host = host();
    let mut w = window(&host);
    let run = &mut panel(&host, "mix");
    w.drag(KNOB.center(), &[vec2(0.0, -150.0), vec2(0.0, -150.0)], Modifiers::NONE, run);
    assert_eq!(host.get("mix"), 1.0);
    one_drag(&host.take(), "mix");
    // Down past 0 in many small moves: the same distance, the same change.
    let moves: Vec<Vec2> = (0..50).map(|_| vec2(0.0, 5.0)).collect();
    w.drag(KNOB.center(), &moves, Modifiers::NONE, run);
    assert_eq!(host.get("mix"), 0.0);
    w.drag(KNOB.center(), &[vec2(0.0, -40.0)], Modifiers::NONE, run);
    assert!((host.get("mix") - 0.2).abs() < 1e-6, "{}", host.get("mix"));
}

#[test]
fn b_pnl_3_a_stepped_knob_moves_a_step_per_its_share_of_the_drag() {
    // 18 values: a step per 200/17 points, even when each pass moves less
    // than a step (the drag's own position is kept unrounded).
    let host = host();
    let mut w = window(&host);
    let moves: Vec<Vec2> = (0..20).map(|_| vec2(0.0, -2.0 * DRAG_RANGE / 17.0 / 20.0)).collect();
    w.drag(KNOB.center(), &moves, Modifiers::NONE, &mut panel(&host, "rate"));
    assert!((host.get("rate") * 17.0 - 2.0).abs() < 1e-3, "{}", host.get("rate") * 17.0);
    one_drag(&host.take(), "rate");
}

#[test]
fn b_pnl_3_double_click_sets_the_default_as_one_gesture() {
    let host = host().with(&[("mix", 0.123)]);
    let mut w = window(&host);
    w.double_click(KNOB.center(), &mut panel(&host, "mix"));
    assert_eq!(host.get("mix"), 0.5);
    one_gesture(&host.take(), "mix");
}

#[test]
fn b_pnl_3_a_click_toggles_as_one_gesture() {
    let host = host();
    let mut w = window(&host);
    let run = &mut panel(&host, "mix");
    w.click(SWITCH.center(), run);
    assert_eq!(host.get("on"), 1.0);
    one_gesture(&host.take(), "on");
    w.click(SWITCH.center(), run);
    assert_eq!(host.get("on"), 0.0);
    one_gesture(&host.take(), "on");
    // A click on a knob sets nothing.
    w.click(KNOB.center(), run);
    assert!(host.take().is_empty());
}

#[test]
fn b_pnl_2_two_quick_clicks_while_the_host_lags_toggle_twice() {
    // The host shows the first click's change only later: the second click
    // goes from what the panel set, so the switch goes on and off again.
    let host = host();
    host.lagging.set(true);
    let mut w = window(&host);
    let run = &mut panel(&host, "mix");
    w.click(SWITCH.center(), run);
    w.click(SWITCH.center(), run);
    let sets: Vec<_> = host.take().into_iter().filter(|g| g.starts_with("set")).collect();
    assert_eq!(sets.len(), 2);
    // A host that never catches up: after half a second the panel goes by
    // the host's value again.
    host.lagging.set(false);
    for _ in 0..6 {
        w.idle(run);
    }
    w.click(SWITCH.center(), run);
    assert_eq!(host.get("on"), 1.0, "toggled from the host's Off");
}

#[test]
fn latest_is_the_panels_value_until_the_host_shows_it() {
    let host = host();
    host.lagging.set(true);
    let mut w = window(&host);
    let mut seen = vec![];
    let mut run = |ui: &mut Ui| {
        if seen.is_empty() {
            gesture::set_once(ui, &host, "mix", 0.8);
        }
        seen.push(gesture::latest(ui, &host, "mix"));
    };
    for _ in 0..8 {
        w.idle(&mut run);
    }
    // Set at the first pass: 0.8 for 0.4 s later (passes are a tenth
    // apart), the host's 0.5 again from 0.6 s (0.5 s itself is at the
    // mercy of the clock's rounding).
    assert_eq!(seen[..5], [0.8; 5]);
    assert_eq!(seen[6..], [0.5; 2]);
    one_gesture(&host.take(), "mix");
}

#[test]
fn a_drag_begins_from_the_panels_latest_value() {
    // The panel set 0.9 a moment ago and the host still shows 0.5: a drag
    // 20 points up goes from 0.9, to 1.0, not to 0.6.
    let host = host();
    let mut w = window(&host);
    host.lagging.set(true);
    w.idle(&mut |ui: &mut Ui| gesture::set(ui, &host, "mix", 0.9));
    host.lagging.set(false);
    w.drag(KNOB.center(), &[vec2(0.0, -20.0)], Modifiers::NONE, &mut panel(&host, "mix"));
    assert!((host.get("mix") - 1.0).abs() < 1e-5, "{}", host.get("mix"));
}

#[test]
fn hot_is_hovered_or_dragged() {
    let host = host();
    let mut w = window(&host);
    let mut hots = vec![];
    let mut run = |ui: &mut Ui| {
        let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
        hots.push(hot(&r));
    };
    w.idle(&mut run);
    w.hover(KNOB.center(), &mut run);
    // Dragged off the knob, it stays hot.
    w.pass(
        vec![egui::Event::PointerButton {
            pos: KNOB.center(),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
        Modifiers::NONE,
        &mut run,
    );
    w.pass(vec![egui::Event::PointerMoved(pos2(250.0, 110.0))], Modifiers::NONE, &mut run);
    w.pass(vec![egui::Event::PointerMoved(pos2(251.0, 110.0))], Modifiers::NONE, &mut run);
    assert!(!hots[0]);
    assert!(hots[1..].iter().all(|h| *h), "{hots:?}");
}
