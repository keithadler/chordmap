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
pub fn chroma(pitch: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
    let max = pitch.iter().cloned().fold(0.0f32, f32::max).max(1e-12);
    let mut out = vec![0.0f32; n * 12];
    let mut energy = vec![0.0f32; n];
    for t in 0..n {
        let mut e = 0.0;
        for p in 0..N_PITCH {
            let v = pitch[t * N_PITCH + p];
            e += v;
            let c = (1.0 + 100.0 * v / max).ln();
            out[t * 12 + p % 12] += c;
        }
        energy[t] = e;
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
