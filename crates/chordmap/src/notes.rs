//! Pitch-class names and key-aware spelling.

pub const SHARP: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const FLAT: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];

/// Keys whose signatures use flats: F, Bb, Eb, Ab, Db, Gb and their relative minors.
pub fn key_uses_flats(tonic: usize, minor: bool) -> bool {
    let major_tonic = if minor { (tonic + 3) % 12 } else { tonic };
    matches!(major_tonic, 5 | 10 | 3 | 8 | 1 | 6)
}

pub fn name(pc: usize, flats: bool) -> &'static str {
    if flats {
        FLAT[pc % 12]
    } else {
        SHARP[pc % 12]
    }
}

/// "Am", "Bb", "N" (no chord).
pub fn chord_label(root: Option<usize>, minor: bool, flats: bool) -> String {
    match root {
        None => "N".to_string(),
        Some(r) => format!("{}{}", name(r, flats), if minor { "m" } else { "" }),
    }
}
