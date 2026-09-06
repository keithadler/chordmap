//! Twelve-bin pitch-class profiles from the semitone spectrogram.

use crate::features::N_PITCH;

/// Median filter along time for each semitone. Sustained harmonic energy
/// survives, broadband drum hits mostly do not.
pub fn harmonic(pitch: &[f32], n: usize, len: usize) -> Vec<f32> {
    let half = len / 2;
    let mut out = vec![0.0f32; n * N_PITCH];
    let mut buf = Vec::with_capacity(len);
    for p in 0..N_PITCH {
        for t in 0..n {
            buf.clear();
            let a = t.saturating_sub(half);
            let b = (t + half + 1).min(n);
            for u in a..b {
                buf.push(pitch[u * N_PITCH + p]);
            }
            buf.sort_by(|x, y| x.partial_cmp(y).unwrap());
            out[t * N_PITCH + p] = buf[buf.len() / 2];
        }
    }
    out
}

/// Log-compressed, octave-folded, per-frame normalised chroma (`n * 12`),
/// plus the linear energy per frame.
///
/// Only C2 to B5 are treated as fundamentals. Each of those pitches gains
/// support from its own octave, twelfth and double octave above, so a real
/// note with harmonics outranks a stray harmonic of a note below it. That
/// is what keeps a plain major triad from reading as a major seventh.
pub fn chroma(pitch: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
    const FOLD: usize = 48;
    const SUPPORT: [(usize, f32); 3] = [(12, 0.5), (19, 0.33), (24, 0.25)];
    const OCTAVE_WEIGHT: [f32; 4] = [1.0, 1.0, 0.8, 0.6];
    let max = pitch.iter().cloned().fold(0.0f32, f32::max).max(1e-12);
    let mut out = vec![0.0f32; n * 12];
    let mut energy = vec![0.0f32; n];
    for t in 0..n {
        let row = &pitch[t * N_PITCH..(t + 1) * N_PITCH];
        energy[t] = row.iter().sum();
        for p in 0..FOLD {
            let mut sal = row[p];
            for &(iv, w) in &SUPPORT {
                if p + iv < N_PITCH {
                    sal += w * row[p + iv];
                }
            }
            let c = (1.0 + 20.0 * sal / max).ln() * OCTAVE_WEIGHT[p / 12];
            out[t * 12 + p % 12] += c;
        }
        let norm = (0..12).map(|k| out[t * 12 + k].powi(2)).sum::<f32>().sqrt();
        if norm > 1e-9 {
            for k in 0..12 {
                out[t * 12 + k] /= norm;
            }
        }
    }
    (out, energy)
}

pub fn normalize_rows(data: &mut [f32], dim: usize) {
    for row in data.chunks_mut(dim) {
        let norm = row.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 1e-9 {
            for v in row.iter_mut() {
                *v /= norm;
            }
        }
    }
}
