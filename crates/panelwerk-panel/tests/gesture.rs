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

/// W-3: a press on the knob, a drag of `moves`, the knob hidden (`shown`
/// false) from the pass before the release, then `after` idle passes. The
/// panel draws the switch throughout, as a panel draws its other controls.
fn drag_then_hide(host: &FakeHost, moves: usize, after: usize) {
    let shown = std::cell::Cell::new(true);
    let mut run = |ui: &mut Ui| {
        if shown.get() {
            let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
            gesture::knob(ui, host, &r, "mix");
        }
        let s = ui.interact(SWITCH, Id::unique("switch"), Sense::click());
        gesture::toggle_on_click(ui, host, &s, "on");
    };
    let mut w = Window::new(vec2(300.0, 120.0));
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let mut at = KNOB.center();
    w.pass(vec![egui::Event::PointerMoved(at)], Modifiers::NONE, &mut run);
    w.pass(vec![button(at, true)], Modifiers::NONE, &mut run);
    for _ in 0..moves {
        at.y -= 5.0;
        w.pass(vec![egui::Event::PointerMoved(at)], Modifiers::NONE, &mut run);
    }
    // A preset, automation or a mode change hides the knob mid-drag.
    shown.set(false);
    w.pass(vec![egui::Event::PointerMoved(at)], Modifiers::NONE, &mut run);
    w.pass(vec![button(at, false)], Modifiers::NONE, &mut run);
    for _ in 0..after {
        w.idle(&mut run);
    }
}

#[test]
fn w_3_a_knob_hidden_mid_drag_still_ends_its_gesture() {
    // An open gesture keeps a host writing automation (Cubase in Touch or
    // Latch): the drag's end must reach the host though the knob that
    // began it is no longer drawn.
    let host = host();
    drag_then_hide(&host, 4, 2);
    one_drag(&host.take(), "mix");
}

#[test]
fn w_3_a_lost_drag_is_ended_by_another_knob_too() {
    // Only knobs left drawn: the next one drawn ends the hidden one's drag.
    let host = host();
    let other = Rect::from_min_size(pos2(120.0, 20.0), vec2(72.0, 72.0));
    let shown = std::cell::Cell::new(true);
    let mut run = |ui: &mut Ui| {
        if shown.get() {
            let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
            gesture::knob(ui, &host, &r, "mix");
        }
        let r = ui.interact(other, Id::unique("other"), Sense::click_and_drag());
        gesture::knob(ui, &host, &r, "rate");
    };
    let mut w = Window::new(vec2(300.0, 120.0));
    let press = |pressed| egui::Event::PointerButton {
        pos: KNOB.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    w.pass(vec![egui::Event::PointerMoved(KNOB.center())], Modifiers::NONE, &mut run);
    w.pass(vec![press(true)], Modifiers::NONE, &mut run);
    w.pass(vec![egui::Event::PointerMoved(KNOB.center() - vec2(0.0, 20.0))], Modifiers::NONE, &mut run);
    shown.set(false);
    w.pass(vec![press(false)], Modifiers::NONE, &mut run);
    w.idle(&mut run);
    one_drag(&host.take(), "mix");
}

#[test]
fn w_3_a_drag_ends_once_whichever_control_is_drawn_first() {
    // The switch drawn before the knob looks for lost drags in the release
    // pass before the knob sees its own drag_stopped: one end, not two.
    let host = host();
    let mut run = |ui: &mut Ui| {
        let s = ui.interact(SWITCH, Id::unique("switch"), Sense::click());
        gesture::toggle_on_click(ui, &host, &s, "on");
        let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
        gesture::knob(ui, &host, &r, "mix");
    };
    let mut w = Window::new(vec2(300.0, 120.0));
    w.drag(KNOB.center(), &[vec2(0.0, -10.0), vec2(0.0, -10.0)], Modifiers::NONE, &mut run);
    one_drag(&host.take(), "mix");
    assert!((host.get("mix") - 0.6).abs() < 1e-5, "{}", host.get("mix"));
}

#[test]
fn w_3_a_knob_hidden_mid_drag_ends_its_gesture_through_held_edits() {
    // With held edits wired in, the lost drag's end waits as any other and
    // reaches the host at the next frame that finds its value shown.
    use panelwerk_panel::hold::Held;
    let host = host();
    let held = Held::new();
    struct Frame<'a>(&'a FakeHost, &'a Held);
    impl panelwerk_panel::ParamHost for Frame<'_> {
        fn value(&self, id: &str) -> f32 {
            self.0.value(id)
        }
        fn default_value(&self, id: &str) -> f32 {
            self.0.default_value(id)
        }
        fn text(&self, id: &str) -> String {
            self.0.text(id)
        }
        fn begin(&self, id: &str) {
            self.1.begin(self.0, id)
        }
        fn set(&self, id: &str, normalized: f32) {
            self.1.set(self.0, id, normalized)
        }
        fn end(&self, id: &str) {
            self.1.end(id)
        }
    }
    let frame = Frame(&host, &held);
    let shown = std::cell::Cell::new(true);
    let mut run = |ui: &mut Ui| {
        let _ = held.frame(&host, ui.input(|i| i.time));
        if shown.get() {
            let r = ui.interact(KNOB, Id::unique("knob"), Sense::click_and_drag());
            gesture::knob(ui, &frame, &r, "mix");
        }
        let s = ui.interact(SWITCH, Id::unique("switch"), Sense::click());
        gesture::toggle_on_click(ui, &frame, &s, "on");
    };
    let mut w = Window::new(vec2(300.0, 120.0));
    let press = |pressed| egui::Event::PointerButton {
        pos: KNOB.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    w.pass(vec![egui::Event::PointerMoved(KNOB.center())], Modifiers::NONE, &mut run);
    w.pass(vec![press(true)], Modifiers::NONE, &mut run);
    w.pass(vec![egui::Event::PointerMoved(KNOB.center() - vec2(0.0, 20.0))], Modifiers::NONE, &mut run);
    shown.set(false);
    w.pass(vec![press(false)], Modifiers::NONE, &mut run);
    w.idle(&mut run);
    w.idle(&mut run);
    one_drag(&host.take(), "mix");
}

/// W-3b: no value sent for a parameter whose gesture isn't open, and every
/// gesture begun is the gesture ended: each `set id` and `end id` comes
/// while `id`'s gesture is open, and none is left open.
fn every_set_inside_its_gesture(gestures: &[String]) {
    let mut open: Vec<&str> = vec![];
    for g in gestures {
        let (what, id) = g.split_once(' ').unwrap();
        match what {
            "begin" => open.push(id),
            "set" => assert!(open.contains(&id), "a set of {id} with no gesture open: {gestures:?}"),
            _ => {
                let at = open.iter().position(|o| *o == id);
                assert!(at.is_some(), "an end of {id} with no gesture open: {gestures:?}");
                open.remove(at.unwrap());
            }
        }
    }
    assert!(open.is_empty(), "left open: {open:?} in {gestures:?}");
}

/// W-3b: Kör's Rate knob, which keeps one widget ID but draws Sync Rate when
/// Sync is on. A press on the knob, a move of 20 points up while it is
/// Rate's, then Sync Rate's for a second move of 20 and the release, then
/// an idle pass; `frame` runs at the start of every pass (held edits).
fn drag_while_the_knob_changes_parameter(host: &dyn panelwerk_panel::ParamHost, frame: &mut dyn FnMut(&Ui)) {
    drag_in_while_the_knob_changes_parameter(&mut Window::new(vec2(300.0, 120.0)), host, frame);
}

/// The same, in window `w`, so a later drag in it sees whatever the first
/// left in egui's memory.
fn drag_in_while_the_knob_changes_parameter(
    w: &mut Window,
    host: &dyn panelwerk_panel::ParamHost,
    frame: &mut dyn FnMut(&Ui),
) {
    let param = std::cell::Cell::new("rate");
    let mut run = |ui: &mut Ui| {
        frame(ui);
        let r = ui.interact(KNOB, Id::unique("rate knob"), Sense::click_and_drag());
        gesture::knob(ui, host, &r, param.get());
    };
    let press = |at, pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let at = KNOB.center();
    w.pass(vec![egui::Event::PointerMoved(at)], Modifiers::NONE, &mut run);
    w.pass(vec![press(at, true)], Modifiers::NONE, &mut run);
    w.pass(vec![egui::Event::PointerMoved(at - vec2(0.0, 20.0))], Modifiers::NONE, &mut run);
    // Sync turned on mid-drag (automation, or another control's MIDI learn):
    // the knob now draws Sync Rate, with the same widget ID.
    param.set("sync_rate");
    w.pass(vec![egui::Event::PointerMoved(at - vec2(0.0, 40.0))], Modifiers::NONE, &mut run);
    w.pass(vec![press(at - vec2(0.0, 40.0), false)], Modifiers::NONE, &mut run);
    w.idle(&mut run);
}

fn rate_host() -> FakeHost {
    FakeHost::new(&[("rate", 0.0), ("sync_rate", 0.0)])
}

#[test]
fn w_3b_a_drag_keeps_setting_the_parameter_it_began_with() {
    // Before W-3b: `begin rate, set rate, set sync_rate, end rate`, a set of
    // Sync Rate with no gesture open (the review of W-3).
    let host = rate_host();
    drag_while_the_knob_changes_parameter(&host, &mut |_| {});
    let gestures = host.take();
    assert_eq!(gestures, ["begin rate", "set rate", "set rate", "end rate"]);
    every_set_inside_its_gesture(&gestures);
    // The whole 40 points went to Rate, 40 / DRAG_RANGE of its travel.
    assert!((host.get("rate") - 40.0 / DRAG_RANGE).abs() < 1e-5, "{}", host.get("rate"));
    assert_eq!(host.get("sync_rate"), 0.0, "Sync Rate untouched");
}

#[test]
fn w_3b_the_next_drag_begins_on_the_parameter_the_knob_now_draws() {
    let host = rate_host();
    drag_while_the_knob_changes_parameter(&host, &mut |_| {});
    host.take();
    let mut w = Window::new(vec2(300.0, 120.0));
    let mut run = |ui: &mut Ui| {
        let r = ui.interact(KNOB, Id::unique("rate knob"), Sense::click_and_drag());
        gesture::knob(ui, &host, &r, "sync_rate");
    };
    w.drag(KNOB.center(), &[vec2(0.0, -20.0)], Modifiers::NONE, &mut run);
    one_drag(&host.take(), "sync_rate");
    assert!((host.get("sync_rate") - 20.0 / DRAG_RANGE).abs() < 1e-5, "{}", host.get("sync_rate"));
}

#[test]
fn w_3b_through_held_edits_under_each_modelled_host_the_drag_stays_rates() {
    // The same drag through held edits, under each of the three modelled
    // hosts (B-PNL-11): one gesture on Rate, and Rate gets the drag's value
    // (under `WhileOpen` a set outside a gesture would be lost).
    use panelwerk_panel::hold::Held;
    use panelwerk_testkit::MODELS;
    struct Frame<'a>(&'a FakeHost, &'a Held);
    impl panelwerk_panel::ParamHost for Frame<'_> {
        fn value(&self, id: &str) -> f32 {
            self.0.value(id)
        }
        fn default_value(&self, id: &str) -> f32 {
            self.0.default_value(id)
        }
        fn text(&self, id: &str) -> String {
            self.0.text(id)
        }
        fn begin(&self, id: &str) {
            self.1.begin(self.0, id)
        }
        fn set(&self, id: &str, normalized: f32) {
            self.1.set(self.0, id, normalized)
        }
        fn end(&self, id: &str) {
            self.1.end(id)
        }
    }
    for model in MODELS {
        let host = rate_host().modelled(model);
        let held = Held::new();
        let frame = Frame(&host, &held);
        drag_while_the_knob_changes_parameter(&frame, &mut |ui| {
            host.process();
            let _ = held.frame(&host, ui.input(|i| i.time));
        });
        host.process();
        held.close(&host);
        let gestures = host.take();
        every_set_inside_its_gesture(&gestures);
        assert!(gestures.iter().all(|g| g.ends_with(" rate")), "{model:?}: {gestures:?}");
        assert!((host.get("rate") - 40.0 / DRAG_RANGE).abs() < 1e-5, "{model:?}: {}", host.get("rate"));
        assert_eq!(host.get("sync_rate"), 0.0, "{model:?}");
    }
}

#[test]
fn w_3b_the_next_drag_in_the_same_window_begins_on_the_parameter_now_drawn() {
    // As the test above, in the window the first drag ran in: nothing it
    // left in egui's memory (its open-drag record, its drag position) sends
    // the second drag to Rate, or starts it from Rate's value.
    let host = rate_host();
    let mut w = Window::new(vec2(300.0, 120.0));
    drag_in_while_the_knob_changes_parameter(&mut w, &host, &mut |_| {});
    host.take();
    let mut run = |ui: &mut Ui| {
        let r = ui.interact(KNOB, Id::unique("rate knob"), Sense::click_and_drag());
        gesture::knob(ui, &host, &r, "sync_rate");
    };
    w.drag(KNOB.center(), &[vec2(0.0, -20.0)], Modifiers::NONE, &mut run);
    one_drag(&host.take(), "sync_rate");
    assert!((host.get("sync_rate") - 20.0 / DRAG_RANGE).abs() < 1e-5, "{}", host.get("sync_rate"));
    assert!((host.get("rate") - 40.0 / DRAG_RANGE).abs() < 1e-5, "Rate as the first drag left it: {}", host.get("rate"));
}

#[test]
fn w_3b_a_knob_not_drawn_as_its_drag_starts_sends_nothing() {
    // The knob is pressed, then not drawn on the pass where egui decides the
    // drag has started, so it never sees `drag_started` and no gesture is
    // begun. Drawn again for the rest of the drag, it is dragged with no
    // gesture open: the whole drag is dropped (no begin, set or end, the
    // value unchanged) rather than send a set outside any gesture.
    let host = rate_host();
    let shown = std::cell::Cell::new(true);
    let mut run = |ui: &mut Ui| {
        if shown.get() {
            let r = ui.interact(KNOB, Id::unique("rate knob"), Sense::click_and_drag());
            gesture::knob(ui, &host, &r, "rate");
        }
    };
    let mut w = Window::new(vec2(300.0, 120.0));
    let press = |at, pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let at = KNOB.center();
    w.pass(vec![egui::Event::PointerMoved(at)], Modifiers::NONE, &mut run);
    w.pass(vec![press(at, true)], Modifiers::NONE, &mut run);
    shown.set(false);
    w.pass(vec![egui::Event::PointerMoved(at - vec2(0.0, 20.0))], Modifiers::NONE, &mut run);
    shown.set(true);
    w.pass(vec![egui::Event::PointerMoved(at - vec2(0.0, 40.0))], Modifiers::NONE, &mut run);
    w.pass(vec![egui::Event::PointerMoved(at - vec2(0.0, 60.0))], Modifiers::NONE, &mut run);
    w.pass(vec![press(at - vec2(0.0, 60.0), false)], Modifiers::NONE, &mut run);
    w.idle(&mut run);
    assert_eq!(host.take(), Vec::<String>::new());
    assert_eq!(host.get("rate"), 0.0);
}
