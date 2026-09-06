//! Pitch-class names and key-aware spelling.

use crate::chords::Quality;

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

/// Spell a root in a key: diatonic roots as the key writes them, chromatic
/// roots as flats, except the raised fourth (the V of V) and, in minor, the
/// raised seventh, which are sharps.
pub fn spell(pc: usize, tonic: usize, minor: bool) -> &'static str {
    let pc = pc % 12;
    if key_uses_flats(tonic, minor) {
        return FLAT[pc];
    }
    let major_tonic = if minor { (tonic + 3) % 12 } else { tonic };
    let diatonic = [0, 2, 4, 5, 7, 9, 11]
        .iter()
        .any(|d| (major_tonic + d) % 12 == pc);
    let raised_seventh = minor && pc == (tonic + 11) % 12;
    if diatonic || raised_seventh || pc == (major_tonic + 6) % 12 {
        SHARP[pc]
    } else {
        FLAT[pc]
    }
}

pub fn name(pc: usize, flats: bool) -> &'static str {
    if flats {
        FLAT[pc % 12]
    } else {
        SHARP[pc % 12]
    }
}

/// "Am", "Bb", "G7", "Cmaj7", "N" (no chord), spelled for a key.
pub fn chord_label(root: Option<usize>, quality: Quality, tonic: usize, minor: bool) -> String {
    match root {
        None => "N".to_string(),
        Some(r) => format!("{}{}", spell(r, tonic, minor), quality.suffix()),
    }
}

/// "Bbm7" -> (10, Min7). Accepts sharps and flats.
pub fn parse_label(label: &str) -> Option<(usize, Quality)> {
    let (name, quality) = Quality::parse_suffix(label)?;
    SHARP
        .iter()
        .position(|&x| x == name)
        .or_else(|| FLAT.iter().position(|&x| x == name))
        .map(|r| (r, quality))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip() {
        for q in Quality::ALL {
            for r in 0..12 {
                for tonic in [0, 7, 10] {
                    let l = chord_label(Some(r), q, tonic, false);
                    assert_eq!(parse_label(&l), Some((r, q)), "{l}");
                }
            }
        }
        assert_eq!(parse_label("N"), None);
    }

    #[test]
    fn chromatic_roots_are_flats_except_the_raised_fourth() {
        assert_eq!(spell(3, 7, false), "Eb"); // bVI in G major
        assert_eq!(spell(6, 7, false), "F#"); // diatonic in G
        assert_eq!(spell(1, 7, false), "C#"); // V of V in G
        assert_eq!(spell(10, 0, false), "Bb"); // bVII in C
        assert_eq!(spell(8, 9, true), "G#"); // raised seventh in A minor
    }
}
