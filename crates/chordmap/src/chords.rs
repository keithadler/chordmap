//! Major and minor triads per beat, smoothed with Viterbi decoding.

/// 0..12 major roots, 12..24 minor roots, 24 = no chord.
pub const N_STATES: usize = 25;
pub const NO_CHORD: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub root: Option<usize>,
    pub minor: bool,
}

impl Chord {
    pub fn from_state(s: usize) -> Self {
        if s == NO_CHORD {
            Chord {
                root: None,
                minor: false,
            }
        } else {
            Chord {
                root: Some(s % 12),
                minor: s >= 12,
            }
        }
    }
}

fn templates() -> Vec<[f32; 12]> {
    let mut v = Vec::with_capacity(N_STATES);
    for minor in [false, true] {
        for root in 0..12 {
            let mut t = [0.0f32; 12];
            t[root] = 1.0;
            t[(root + if minor { 3 } else { 4 }) % 12] = 1.0;
            t[(root + 7) % 12] = 1.0;
            let n = 3f32.sqrt();
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
    let mut emit = vec![0.0f32; n * N_STATES];
    for b in 0..n {
        let c = &beat_chroma[b * 12..b * 12 + 12];
        let quiet = beat_energy[b] < 0.03 * median;
        for (s, t) in tpl.iter().enumerate() {
            let sim: f32 = c.iter().zip(t.iter()).map(|(a, b)| a * b).sum();
            let sim = if s == NO_CHORD {
                if quiet {
                    1.0
                } else {
                    0.0
                }
            } else if quiet {
                0.0
            } else {
                sim
            };
            emit[b * N_STATES + s] = emit_w * sim;
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
