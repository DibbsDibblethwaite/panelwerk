//! CLAUDE.md rule 2: `panelwerk-text` has no dependencies beyond `std`, so
//! a consumer's plugin crate, or its engine, takes nothing else with it.
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
fn rule_2_the_text_layer_has_no_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[target."), "no target-specific dependency sections");
    assert!(!manifest.contains("[dependencies."), "no dotted dependency tables");
    assert_eq!(section_entries(manifest, "[dependencies]"), Vec::<String>::new());
    assert_eq!(section_entries(manifest, "[build-dependencies]"), Vec::<String>::new());
}

#[test]
fn section_entries_reads_one_section() {
    let m = "[package]\nname = \"x\"\n\n[dependencies]\n# c\nfoo = \"1\"\n[dev-dependencies]\nbar = \"2\"\n";
    assert_eq!(section_entries(m, "[dependencies]"), vec!["foo = \"1\""]);
    assert_eq!(section_entries(m, "[dev-dependencies]"), vec!["bar = \"2\""]);
}
