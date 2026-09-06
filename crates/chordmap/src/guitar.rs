//! Capo recommendation: the fret that turns the most of the song into open
//! chord shapes, weighted by how long each chord is played.

use crate::chords::Quality;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How hard a shape is with no capo, 0 = open chord, 2.5 = full barre.
pub fn difficulty(root: usize, quality: Quality) -> f32 {
    let r = root % 12;
    match quality {
        Quality::Major => match r {
            0 | 2 | 4 | 7 | 9 => 0.0,
            5 => 0.5,
            10 | 11 => 2.0,
            _ => 2.5,
        },
        Quality::Minor => match r {
            2 | 4 | 9 => 0.0,
            11 => 1.5,
            6 => 2.0,
            _ => 2.5,
        },
        // Open sevenths: A7 B7 C7 D7 E7 G7.
        Quality::Dom7 => match r {
            9 | 11 | 0 | 2 | 4 | 7 => 0.0,
            5 => 1.0,
            _ => 2.5,
        },
        // Cmaj7 Fmaj7 Amaj7 Dmaj7 Emaj7 Gmaj7 all have easy open voicings.
        Quality::Maj7 => match r {
            0 | 5 | 9 | 2 | 4 | 7 => 0.5,
            _ => 2.5,
        },
        // Am7 Dm7 Em7 are open; Bm7 is a barre.
        Quality::Min7 => match r {
            9 | 2 | 4 => 0.0,
            11 => 1.5,
            6 => 2.0,
            _ => 2.5,
        },
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
    /// Sounding chord to the shape to play with the capo on, for chords
    /// that fill at least 2 percent of the song.
    pub shapes: BTreeMap<String, String>,
    /// Every fret from 0 to 7, in fret order.
    pub options: Vec<CapoOption>,
}

/// `chords` are (root, quality, seconds); `label` names a sounding chord and
/// `shape` names the transposed shape.
pub fn recommend(
    chords: &[(Option<usize>, Quality, f32)],
    label: impl Fn(usize, Quality) -> String,
    shape: impl Fn(usize, Quality) -> String,
) -> Guitar {
    let total: f32 = chords.iter().map(|c| c.2).sum();
    let mut options = Vec::with_capacity(8);
    for capo in 0..=7usize {
        let mut score = 0.0;
        let mut hard = 0.0;
        for &(root, quality, secs) in chords {
            if let Some(r) = root {
                let d = difficulty((r + 12 - capo) % 12, quality);
                score += d * secs;
                if d >= 1.5 {
                    hard += secs;
                }
            }
        }
        // A capo has a cost of its own, so it only wins when it removes real barre time.
        score += 0.05 * capo as f32 * total;
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
    // Shapes worth listing: chords that fill at least 2 percent of the song.
    let mut seconds: BTreeMap<(usize, Quality), f32> = BTreeMap::new();
    for &(root, quality, secs) in chords {
        if let Some(r) = root {
            *seconds.entry((r % 12, quality)).or_insert(0.0) += secs;
        }
    }
    let mut shapes = BTreeMap::new();
    for (&(r, quality), &secs) in &seconds {
        if secs >= 0.02 * total {
            shapes.insert(label(r, quality), shape((r + 12 - capo) % 12, quality));
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

    fn lab(r: usize, q: Quality) -> String {
        format!("{r}{}", q.suffix())
    }

    #[test]
    fn bb_song_gets_capo_three() {
        // Bb F Gm Eb: with capo 3 that is G D Em C.
        let c = vec![
            (Some(10), Quality::Major, 60.0),
            (Some(5), Quality::Major, 60.0),
            (Some(7), Quality::Minor, 60.0),
            (Some(3), Quality::Major, 60.0),
        ];
        let g = recommend(&c, lab, lab);
        assert_eq!(g.capo, 3, "{:?}", g.options);
        assert_eq!(g.shapes.get("10").map(String::as_str), Some("7"));
        assert_eq!(g.shapes.get("7m").map(String::as_str), Some("4m"));
    }

    #[test]
    fn open_song_keeps_no_capo() {
        let c = vec![
            (Some(0), Quality::Major, 10.0),
            (Some(7), Quality::Major, 10.0),
            (Some(9), Quality::Minor, 10.0),
            (Some(5), Quality::Major, 10.0),
        ];
        assert_eq!(recommend(&c, lab, lab).capo, 0);
    }

    #[test]
    fn sevenths_have_open_shapes() {
        assert_eq!(difficulty(4, Quality::Dom7), 0.0);
        assert_eq!(difficulty(11, Quality::Dom7), 0.0);
        assert!(difficulty(1, Quality::Min7) >= 2.0);
    }
}
