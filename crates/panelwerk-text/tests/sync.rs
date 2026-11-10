//! The tempo-sync length names and list IDs, frozen (PW-3, CLAUDE.md rule 6):
//! every name and ID written out literally, so a change to any word fails
//! here. Rödalog's presets save a list's choice by its shown name (its
//! B-PRE-3), and Kör's and Fläns's states by the list ID, so both are saved
//! keys.

use panelwerk_text::sync::{ids, names, NoteLength, NoteLength::*};

/// Every length, its name (FAM-1, Rödalog's words) and its list ID (Kör's
/// and Fläns's), in `ALL`'s order: frozen (PW-3).
const FROZEN: [(NoteLength, &str, &str); 23] = [
    (Bars(16), "16 bars", "16bar"),
    (Bars(8), "8 bars", "8bar"),
    (Bars(4), "4 bars", "4bar"),
    (Bars(2), "2 bars", "2bar"),
    (Bars(1), "1 bar", "1bar"),
    (Note(1), "1/1", "1"),
    (Dotted(1), "1/1 dotted", "1d"),
    (Triplet(1), "1/1 triplet", "1t"),
    (Note(2), "1/2", "2"),
    (Dotted(2), "1/2 dotted", "2d"),
    (Triplet(2), "1/2 triplet", "2t"),
    (Note(4), "1/4", "4"),
    (Dotted(4), "1/4 dotted", "4d"),
    (Triplet(4), "1/4 triplet", "4t"),
    (Note(8), "1/8", "8"),
    (Dotted(8), "1/8 dotted", "8d"),
    (Triplet(8), "1/8 triplet", "8t"),
    (Note(16), "1/16", "16"),
    (Dotted(16), "1/16 dotted", "16d"),
    (Triplet(16), "1/16 triplet", "16t"),
    (Note(32), "1/32", "32"),
    (Dotted(32), "1/32 dotted", "32d"),
    (Triplet(32), "1/32 triplet", "32t"),
];

#[test]
fn pw_3_all_is_every_length_longest_first() {
    assert_eq!(NoteLength::ALL, FROZEN.map(|(l, _, _)| l));
}

#[test]
fn pw_3_fam_1_every_name_is_frozen() {
    for (length, name, _) in FROZEN {
        assert_eq!(length.name(), name, "{length:?}");
    }
}

#[test]
fn pw_3_every_list_id_is_frozen() {
    for (length, _, id) in FROZEN {
        assert_eq!(length.id(), id, "{length:?}");
    }
}

#[test]
fn pw_3_names_and_ids_are_unique() {
    let mut names = NoteLength::ALL.map(NoteLength::name).to_vec();
    let mut ids = NoteLength::ALL.map(NoteLength::id).to_vec();
    names.sort_unstable();
    names.dedup();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!((names.len(), ids.len()), (23, 23));
}

#[test]
fn every_list_id_is_an_id() {
    for length in NoteLength::ALL {
        assert!(panelwerk_text::ids::is_id(length.id()), "{length:?}");
    }
}

#[test]
#[should_panic(expected = "not a length the family names")]
fn name_panics_for_a_length_not_in_all() {
    let _ = Note(3).name();
}

#[test]
#[should_panic(expected = "not a length the family names")]
fn id_panics_for_a_length_not_in_all() {
    let _ = Bars(3).id();
}

/// Each consumer's list as it is today (Rödalog `v3` `c75580d`'s
/// `NoteChoice`; Kör `main` `e97fc99`'s `SyncRate`; Fläns `main`
/// `4cfe9a5`'s `SyncRateParam`), in its order, with the names and IDs it
/// will show once it adopts: the tables a consumer's one-line test (PW-4)
/// compares its literals with.
#[test]
fn pw_4_each_consumers_list_reads_as_today() {
    const RODALOG: [NoteLength; 16] = [
        Bars(4),
        Bars(2),
        Bars(1),
        Dotted(2),
        Note(2),
        Dotted(4),
        Triplet(2),
        Note(4),
        Dotted(8),
        Triplet(4),
        Note(8),
        Dotted(16),
        Triplet(8),
        Note(16),
        Triplet(16),
        Note(32),
    ];
    assert_eq!(
        names(RODALOG),
        [
            "4 bars",
            "2 bars",
            "1 bar",
            "1/2 dotted",
            "1/2",
            "1/4 dotted",
            "1/2 triplet",
            "1/4",
            "1/8 dotted",
            "1/4 triplet",
            "1/8",
            "1/16 dotted",
            "1/8 triplet",
            "1/16",
            "1/16 triplet",
            "1/32",
        ]
    );
    // Kör's 18 and Fläns's 22: Fläns's list is 16 … 2 bars, then Kör's.
    let kör: [NoteLength; 18] = std::array::from_fn(|i| NoteLength::ALL[i + 5]);
    assert_eq!(
        ids(kör),
        ["1", "1d", "1t", "2", "2d", "2t", "4", "4d", "4t", "8", "8d", "8t", "16", "16d", "16t", "32", "32d", "32t"]
    );
    let fläns: [NoteLength; 22] = std::array::from_fn(|i| NoteLength::ALL[if i < 4 { i } else { i + 1 }]);
    assert_eq!(&ids(fläns)[..4], ["16bar", "8bar", "4bar", "2bar"]);
    assert_eq!(&ids(fläns)[4..], ids(kör));
    assert_eq!(
        names(fläns),
        [
            "16 bars",
            "8 bars",
            "4 bars",
            "2 bars",
            "1/1",
            "1/1 dotted",
            "1/1 triplet",
            "1/2",
            "1/2 dotted",
            "1/2 triplet",
            "1/4",
            "1/4 dotted",
            "1/4 triplet",
            "1/8",
            "1/8 dotted",
            "1/8 triplet",
            "1/16",
            "1/16 dotted",
            "1/16 triplet",
            "1/32",
            "1/32 dotted",
            "1/32 triplet",
        ]
    );
}

#[test]
fn names_and_ids_work_at_compile_time() {
    const LIST: [NoteLength; 2] = [Note(4), Dotted(8)];
    const NAMES: [&str; 2] = names(LIST);
    const IDS: [&str; 2] = ids(LIST);
    assert_eq!((NAMES, IDS), (["1/4", "1/8 dotted"], ["4", "8d"]));
}

#[test]
fn from_name_takes_exactly_a_shown_name() {
    for (length, name, _) in FROZEN {
        assert_eq!(NoteLength::from_name(name), Some(length));
    }
    assert_eq!(NoteLength::from_name("1/4 Dotted"), None);
    assert_eq!(NoteLength::from_name(" 1/4"), None);
    assert_eq!(NoteLength::from_name("4"), None);
}

#[test]
fn from_id_takes_exactly_a_list_id() {
    for (length, _, id) in FROZEN {
        assert_eq!(NoteLength::from_id(id), Some(length));
    }
    assert_eq!(NoteLength::from_id("4D"), None);
    assert_eq!(NoteLength::from_id("1/4"), None);
}

/// Typed text: each name in any case and spacing, a bar count singular or
/// plural, and Kör's former short forms, so what a user is used to typing
/// still works. Bare IDs aren't taken: "4" would read as 1/4, not 4 bars.
#[test]
fn parse_reads_typed_text() {
    for (length, name, _) in FROZEN {
        assert_eq!(NoteLength::parse(name), Some(length), "{name}");
        assert_eq!(NoteLength::parse(&name.to_uppercase()), Some(length), "{name}");
    }
    let typed = [
        ("  1/4   dotted ", Dotted(4)),
        ("1/4dotted", Dotted(4)),
        ("1/16 Triplet", Triplet(16)),
        ("4 bar", Bars(4)),
        ("4bars", Bars(4)),
        ("4bar", Bars(4)),
        ("1 bars", Bars(1)),
        ("16 BARS", Bars(16)),
        // Kör's former names ("1/4.", "1/4T"), and the ID after "1/".
        ("1/4.", Dotted(4)),
        ("1/1T", Triplet(1)),
        ("1/32t", Triplet(32)),
        ("1/8d", Dotted(8)),
        ("1/8 d", Dotted(8)),
    ];
    for (text, length) in typed {
        assert_eq!(NoteLength::parse(text), Some(length), "{text:?}");
    }
    for text in ["", "4", "16t", "1/3", "1/64", "3 bars", "32 bars", "1/4 dotted triplet", "quarter"] {
        assert_eq!(NoteLength::parse(text), None, "{text:?}");
    }
}
