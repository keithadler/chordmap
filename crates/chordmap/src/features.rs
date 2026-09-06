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
    /// Estimated deviation from A440 in cents; the semitone grid was shifted by it.
    pub tuning_cents: f32,
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

/// Histogram of how far spectral peaks sit from the nearest equal-tempered
/// semitone, over a subset of frames. Returns the modal offset in cents.
fn estimate_tuning(stft: &mut Stft, x: &[f32], n: usize) -> f32 {
    let mut hist = [0.0f32; 20]; // 5-cent bins from -50 to +50
    let mut spec = vec![0.0f32; Stft::BINS];
    let mut t = 0;
    while t < n {
        stft.power(x, t, &mut spec);
        let max = spec.iter().cloned().fold(0.0f32, f32::max);
        if max > 1e-6 {
            for b in 2..Stft::BINS - 2 {
                let f = Stft::bin_hz(b);
                if !(80.0..=2000.0).contains(&f) {
                    continue;
                }
                if spec[b] > spec[b - 1] && spec[b] >= spec[b + 1] && spec[b] > 1e-3 * max {
                    // Parabolic interpolation of the peak frequency.
                    let (a, c, d) = (
                        (spec[b - 1] + 1e-12).ln(),
                        (spec[b] + 1e-12).ln(),
                        (spec[b + 1] + 1e-12).ln(),
                    );
                    let denom = a - 2.0 * c + d;
                    let off = if denom.abs() > 1e-9 {
                        0.5 * (a - d) / denom
                    } else {
                        0.0
                    };
                    let fp = Stft::bin_hz(b)
                        + off.clamp(-0.5, 0.5) * (SR as f32 / crate::dsp::N_FFT as f32);
                    let m = 69.0 + 12.0 * (fp / 440.0).log2();
                    let cents = (m - m.round()) * 100.0;
                    let idx = (((cents + 50.0) / 5.0).floor() as isize).clamp(0, 19) as usize;
                    hist[idx] += (1.0 + spec[b] / max).ln();
                }
            }
        }
        t += 4;
    }
    // Circular smoothing over three bins, then the peak.
    let sm: Vec<f32> = (0..20)
        .map(|i| hist[(i + 19) % 20] + hist[i] + hist[(i + 1) % 20])
        .collect();
    let best = sm
        .iter()
        .enumerate()
        .fold((0, 0.0f32), |m, (i, &v)| if v > m.1 { (i, v) } else { m })
        .0;
    let cents = best as f32 * 5.0 - 50.0 + 2.5;
    if cents.abs() < 7.5 {
        0.0
    } else {
        cents
    }
}

pub fn extract(x: &[f32]) -> Features {
    let n = Stft::n_frames(x.len());
    let mut stft = Stft::new();
    let mels = mel_filters();
    let tuning_cents = estimate_tuning(&mut stft, x, n);
    let mut spec = vec![0.0f32; Stft::BINS];
    let mut pitch = vec![0.0f32; n * N_PITCH];
    let mut mel = vec![0.0f32; n * N_MELS];
    let mut global_max = f32::MIN;
    for t in 0..n {
        stft.power(x, t, &mut spec);
        // Only spectral peaks feed the pitch grid. Below middle C the window's
        // main lobe is wider than a semitone, so summing every bin would smear
        // each bass note into its neighbours; a peak has one frequency.
        for b in 2..Stft::BINS - 2 {
            if !(spec[b] > spec[b - 1] && spec[b] >= spec[b + 1]) {
                continue;
            }
            let f = Stft::bin_hz(b);
            if !(55.0..=2200.0).contains(&f) {
                continue;
            }
            let (a, c, d) = (
                (spec[b - 1] + 1e-12).ln(),
                (spec[b] + 1e-12).ln(),
                (spec[b + 1] + 1e-12).ln(),
            );
            let denom = a - 2.0 * c + d;
            let off = if denom.abs() > 1e-9 {
                0.5 * (a - d) / denom
            } else {
                0.0
            };
            let fp = f + off.clamp(-0.5, 0.5) * (SR as f32 / crate::dsp::N_FFT as f32);
            let m = 69.0 + 12.0 * (fp / 440.0).log2() - tuning_cents / 100.0;
            let idx = m.round() as i32 - PITCH_MIDI0;
            if idx >= 0 && (idx as usize) < N_PITCH {
                pitch[t * N_PITCH + idx as usize] += spec[b - 1] + spec[b] + spec[b + 1];
            }
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
        tuning_cents,
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
