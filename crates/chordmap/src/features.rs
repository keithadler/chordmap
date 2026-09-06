//! Frame-level features: a semitone spectrogram for harmony, log-mel bands
//! for timbre, and an onset strength curve for rhythm.

use crate::dsp::{self, Stft, SR};

pub const N_MELS: usize = 40;
/// Semitones from MIDI 36 (C2) to 95 (B6).
pub const N_PITCH: usize = 60;
pub const PITCH_MIDI0: i32 = 36;

pub struct Features {
    pub fps: f32,
    pub n: usize,
    /// `n * N_PITCH` linear energies.
    pub pitch: Vec<f32>,
    /// `n * N_MELS` log power in dB, clamped to an 80 dB range.
    pub mel: Vec<f32>,
    /// `n` onset strengths, non-negative, mean about 1.
    pub onset: Vec<f32>,
    /// `n` low-band levels in dB (kick and bass), for downbeats.
    pub bass: Vec<f32>,
}

fn hz_to_mel(f: f32) -> f32 {
    2595.0 * (1.0 + f / 700.0).log10()
}
fn mel_to_hz(m: f32) -> f32 {
    700.0 * (10f32.powf(m / 2595.0) - 1.0)
}

/// Triangular mel filters as sparse (bin, weight) lists.
fn mel_filters() -> Vec<Vec<(usize, f32)>> {
    let max_mel = hz_to_mel(SR as f32 / 2.0);
    let pts: Vec<f32> = (0..N_MELS + 2)
        .map(|i| mel_to_hz(max_mel * i as f32 / (N_MELS + 1) as f32))
        .collect();
    (0..N_MELS)
        .map(|m| {
            let (lo, c, hi) = (pts[m], pts[m + 1], pts[m + 2]);
            let mut v = Vec::new();
            for b in 0..Stft::BINS {
                let f = Stft::bin_hz(b);
                let w = if f > lo && f < c {
                    (f - lo) / (c - lo)
                } else if f >= c && f < hi {
                    (hi - f) / (hi - c)
                } else {
                    0.0
                };
                if w > 0.0 {
                    v.push((b, w * 2.0 / (hi - lo)));
                }
            }
            v
        })
        .collect()
}

/// Each bin is shared between its two nearest semitones.
fn pitch_weights() -> Vec<(usize, usize, f32)> {
    let mut v = Vec::new();
    for b in 1..Stft::BINS {
        let f = Stft::bin_hz(b);
        if !(55.0..=2200.0).contains(&f) {
            continue;
        }
        let m = 69.0 + 12.0 * (f / 440.0).log2();
        let lo = m.floor();
        for p in [lo, lo + 1.0] {
            let w = 1.0 - (m - p).abs();
            let idx = p as i32 - PITCH_MIDI0;
            if w > 0.0 && idx >= 0 && (idx as usize) < N_PITCH {
                v.push((b, idx as usize, w));
            }
        }
    }
    v
}

pub fn extract(x: &[f32]) -> Features {
    let n = Stft::n_frames(x.len());
    let mut stft = Stft::new();
    let mels = mel_filters();
    let pw = pitch_weights();
    let mut spec = vec![0.0f32; Stft::BINS];
    let mut pitch = vec![0.0f32; n * N_PITCH];
    let mut mel = vec![0.0f32; n * N_MELS];
    let mut global_max = f32::MIN;
    for t in 0..n {
        stft.power(x, t, &mut spec);
        for &(b, p, w) in &pw {
            pitch[t * N_PITCH + p] += spec[b] * w;
        }
        for (m, filt) in mels.iter().enumerate() {
            let e: f32 = filt.iter().map(|&(b, w)| spec[b] * w).sum();
            let db = 10.0 * (e + 1e-10).log10();
            mel[t * N_MELS + m] = db;
            global_max = global_max.max(db);
        }
    }
    let floor = global_max - 80.0;
    for v in mel.iter_mut() {
        *v = v.max(floor);
    }
    // Spectral flux: positive band-wise change, averaged over bands.
    let mut onset = vec![0.0f32; n];
    for t in 1..n {
        let mut s = 0.0;
        for m in 0..N_MELS {
            let d = mel[t * N_MELS + m] - mel[(t - 1) * N_MELS + m];
            if d > 0.0 {
                s += d;
            }
        }
        onset[t] = s / N_MELS as f32;
    }
    let mean = onset.iter().sum::<f32>() / n.max(1) as f32;
    if mean > 1e-6 {
        for v in onset.iter_mut() {
            *v /= mean;
        }
    }
    let bass = (0..n)
        .map(|t| (0..5).map(|m| mel[t * N_MELS + m]).sum::<f32>() / 5.0)
        .collect();
    Features {
        fps: dsp::fps(),
        n,
        pitch,
        mel,
        onset,
        bass,
    }
}

/// Average rows of a `n * dim` matrix over segments `[bounds[i], bounds[i+1])`,
/// with a final segment running to `n`.
pub fn segment_mean(data: &[f32], dim: usize, n: usize, bounds: &[usize]) -> Vec<f32> {
    let mut out = Vec::with_capacity(bounds.len() * dim);
    for (i, &b0) in bounds.iter().enumerate() {
        let b1 = if i + 1 < bounds.len() {
            bounds[i + 1]
        } else {
            n
        };
        let b1 = b1.max(b0 + 1).min(n.max(b0 + 1));
        let mut acc = vec![0.0f32; dim];
        let mut cnt = 0usize;
        for t in b0..b1.min(n) {
            for d in 0..dim {
                acc[d] += data[t * dim + d];
            }
            cnt += 1;
        }
        if cnt > 0 {
            for a in acc.iter_mut() {
                *a /= cnt as f32;
            }
        }
        out.extend(acc);
    }
    out
}
