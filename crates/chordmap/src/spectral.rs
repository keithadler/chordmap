//! General STFT and inverse with the same conventions as `torch.stft`
//! (`center=True`, reflect padding, periodic Hann), used by the separation
//! and pitch-shift code. The analysis STFT in `dsp` is tuned for features;
//! this one is for signals that have to come back out as audio.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub struct Spectral {
    pub n_fft: usize,
    pub hop: usize,
    window: Vec<f32>,
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
}

impl Spectral {
    pub fn new(n_fft: usize, hop: usize) -> Self {
        let mut p = FftPlanner::<f32>::new();
        let window = (0..n_fft)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n_fft as f32).cos())
            .collect();
        Spectral {
            n_fft,
            hop,
            window,
            fwd: p.plan_fft_forward(n_fft),
            inv: p.plan_fft_inverse(n_fft),
        }
    }

    pub fn bins(&self) -> usize {
        self.n_fft / 2 + 1
    }

    /// Frames produced for `len` samples with centre padding.
    pub fn frames(&self, len: usize) -> usize {
        1 + len / self.hop
    }

    fn padded(&self, x: &[f32]) -> Vec<f32> {
        let p = self.n_fft / 2;
        let n = x.len();
        let mut out = Vec::with_capacity(n + 2 * p);
        for i in 0..p {
            let j = p - i;
            out.push(if j < n { x[j] } else { 0.0 });
        }
        out.extend_from_slice(x);
        for i in 0..p {
            let j = n as i64 - 2 - i as i64;
            out.push(if j >= 0 && (j as usize) < n {
                x[j as usize]
            } else {
                0.0
            });
        }
        out
    }

    /// Complex spectrogram, `frames * bins`, frame-major.
    pub fn forward(&self, x: &[f32]) -> Vec<Complex<f32>> {
        let xp = self.padded(x);
        let frames = self.frames(x.len());
        let bins = self.bins();
        let mut out = vec![Complex::new(0.0, 0.0); frames * bins];
        let mut buf = vec![Complex::new(0.0, 0.0); self.n_fft];
        let mut scratch = vec![Complex::new(0.0, 0.0); self.fwd.get_inplace_scratch_len()];
        for t in 0..frames {
            let start = t * self.hop;
            for (i, slot) in buf.iter_mut().enumerate() {
                let s = xp.get(start + i).copied().unwrap_or(0.0);
                *slot = Complex::new(s * self.window[i], 0.0);
            }
            self.fwd.process_with_scratch(&mut buf, &mut scratch);
            out[t * bins..(t + 1) * bins].copy_from_slice(&buf[..bins]);
        }
        out
    }

    /// Overlap-add inverse for `len` output samples.
    pub fn inverse(&self, spec: &[Complex<f32>], len: usize) -> Vec<f32> {
        let bins = self.bins();
        let frames = spec.len() / bins;
        let p = self.n_fft / 2;
        let total = (frames - 1) * self.hop + self.n_fft;
        let mut y = vec![0.0f32; total];
        let mut norm = vec![0.0f32; total];
        let mut buf = vec![Complex::new(0.0, 0.0); self.n_fft];
        let mut scratch = vec![Complex::new(0.0, 0.0); self.inv.get_inplace_scratch_len()];
        let scale = 1.0 / self.n_fft as f32;
        for t in 0..frames {
            let f = &spec[t * bins..(t + 1) * bins];
            buf[..bins].copy_from_slice(&f[..bins]);
            for b in bins..self.n_fft {
                buf[b] = f[self.n_fft - b].conj();
            }
            self.inv.process_with_scratch(&mut buf, &mut scratch);
            let start = t * self.hop;
            for i in 0..self.n_fft {
                y[start + i] += buf[i].re * scale * self.window[i];
                norm[start + i] += self.window[i] * self.window[i];
            }
        }
        (0..len)
            .map(|i| {
                let j = i + p;
                if j < total && norm[j] > 1e-8 {
                    y[j] / norm[j]
                } else {
                    0.0
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_identity() {
        let x: Vec<f32> = (0..20000)
            .map(|i| ((i as f32) * 0.05).sin() * 0.5 + ((i as f32) * 0.31).cos() * 0.2)
            .collect();
        for (n, h) in [(2048, 512), (6144, 1024), (4096, 1024)] {
            let s = Spectral::new(n, h);
            let y = s.inverse(&s.forward(&x), x.len());
            let err = x
                .iter()
                .zip(&y)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(err < 1e-3, "n_fft {n}: max error {err}");
        }
    }
}
