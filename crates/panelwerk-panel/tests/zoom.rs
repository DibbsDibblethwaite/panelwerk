//! Zoom (Kör B-PNL-6, 7; PW-7: the mechanism shared, the range each
//! consumer's): the panel drawn at its base size and scaled as a whole; a
//! size the host gives becomes a zoom; the right-click menu; the settings
//! file that remembers the last zoom for new instances (PW-8).

use std::ffi::OsString;
use std::path::PathBuf;

use egui::{pos2, vec2, Align2, Context, Id, PointerButton, Pos2, Rect, Sense, Ui, Vec2, ViewportCommand};
use panelwerk_panel::palette::{CHOICE_TEXT, CHOSEN};
use panelwerk_panel::settings;
use panelwerk_panel::zoom::{self, Range, MENU_ROW};
use panelwerk_panel::{font, title};
use panelwerk_testkit::{FakeHost, Window};

/// Kör's range (B-PNL-6), as a consumer gives it.
const KOER: Range = Range { min: 0.75, max: 3.0, menu: &[0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0] };
const BASE: Vec2 = vec2(520.0, 280.0);
const NAME: Rect = Rect { min: pos2(12.0, 4.0), max: pos2(56.0, 28.0) };
const MARKER: Rect = Rect { min: pos2(300.0, 100.0), max: pos2(340.0, 140.0) };

/// A panel as a consumer's: zoom::follow first; a marker control; the name
/// strip opening the menu on a right-click; the menu under it.
fn panel<'a>(host: &'a FakeHost) -> impl FnMut(&mut Ui) + 'a {
    move |ui: &mut Ui| {
        let ctx = ui.ctx().clone();
        zoom::follow(&ctx, host, BASE, &KOER);
        ui.interact(MARKER, Id::unique("marker"), Sense::click());
        ui.painter().text(MARKER.center(), Align2::CENTER_CENTER, "x", font(), CHOSEN);
        if title::strip(ui, NAME, Id::unique("name"), "Kör", 15.0).secondary_clicked() {
            zoom::open_menu(&ctx);
        }
        zoom::menu(&ctx, host, &KOER, NAME.left_bottom() + vec2(0.0, 4.0));
    }
}

fn window() -> Window {
    Window::new(BASE)
}

#[test]
fn pw_7_a_range_clamps_and_turns_a_size_into_a_zoom() {
    assert_eq!(KOER.clamp(0.5), 0.75);
    assert_eq!(KOER.clamp(4.0), 3.0);
    assert_eq!(KOER.clamp(1.3), 1.3);
    assert_eq!(KOER.clamp(f32::NAN), 1.0);
    // The height decides, as nice-plug's aspect-ratio adjustment does.
    assert_eq!(KOER.for_size(vec2(780.0, 420.0), BASE), 1.5);
    assert_eq!(KOER.for_size(vec2(1000.0, 420.0), BASE), 1.5);
    assert_eq!(KOER.for_size(vec2(4000.0, 2000.0), BASE), 3.0);
    assert_eq!(KOER.for_size(vec2(100.0, 50.0), BASE), 0.75);
}

#[test]
fn every_control_scales_with_the_zoom() {
    for z in [0.75, 1.0, 1.5, 3.0] {
        let host = FakeHost::default();
        let mut w = window();
        w.ctx.set_zoom_factor(z);
        let run = &mut panel(&host);
        w.idle(run);
        w.idle(run);
        assert_eq!(w.ctx.zoom_factor(), z);
        assert_eq!(w.output().pixels_per_point, z, "drawn at the window's resolution");
        assert_eq!(w.rect(Id::unique("marker")), Some(MARKER), "in points at the base size");
    }
}

/// What egui-baseview does each pass (Kör's `zoom.rs` note): the screen in
/// points is the window over the zoom it last saw, and after the pass a
/// changed zoom or an InnerSize command resizes the window. The window's
/// size and zoom after `passes`, from `size` at zoom `from`.
fn host_resizes(size: Vec2, from: f32, passes: usize) -> (Vec<f32>, Vec2, f32) {
    let host = FakeHost::default();
    let mut w = window();
    w.ctx.set_zoom_factor(from);
    let run = &mut panel(&host);
    w.idle(run);
    w.idle(run);
    let (mut size, mut seen) = (size, from);
    for _ in 0..passes {
        w.screen = size / seen;
        w.idle(run);
        let zoom = w.ctx.zoom_factor();
        let mut next = None;
        if zoom != seen {
            next = Some(size / seen * zoom);
            seen = zoom;
        }
        for c in w.commands() {
            if let ViewportCommand::InnerSize(s) = c {
                next = Some(s * zoom);
            }
        }
        size = next.unwrap_or(size);
    }
    let zooms = host.zooms.borrow().clone();
    (zooms, size, seen)
}

#[test]
fn the_window_dragged_larger_scales_the_panel_and_the_choice_is_saved() {
    let (zooms, size, zoom) = host_resizes(vec2(780.0, 420.0), 1.0, 4);
    assert_eq!(zoom, 1.5);
    assert!((size - vec2(780.0, 420.0)).length() < 0.01, "{size:?}");
    assert_eq!(zooms, [1.5], "the user chose it");
}

#[test]
fn off_the_aspect_ratio_or_the_range_the_window_is_brought_back() {
    for (asked, want) in [
        (vec2(1000.0, 420.0), vec2(780.0, 420.0)),
        (vec2(600.0, 280.0), vec2(520.0, 280.0)),
        (vec2(4000.0, 2000.0), vec2(1560.0, 840.0)),
        (vec2(200.0, 100.0), vec2(390.0, 210.0)),
    ] {
        let (_, size, zoom) = host_resizes(asked, 1.0, 4);
        assert!((size - want).length() < 0.01, "{asked:?}: {size:?}, zoom {zoom}");
        assert_eq!(BASE * zoom, want);
    }
}

#[test]
fn a_host_that_refuses_the_size_is_not_asked_again_and_again() {
    let host = FakeHost::default();
    let mut w = window();
    let run = &mut panel(&host);
    w.idle(run);
    w.screen = vec2(600.0, 280.0);
    w.idle(run);
    assert_eq!(w.commands(), [ViewportCommand::InnerSize(BASE)]);
    w.idle(run);
    assert!(w.commands().is_empty(), "{:?}", w.commands());
}

fn menu_area() -> Rect {
    Rect::from_min_size(NAME.left_bottom(), vec2(90.0, 150.0))
}

fn sizes(w: &Window) -> Vec<String> {
    w.texts_in(menu_area()).into_iter().filter(|t| t.ends_with(" %")).collect()
}

#[test]
fn the_menu_offers_the_consumers_sizes_the_current_one_in_white() {
    assert_eq!(MENU_ROW, vec2(76.0, 20.0));
    let host = FakeHost::default();
    let mut w = window();
    let run = &mut panel(&host);
    w.idle(run);
    assert!(sizes(&w).is_empty(), "closed until asked");
    w.click_with(NAME.center(), PointerButton::Secondary, run);
    assert_eq!(sizes(&w), ["75 %", "100 %", "125 %", "150 %", "200 %", "250 %", "300 %"]);
    let lit: Vec<_> = w.texts_coloured(CHOSEN).into_iter().filter(|t| t.ends_with(" %")).collect();
    assert_eq!(lit, ["100 %"]);
    assert_eq!(w.texts_coloured(CHOICE_TEXT).len(), 6);
    // Each row where zoom::menu_row says; 150 %, the fourth, clicked.
    let top = NAME.left_bottom() + vec2(0.0, 4.0);
    assert_eq!(zoom::menu_row(top, 3), Rect::from_min_size(top + vec2(0.0, 60.0), MENU_ROW));
    w.hover(zoom::menu_row(top, 3).center(), run);
    assert!(w.shapes().iter().any(|s| matches!(s, egui::Shape::Rect(r) if r.fill == panelwerk_panel::palette::HOVER)));
    w.click(zoom::menu_row(top, 3).center(), run);
    assert_eq!(*host.zooms.borrow(), [1.5]);
    assert_eq!(w.ctx.zoom_factor(), 1.5);
    assert!(sizes(&w).is_empty(), "closed after choosing");
    assert!(host.take().is_empty(), "the size is not a parameter");
}

#[test]
fn a_click_elsewhere_closes_the_menu_unchanged() {
    let host = FakeHost::default();
    let mut w = window();
    let run = &mut panel(&host);
    w.idle(run);
    w.click_with(NAME.center(), PointerButton::Secondary, run);
    assert_eq!(sizes(&w).len(), 7);
    w.click(pos2(300.0, 200.0), run);
    assert!(sizes(&w).is_empty());
    assert!(host.zooms.borrow().is_empty());
    assert_eq!(w.ctx.zoom_factor(), 1.0);
}

#[test]
fn choose_holds_the_zoom_to_the_range() {
    let host = FakeHost::default();
    let ctx = Context::default();
    zoom::choose(&ctx, &host, &KOER, 9.0);
    assert_eq!(*host.zooms.borrow(), [3.0]);
    let _ = Pos2::ZERO;
}

// The settings file (Kör B-PNL-7, PW-8).

fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("panelwerk-settings-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_file_is_panel_json_in_the_consumers_folder_in_appdata() {
    let at = settings::file_in(Some(OsString::from("C:\\Users\\ian\\AppData\\Roaming")), "Koer");
    assert_eq!(at, Some(PathBuf::from("C:\\Users\\ian\\AppData\\Roaming").join("Koer").join("panel.json")));
    assert_eq!(settings::file_in(None, "Koer"), None);
    assert_eq!(settings::file_in(Some(OsString::new()), "Koer"), None);
}

#[test]
fn a_zoom_written_is_read_back_and_no_temporary_file_is_left() {
    let path = folder("200").join("Koer").join("panel.json");
    settings::write(&path, 2.0, &KOER).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"zoom\": 2}\n");
    assert_eq!(settings::read(&path, &KOER), Some(2.0));
    assert_eq!(settings::opening_zoom(None, settings::read(&path, &KOER), &KOER), 2.0);
    settings::write(&path, 1.25, &KOER).unwrap();
    assert_eq!(settings::read(&path, &KOER), Some(1.25));
    let left: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(left, [OsString::from("panel.json")]);
    // Written within the range.
    settings::write(&path, 9.0, &KOER).unwrap();
    assert_eq!(settings::read(&path, &KOER), Some(3.0));
}

#[test]
fn the_projects_zoom_comes_before_the_files() {
    assert_eq!(settings::opening_zoom(Some(1.5), Some(2.0), &KOER), 1.5);
    assert_eq!(settings::opening_zoom(Some(1.5), None, &KOER), 1.5);
    assert_eq!(settings::opening_zoom(None, None, &KOER), 1.0);
    assert_eq!(settings::opening_zoom(Some(9.0), None, &KOER), 3.0);
}

#[test]
fn missing_or_garbage_means_none() {
    let dir = folder("garbage");
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("panel.json");
    assert_eq!(settings::read(&missing, &KOER), None);
    for garbage in ["", "not json", "{\"zoom\": \"big\"}", "{\"size\": 2}", "[2.0]", "{\"zoom\": 1e999}", "\u{0}\u{1}"]
    {
        std::fs::write(&missing, garbage).unwrap();
        assert_eq!(settings::read(&missing, &KOER), None, "{garbage:?}");
    }
    std::fs::write(&missing, "{\"zoom\": 0.1}").unwrap();
    assert_eq!(settings::read(&missing, &KOER), Some(0.75));
    let folder_instead = dir.join("folder.json");
    std::fs::create_dir_all(&folder_instead).unwrap();
    assert_eq!(settings::read(&folder_instead, &KOER), None);
}

#[test]
fn a_failed_write_says_so() {
    let dir = folder("fail");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Koer"), "a file").unwrap();
    assert!(settings::write(&dir.join("Koer").join("panel.json"), 1.5, &KOER).is_err());
    let path = dir.join("taken.json");
    std::fs::create_dir_all(path.join("inside")).unwrap();
    assert!(settings::write(&path, 1.5, &KOER).is_err());
    assert!(settings::write(std::path::Path::new(""), 1.0, &KOER).is_err());
}
