//! Resampling and the short-time Fourier transform.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

/// Every analysis runs at this rate.
pub const SR: u32 = 22050;
/// FFT size (about 186 ms at 22.05 kHz), fine enough to separate low notes.
pub const N_FFT: usize = 4096;
/// Hop between frames (about 23 ms).
pub const HOP: usize = 512;

/// Frames per second of the feature grid.
pub fn fps() -> f32 {
    SR as f32 / HOP as f32
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

fn blackman(u: f64) -> f64 {
    if u.abs() >= 1.0 {
        return 0.0;
    }
    let p = std::f64::consts::PI * u;
    0.42 + 0.5 * p.cos() + 0.08 * (2.0 * p).cos()
}

/// Windowed-sinc resampling of a mono signal.
pub fn resample(x: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || x.is_empty() {
        return x.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let fc = if ratio > 1.0 { 0.45 / ratio } else { 0.45 };
    const HALF: i64 = 16;
    let n_out = ((x.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(n_out);
    for i in 0..n_out {
        let pos = i as f64 * ratio;
        let i0 = pos.floor() as i64;
        let frac = pos - i0 as f64;
        let mut acc = 0.0f64;
        let mut wsum = 0.0f64;
        for k in (-HALF + 1)..=HALF {
            let idx = i0 + k;
            if idx < 0 || idx >= x.len() as i64 {
                continue;
            }
            let t = k as f64 - frac;
            let w = 2.0 * fc * sinc(2.0 * fc * t) * blackman(t / HALF as f64);
            acc += w * x[idx as usize] as f64;
            wsum += w;
        }
        out.push(if wsum.abs() > 1e-9 {
            (acc / wsum) as f32
        } else {
            0.0
        });
    }
    out
}

/// Scale so the loudest sample is 1.0 (no-op for silence).
pub fn normalize_peak(x: &mut [f32]) {
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak > 1e-9 {
        let g = 1.0 / peak;
        for v in x.iter_mut() {
            *v *= g;
        }
    }
}

/// Hann-windowed power spectra, frames centred on `t * HOP`.
pub struct Stft {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
}

impl Default for Stft {
    fn default() -> Self {
        Self::new()
    }
}

impl Stft {
    pub fn new() -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(N_FFT);
        let window = (0..N_FFT)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / N_FFT as f32).cos())
            .collect();
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        Self {
            fft,
            window,
            buf: vec![Complex::new(0.0, 0.0); N_FFT],
            scratch,
        }
    }

    /// Number of frames for a signal of `len` samples.
    pub fn n_frames(len: usize) -> usize {
        1 + len / HOP
    }

    /// Number of frequency bins per frame.
    pub const BINS: usize = N_FFT / 2 + 1;

    /// Centre frequency of bin `b` in Hz.
    pub fn bin_hz(b: usize) -> f32 {
        b as f32 * SR as f32 / N_FFT as f32
    }

    /// Power spectrum of frame `t` into `out` (length `BINS`).
    pub fn power(&mut self, x: &[f32], t: usize, out: &mut [f32]) {
        let start = t as i64 * HOP as i64 - (N_FFT / 2) as i64;
        for (i, c) in self.buf.iter_mut().enumerate() {
            let idx = start + i as i64;
            let s = if idx >= 0 && (idx as usize) < x.len() {
                x[idx as usize]
            } else {
                0.0
            };
            *c = Complex::new(s * self.window[i], 0.0);
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);
        for (b, o) in out.iter_mut().enumerate().take(Self::BINS) {
            *o = self.buf[b].norm_sqr();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_keeps_a_sine() {
        let from = 44100u32;
        let x: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / from as f32).sin())
            .collect();
        let y = resample(&x, from, SR);
        assert!((y.len() as i64 - 22050).abs() <= 1);
        let t = 5000usize;
        let expect = (2.0 * std::f32::consts::PI * 440.0 * t as f32 / SR as f32).sin();
        assert!((y[t] - expect).abs() < 0.03, "{} vs {}", y[t], expect);
    }

    #[test]
    fn stft_peak_is_at_the_tone() {
        let x: Vec<f32> = (0..SR as usize)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / SR as f32).sin())
            .collect();
        let mut s = Stft::new();
        let mut p = vec![0.0; Stft::BINS];
        s.power(&x, 20, &mut p);
        let (best, _) = p
            .iter()
            .enumerate()
            .fold((0, 0.0), |m, (i, &v)| if v > m.1 { (i, v) } else { m });
        assert!((Stft::bin_hz(best) - 1000.0).abs() < 6.0);
    }
}
