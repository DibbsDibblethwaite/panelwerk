//! CLAUDE.md, Layout and rules 2 and 15, for the one crate allowed the two
//! exceptions (Ian, 2026-11-23, PW-BUILT, point 1 (a)): `panelwerk-nice`
//! depends on nice-plug, exactly 0.4.2, and the panel, nothing else; and
//! `unsafe` is in its `adapter` module and nowhere else in its source, each
//! block with a `// SAFETY:` comment. The other crates keep
//! `#![forbid(unsafe_code)]` and their own manifest checks (the panel's
//! and the test kit's lists are exact, so neither can take nice-plug, nor
//! this crate, by accident).

/// The entries of one `[section]` of a Cargo manifest (not a TOML parser:
/// enough for the plain manifest this crate keeps). The panel's.
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

/// The crate's source files, by name: every module is listed here, and the
/// test below fails on a file in `src/` that isn't, so a new module can't
/// escape the check.
const SOURCES: [(&str, &str); 3] = [
    ("lib.rs", include_str!("../src/lib.rs")),
    ("adapter.rs", include_str!("../src/adapter.rs")),
    ("window.rs", include_str!("../src/window.rs")),
];

/// The one module allowed `unsafe` (rule 15).
const UNSAFE_MODULE: &str = "adapter.rs";

/// The lines of `source` that aren't comments (`//`, `//!`, `///`), so a
/// doc that names `unsafe` doesn't count as code using it.
fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    source.lines().enumerate().filter(|(_, l)| !l.trim_start().starts_with("//"))
}

#[test]
fn rule_2_the_crate_depends_on_nice_plug_0_4_2_and_the_panel_only() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("[target."), "no target-specific dependency sections");
    assert!(!manifest.contains("[dependencies."), "no dotted dependency tables");
    assert_eq!(
        section_entries(manifest, "[dependencies]"),
        [
            "nice-plug = { version = \"=0.4.2\", default-features = false, features = [\"editor\"] }",
            "panelwerk-panel = { path = \"../panelwerk-panel\" }",
        ]
    );
    assert_eq!(section_entries(manifest, "[build-dependencies]"), Vec::<String>::new());
    assert_eq!(
        section_entries(manifest, "[dev-dependencies]"),
        [
            "nice-plug = { version = \"=0.4.2\", default-features = false, features = [\"editor\", \"vst3\"] }",
            "panelwerk-testkit = { path = \"../panelwerk-testkit\" }",
        ]
    );
}

#[test]
fn rule_15_every_source_file_is_checked() {
    let mut files: Vec<String> = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    let mut listed: Vec<&str> = SOURCES.iter().map(|(name, _)| *name).collect();
    listed.sort();
    assert_eq!(files, listed, "every file in src/ is in SOURCES");
}

#[test]
fn rule_15_unsafe_is_denied_in_the_crate_and_allowed_on_the_adapter_module_alone() {
    let lib = SOURCES[0].1;
    assert!(lib.lines().any(|l| l == "#![deny(unsafe_code)]"), "the crate denies unsafe code");
    // An `allow` anywhere else would lift the `deny` there; this one is
    // on the line before the adapter's `mod` and nowhere else.
    for (name, source) in SOURCES {
        let allows: Vec<(usize, &str)> = code_lines(source).filter(|(_, l)| l.contains("unsafe_code")).collect();
        let expected: Vec<&str> = match name {
            "lib.rs" => vec!["#![deny(unsafe_code)]", "#[allow(unsafe_code)]"],
            _ => vec![],
        };
        assert_eq!(allows.iter().map(|(_, l)| l.trim()).collect::<Vec<_>>(), expected, "{name}");
    }
    let lines: Vec<&str> = lib.lines().collect();
    let at = lines.iter().position(|l| *l == "#[allow(unsafe_code)]").unwrap();
    assert_eq!(lines[at + 1], "mod adapter;", "the allow is the adapter module's");
}

#[test]
fn rule_15_only_the_adapter_module_has_unsafe_code_and_each_block_says_why_it_is_sound() {
    for (name, source) in SOURCES {
        let unsafe_lines: Vec<(usize, &str)> = code_lines(source).filter(|(_, l)| l.contains("unsafe")).collect();
        if name != UNSAFE_MODULE {
            let used: Vec<_> = unsafe_lines.iter().filter(|(_, l)| !l.contains("unsafe_code")).collect();
            assert!(used.is_empty(), "{name} has unsafe code: {used:?}");
            continue;
        }
        assert!(!unsafe_lines.is_empty(), "the adapter's calls are unsafe, so the check finds them");
        let lines: Vec<&str> = source.lines().collect();
        for (n, line) in unsafe_lines {
            assert!(line.contains("unsafe {"), "{name}:{}: only unsafe blocks, no unsafe fn or impl: {line}", n + 1);
            // The comment block right above the line starts `// SAFETY:`.
            let comment: Vec<&str> =
                lines[..n].iter().rev().take_while(|l| l.trim_start().starts_with("//")).copied().collect();
            assert!(
                comment.last().is_some_and(|l| l.trim_start().starts_with("// SAFETY:")),
                "{name}:{}: an unsafe block without a `// SAFETY:` comment above it",
                n + 1
            );
        }
    }
}

#[test]
fn rule_15_no_source_comes_from_outside_the_files_checked() {
    // `#[path = …] mod` and `include!` would bring in code from a file the
    // scan above never reads, `unsafe` and its `allow` with it (found in
    // the W-2b review: both passed every other check here). Refused rather
    // than followed: the crate needs neither. Its modules are the two in
    // SOURCES, declared plainly.
    for (name, source) in SOURCES {
        let outside: Vec<(usize, &str)> =
            code_lines(source).filter(|(_, l)| l.contains("#[path") || l.contains("include!")).collect();
        assert!(outside.is_empty(), "{name} takes source from elsewhere: {outside:?}");
        let mods: Vec<&str> = code_lines(source)
            .map(|(_, l)| l.trim())
            .filter(|l| l.starts_with("mod ") || l.starts_with("pub mod ") || l.contains(" mod "))
            .collect();
        let expected: Vec<&str> = if name == "lib.rs" { vec!["mod adapter;", "mod window;"] } else { vec![] };
        assert_eq!(mods, expected, "{name}'s modules");
    }
}

#[test]
fn code_lines_leaves_out_comments() {
    let source = "//! unsafe in a doc\n/// unsafe\n    // unsafe\nlet x = unsafe { y };\n";
    assert_eq!(code_lines(source).collect::<Vec<_>>(), [(3, "let x = unsafe { y };")]);
}
