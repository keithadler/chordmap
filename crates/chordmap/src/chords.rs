//! Chords per beat: major, minor, dominant seventh, major seventh and minor
//! seventh, smoothed with Viterbi decoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quality {
    Major,
    Minor,
    Dom7,
    Maj7,
    Min7,
}

impl Quality {
    pub const ALL: [Quality; 5] = [
        Quality::Major,
        Quality::Minor,
        Quality::Dom7,
        Quality::Maj7,
        Quality::Min7,
    ];

    /// Intervals above the root and their template weights.
    fn tones(self) -> &'static [(usize, f32)] {
        match self {
            Quality::Major => &[(0, 1.0), (4, 1.0), (7, 1.0)],
            Quality::Minor => &[(0, 1.0), (3, 1.0), (7, 1.0)],
            Quality::Dom7 => &[(0, 1.0), (4, 1.0), (7, 1.0), (10, 0.8)],
            Quality::Maj7 => &[(0, 1.0), (4, 1.0), (7, 1.0), (11, 0.8)],
            Quality::Min7 => &[(0, 1.0), (3, 1.0), (7, 1.0), (10, 0.8)],
        }
    }

    /// Label suffix: "", "m", "7", "maj7", "m7".
    pub fn suffix(self) -> &'static str {
        match self {
            Quality::Major => "",
            Quality::Minor => "m",
            Quality::Dom7 => "7",
            Quality::Maj7 => "maj7",
            Quality::Min7 => "m7",
        }
    }

    pub fn is_minor(self) -> bool {
        matches!(self, Quality::Minor | Quality::Min7)
    }

    pub fn is_seventh(self) -> bool {
        matches!(self, Quality::Dom7 | Quality::Maj7 | Quality::Min7)
    }

    /// Longest suffix first so "maj7" is not read as "m".
    pub fn parse_suffix(s: &str) -> Option<(&str, Quality)> {
        for q in [
            Quality::Maj7,
            Quality::Min7,
            Quality::Dom7,
            Quality::Minor,
            Quality::Major,
        ] {
            if let Some(rest) = s.strip_suffix(q.suffix()) {
                return Some((rest, q));
            }
        }
        None
    }
}

/// States 0..60 are quality-major blocks of 12 roots; 60 = no chord.
pub const N_STATES: usize = 61;
pub const NO_CHORD: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub root: Option<usize>,
    pub quality: Quality,
}

impl Chord {
    pub fn from_state(s: usize) -> Self {
        if s == NO_CHORD {
            Chord {
                root: None,
                quality: Quality::Major,
            }
        } else {
            Chord {
                root: Some(s % 12),
                quality: Quality::ALL[s / 12],
            }
        }
    }
    pub fn state(root: usize, quality: Quality) -> usize {
        Quality::ALL.iter().position(|&q| q == quality).unwrap() * 12 + root % 12
    }
}

fn templates() -> Vec<[f32; 12]> {
    let mut v = Vec::with_capacity(N_STATES);
    for q in Quality::ALL {
        for root in 0..12 {
            let mut t = [0.0f32; 12];
            for &(iv, w) in q.tones() {
                t[(root + iv) % 12] = w;
            }
            let n = t.iter().map(|x| x * x).sum::<f32>().sqrt();
            for x in t.iter_mut() {
                *x /= n;
            }
            v.push(t);
        }
    }
    v.push([1.0 / 12f32.sqrt(); 12]);
    v
}

/// Chord state per beat.
pub fn decode(beat_chroma: &[f32], beat_energy: &[f32]) -> Vec<usize> {
    let n = beat_energy.len();
    if n == 0 {
        return Vec::new();
    }
    let tpl = templates();
    let mut sorted = beat_energy.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[n / 2].max(1e-12);
    let emit_w = 10.0f32;
    let change = 1.5f32;
    // Sevenths must earn their place: a triad wins a near tie.
    let seventh_cost = 0.25f32;
    let mut emit = vec![0.0f32; n * N_STATES];
    for b in 0..n {
        let c = &beat_chroma[b * 12..b * 12 + 12];
        let quiet = beat_energy[b] < 0.03 * median;
        for (s, t) in tpl.iter().enumerate() {
            let sim: f32 = c.iter().zip(t.iter()).map(|(a, b)| a * b).sum();
            let e = if s == NO_CHORD {
                if quiet {
                    emit_w
                } else {
                    0.0
                }
            } else if quiet {
                0.0
            } else {
                emit_w * sim
                    - if Chord::from_state(s).quality.is_seventh() {
                        seventh_cost
                    } else {
                        0.0
                    }
            };
            emit[b * N_STATES + s] = e;
        }
    }
    let mut score = vec![f32::MIN; n * N_STATES];
    let mut back = vec![0usize; n * N_STATES];
    score[..N_STATES].copy_from_slice(&emit[..N_STATES]);
    for b in 1..n {
        let prev: Vec<f32> = score[(b - 1) * N_STATES..b * N_STATES].to_vec();
        let (best_prev, best_val) =
            prev.iter()
                .enumerate()
                .fold((0, f32::MIN), |m, (i, &v)| if v > m.1 { (i, v) } else { m });
        for s in 0..N_STATES {
            let stay = prev[s];
            let (from, val) = if stay >= best_val - change {
                (s, stay)
            } else {
                (best_prev, best_val - change)
            };
            score[b * N_STATES + s] = val + emit[b * N_STATES + s];
            back[b * N_STATES + s] = from;
        }
    }
    let last = &score[(n - 1) * N_STATES..];
    let mut s = last
        .iter()
        .enumerate()
        .fold((0, f32::MIN), |m, (i, &v)| if v > m.1 { (i, v) } else { m })
        .0;
    let mut path = vec![0usize; n];
    for b in (0..n).rev() {
        path[b] = s;
        s = back[b * N_STATES + s];
    }
    path
}

/// How chord-like the beats are: the energy-weighted mean of the best
/// triad-or-seventh match minus what random chroma would score. About 0.7
/// for a strummed song, under 0.4 for drums with a bass line and a rap.
pub fn harmonicity(beat_chroma: &[f32], beat_energy: &[f32]) -> f32 {
    let n = beat_energy.len();
    if n == 0 {
        return 0.0;
    }
    let tpl = templates();
    let mut acc = 0.0;
    let mut wsum = 0.0;
    for b in 0..n {
        let c = &beat_chroma[b * 12..b * 12 + 12];
        let best = tpl[..NO_CHORD]
            .iter()
            .map(|t| c.iter().zip(t.iter()).map(|(a, b)| a * b).sum::<f32>())
            .fold(0.0f32, f32::max);
        // Flat chroma scores about 0.5 against a triad; a clean triad scores 1.
        // Noise still finds some triad, so also ask how peaked the chroma is:
        // the top three pitch classes hold nearly everything for a chord and
        // about a third of it for drums or noise.
        let mut sorted = c.to_vec();
        sorted.sort_by(|x, y| y.partial_cmp(x).unwrap());
        let total: f32 = sorted.iter().sum::<f32>().max(1e-9);
        let peak = ((sorted[0] + sorted[1] + sorted[2]) / total - 0.35) / 0.55;
        let w = beat_energy[b].max(0.0);
        acc += ((best - 0.5) * 2.0).clamp(0.0, 1.0) * peak.clamp(0.0, 1.0) * w;
        wsum += w;
    }
    if wsum > 0.0 {
        acc / wsum
    } else {
        0.0
    }
}
