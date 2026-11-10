//! CLAUDE.md rule 2: `panelwerk-panel` depends on egui alone, at exactly the
//! version the consumers' nice-plug-egui 0.6.0 uses, and never on nice-plug
//! or nice-plug-egui, which each consumer vendors and patches for itself.
//! Dev-dependencies are allowed.

/// The entries of one `[section]` of a Cargo manifest (not a TOML parser:
/// enough for the plain manifest this crate keeps).
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
fn rule_2_the_panel_layer_depends_on_egui_0_37_0_only() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[target."), "no target-specific dependency sections");
    assert!(!manifest.contains("[dependencies."), "no dotted dependency tables");
    assert_eq!(section_entries(manifest, "[dependencies]"), vec!["egui = \"=0.37.0\""]);
    assert_eq!(section_entries(manifest, "[build-dependencies]"), Vec::<String>::new());
    let dev = section_entries(manifest, "[dev-dependencies]");
    assert!(!dev.iter().any(|e| e.starts_with("nice-plug")), "never nice-plug or nice-plug-egui, in tests either");
}

#[test]
fn section_entries_reads_one_section() {
    let m = "[package]\nname = \"x\"\n\n[dependencies]\n# c\nfoo = \"1\"\n[dev-dependencies]\nbar = \"2\"\n";
    assert_eq!(section_entries(m, "[dependencies]"), vec!["foo = \"1\""]);
    assert_eq!(section_entries(m, "[dev-dependencies]"), vec!["bar = \"2\""]);
}
