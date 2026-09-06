//! Capo recommendation: the fret that turns the most of the song into open
//! chord shapes, weighted by how long each chord is played.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How hard a shape is with no capo, 0 = open chord, 2.5 = full barre.
pub fn difficulty(root: usize, minor: bool) -> f32 {
    match (root % 12, minor) {
        // Open shapes: C D E G A, Em Am Dm.
        (0, false) | (2, false) | (4, false) | (7, false) | (9, false) => 0.0,
        (4, true) | (9, true) | (2, true) => 0.0,
        // F: the small barre or the four-string version.
        (5, false) => 0.5,
        // Bm and F#m: the barre shapes every guitarist meets first.
        (11, true) => 1.5,
        (6, true) => 2.0,
        // B and Bb: full barres.
        (11, false) | (10, false) => 2.0,
        _ => 2.5,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapoOption {
    pub capo: usize,
    /// Seconds of the song spent on barre or otherwise hard shapes.
    pub hard_seconds: f32,
    /// Weighted difficulty, lower is easier.
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Guitar {
    /// Recommended capo fret, 0 = none.
    pub capo: usize,
    /// Sounding chord to the shape to play with the capo on.
    pub shapes: BTreeMap<String, String>,
    /// Every fret from 0 to 7, in fret order.
    pub options: Vec<CapoOption>,
}

/// `chords` are (root, minor, seconds); `label` names a sounding chord and
/// `shape` names the transposed shape.
pub fn recommend(
    chords: &[(Option<usize>, bool, f32)],
    label: impl Fn(usize, bool) -> String,
    shape: impl Fn(usize, bool) -> String,
) -> Guitar {
    let mut options = Vec::with_capacity(8);
    for capo in 0..=7usize {
        let mut score = 0.0;
        let mut hard = 0.0;
        for &(root, minor, secs) in chords {
            if let Some(r) = root {
                let d = difficulty((r + 12 - capo) % 12, minor);
                score += d * secs;
                if d >= 1.5 {
                    hard += secs;
                }
            }
        }
        // A capo has a cost of its own, so it only wins when it removes real barre time.
        score += 0.05 * capo as f32 * chords.iter().map(|c| c.2).sum::<f32>();
        options.push(CapoOption {
            capo,
            hard_seconds: (hard * 10.0f32).round() / 10.0,
            score: (score * 10.0).round() / 10.0,
        });
    }
    let capo = options
        .iter()
        .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        .map_or(0, |o| o.capo);
    let mut shapes = BTreeMap::new();
    for &(root, minor, _) in chords {
        if let Some(r) = root {
            shapes
                .entry(label(r, minor))
                .or_insert_with(|| shape((r + 12 - capo) % 12, minor));
        }
    }
    Guitar {
        capo,
        shapes,
        options,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bb_song_gets_capo_three() {
        // Bb F Gm Eb: with capo 3 that is G D Em C.
        let chords = vec![
            (Some(10), false, 60.0),
            (Some(5), false, 60.0),
            (Some(7), true, 60.0),
            (Some(3), false, 60.0),
        ];
        let g = recommend(
            &chords,
            |r, m| format!("{r}{}", if m { "m" } else { "" }),
            |r, m| format!("{r}{}", if m { "m" } else { "" }),
        );
        assert_eq!(g.capo, 3, "{:?}", g.options);
        assert_eq!(g.shapes.get("10").map(String::as_str), Some("7"));
        assert_eq!(g.shapes.get("7m").map(String::as_str), Some("4m"));
    }

    #[test]
    fn open_song_keeps_no_capo() {
        let chords = vec![
            (Some(0), false, 10.0),
            (Some(7), false, 10.0),
            (Some(9), true, 10.0),
            (Some(5), false, 10.0),
        ];
        let g = recommend(&chords, |r, _| r.to_string(), |r, _| r.to_string());
        assert_eq!(g.capo, 0, "{:?}", g.options);
    }
}
