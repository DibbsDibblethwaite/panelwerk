//! Parameter and list IDs: lower-case ASCII letters, digits and "_", as
//! Kör's and Fläns's frozen-ID tests require.

use panelwerk_text::ids::is_id;

#[test]
fn ids_are_lower_case_ascii_digits_and_underscores() {
    for id in ["mode", "i_ii", "1d", "16bar", "noise_fade_in", "osc2_semi", "_", "0"] {
        assert!(is_id(id), "{id:?}");
    }
    for id in ["", "Mode", "fade-in", "fade in", "1/4", "kör", "a.b", "A"] {
        assert!(!is_id(id), "{id:?}");
    }
}
