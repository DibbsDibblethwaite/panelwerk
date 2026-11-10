//! The settings file (Kör B-PNL-7, PW-8): the last zoom the user chose, in
//! `%APPDATA%\<the consumer's folder>\panel.json`, for new instances. Read
//! and written only on the GUI thread (a consumer calls these from its
//! window's code, never `process`).
//!
//! ```
//! use std::ffi::OsString;
//! use panelwerk_panel::{settings, zoom::Range};
//!
//! const KOER: Range = Range { min: 0.75, max: 3.0, menu: &[1.0] };
//! let file = settings::file_in(Some(OsString::from("/appdata")), "Koer").unwrap();
//! assert!(file.ends_with("Koer/panel.json"));
//! // A new instance: no zoom in its project, none in the file.
//! assert_eq!(settings::opening_zoom(None, settings::read(&file, &KOER), &KOER), 1.0);
//! ```

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::zoom::Range;

/// The settings file in `folder` (the consumer's, "Koer") of the folder
/// `appdata` names (`%APPDATA%`'s value); none without it.
pub fn file_in(appdata: Option<OsString>, folder: &str) -> Option<PathBuf> {
    appdata.filter(|a| !a.is_empty()).map(|a| PathBuf::from(a).join(folder).join("panel.json"))
}

/// The zoom in the file at `path`, held to `range`; none if the file is
/// missing or unreadable or holds no zoom (the panel then opens at 100 %).
pub fn read(path: &Path, range: &Range) -> Option<f32> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let zoom = value.get("zoom")?.as_f64()? as f32;
    zoom.is_finite().then(|| range.clamp(zoom))
}

/// Writes `zoom`, held to `range`, to the file at `path`, making its
/// folder, through a temporary file and a rename so a failed write leaves
/// the old file whole.
pub fn write(path: &Path, zoom: f32, range: &Range) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, format!("{{\"zoom\": {}}}\n", range.clamp(zoom)))?;
    std::fs::rename(&temporary, path)
}

/// The zoom a window opens at: the project's, if its state has one; else
/// the settings file's; else 100 %; held to `range`.
pub fn opening_zoom(project: Option<f32>, file: Option<f32>, range: &Range) -> f32 {
    range.clamp(project.or(file).unwrap_or(1.0))
}
