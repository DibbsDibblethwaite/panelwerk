//! CLAUDE.md rule 2: the test kit depends on egui (with its fonts) and the
//! panel only, never nice-plug or nice-plug-egui.

fn section_entries(manifest: &str, section: &str) -> Vec<String> {
    let mut inside = false;
    let mut entries = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == section;
        } else if inside && !line.is_empty() && !line.starts_with('#') {
            entries.push(line.to_string());
        }
    }
    entries
}

#[test]
fn rule_2_the_test_kit_depends_on_egui_and_the_panel_only() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[target."), "no target-specific dependency sections");
    assert_eq!(
        section_entries(manifest, "[dependencies]"),
        [
            "egui = { version = \"=0.37.0\", features = [\"default_fonts\"] }",
            "panelwerk-panel = { path = \"../panelwerk-panel\" }"
        ]
    );
    assert!(section_entries(manifest, "[dev-dependencies]").is_empty());
}
