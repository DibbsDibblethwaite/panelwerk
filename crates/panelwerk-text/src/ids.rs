//! The family's convention for parameter IDs and list IDs: lower-case ASCII
//! letters, digits and `_`. An ID is permanent once a build has been given
//! out (each consumer's own rule), so a consumer's frozen-ID test checks
//! each of its IDs with [`is_id`] as well as listing them.

/// Whether `text` is a well-formed parameter or list ID: not empty, and
/// only lower-case ASCII letters, digits and `_`.
///
/// ```
/// use panelwerk_text::ids::is_id;
///
/// assert!(is_id("noise_fade_in"));
/// assert!(is_id("16bar"));
/// assert!(!is_id("Fade In"));
/// assert!(!is_id(""));
/// ```
pub fn is_id(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
