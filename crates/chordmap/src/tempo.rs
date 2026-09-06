//! Tempo from the autocorrelation of the onset curve, beats by dynamic
//! programming (Ellis 2007).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoCandidate {
    pub bpm: f32,
    /// Prior-weighted autocorrelation, higher is better.
    pub score: f32,
}

const MIN_BPM: f32 = 40.0;
const MAX_BPM: f32 = 240.0;

fn prior(bpm: f32) -> f32 {
    let d = (bpm / 120.0).log2();
    (-0.5 * d * d).exp()
}

/// Ranked tempo candidates. With a hint, the candidate nearest the hint
/// is moved to the front.
pub fn candidates(onset: &[f32], fps: f32, hint: Option<f32>) -> Vec<TempoCandidate> {
    let n = onset.len();
    if n < 8 {
        return vec![TempoCandidate {
            bpm: 120.0,
            score: 0.0,
        }];
    }
    let mean = onset.iter().sum::<f32>() / n as f32;
    let oe: Vec<f32> = onset.iter().map(|v| v - mean).collect();
    let max_lag = ((60.0 * fps / MIN_BPM).ceil() as usize).min(n - 1);
    let min_lag = ((60.0 * fps / MAX_BPM).floor() as usize).max(1);
    let mut ac = vec![0.0f32; max_lag + 2];
    let e0: f32 = oe.iter().map(|v| v * v).sum::<f32>().max(1e-9);
    for (lag, a) in ac.iter_mut().enumerate().take(max_lag + 2) {
        let mut s = 0.0;
        for i in lag..n {
            s += oe[i] * oe[i - lag];
        }
        *a = s / e0;
    }
    let score = |lag: usize| ac[lag] * prior(60.0 * fps / lag as f32);
    let mut peaks: Vec<TempoCandidate> = Vec::new();
    for lag in min_lag.max(2)..=max_lag {
        if lag + 1 >= ac.len() {
            break;
        }
        if ac[lag] > ac[lag - 1] && ac[lag] >= ac[lag + 1] && ac[lag] > 0.0 {
            // Parabolic refinement of the peak position.
            let (a, b, c) = (ac[lag - 1], ac[lag], ac[lag + 1]);
            let denom = a - 2.0 * b + c;
            let off = if denom.abs() > 1e-9 {
                0.5 * (a - c) / denom
            } else {
                0.0
            };
            let l = lag as f32 + off.clamp(-0.5, 0.5);
            peaks.push(TempoCandidate {
                bpm: 60.0 * fps / l,
                score: score(lag),
            });
        }
    }
    if peaks.is_empty() {
        peaks.push(TempoCandidate {
            bpm: 120.0,
            score: 0.0,
        });
    }
    peaks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    peaks.truncate(8);
    if let Some(h) = hint {
        if h > 0.0 {
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for (i, p) in peaks.iter().enumerate() {
                let d = (p.bpm / h).log2().abs();
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            if best_d < 0.12 {
                let p = peaks.remove(best);
                peaks.insert(0, p);
            } else {
                peaks.insert(0, TempoCandidate { bpm: h, score: 0.0 });
            }
        }
    }
    peaks
}

/// Beat frame indices for a given tempo.
pub fn track_beats(onset: &[f32], fps: f32, bpm: f32) -> Vec<usize> {
    let n = onset.len();
    if n == 0 || bpm <= 0.0 {
        return Vec::new();
    }
    let period = 60.0 * fps / bpm;
    // Smooth the onset curve with a narrow Gaussian, as librosa does.
    let std = (period / 32.0).max(0.5);
    let half = (3.0 * std).ceil() as i64;
    let onset_std = {
        let m = onset.iter().sum::<f32>() / n as f32;
        (onset.iter().map(|v| (v - m) * (v - m)).sum::<f32>() / n as f32)
            .sqrt()
            .max(1e-6)
    };
    let local: Vec<f32> = (0..n as i64)
        .map(|t| {
            let mut s = 0.0;
            let mut w = 0.0;
            for k in -half..=half {
                let i = t + k;
                if i >= 0 && i < n as i64 {
                    let g = (-0.5 * (k as f32 / std).powi(2)).exp();
                    s += g * onset[i as usize] / onset_std;
                    w += g;
                }
            }
            s / w.max(1e-9)
        })
        .collect();
    let tightness = 100.0f32;
    let lo = (period / 2.0).round() as i64;
    let hi = (period * 2.0).round() as i64;
    let mut cum = vec![0.0f32; n];
    let mut back = vec![-1i64; n];
    let mut started = false;
    for i in 0..n as i64 {
        let mut best = f32::MIN;
        let mut best_j = -1i64;
        for d in lo..=hi {
            let j = i - d;
            if j < 0 {
                break;
            }
            let tx = -tightness * ((d as f32 / period).ln()).powi(2);
            let v = cum[j as usize] + tx;
            if v > best {
                best = v;
                best_j = j;
            }
        }
        let mut score = local[i as usize];
        if best_j >= 0 && (started || best > 0.0 || i >= lo) {
            score += best;
            back[i as usize] = best_j;
            started = true;
        }
        cum[i as usize] = score;
    }
    // Start backtracking from the last strong local maximum of the cumulative score.
    let mut maxima = Vec::new();
    for i in 1..n.saturating_sub(1) {
        if cum[i] > cum[i - 1] && cum[i] >= cum[i + 1] {
            maxima.push(i);
        }
    }
    if maxima.is_empty() {
        return Vec::new();
    }
    let mut vals: Vec<f32> = maxima.iter().map(|&i| cum[i]).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = vals[vals.len() / 2];
    let tail = *maxima
        .iter()
        .rev()
        .find(|&&i| cum[i] >= 0.5 * median)
        .unwrap_or(&maxima[maxima.len() - 1]);
    let mut beats = vec![tail];
    let mut cur = back[tail];
    while cur >= 0 {
        beats.push(cur as usize);
        cur = back[cur as usize];
    }
    beats.reverse();
    // Drop weak beats at the edges (silence before the music starts).
    let rms = (beats.iter().map(|&b| local[b] * local[b]).sum::<f32>() / beats.len() as f32).sqrt();
    let strong = |b: usize| local[b] >= 0.5 * rms;
    while beats.len() > 2 && !strong(beats[0]) {
        beats.remove(0);
    }
    while beats.len() > 2 && !strong(*beats.last().unwrap()) {
        beats.pop();
    }
    beats
}
