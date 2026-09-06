//! Pitch shift by a phase vocoder time-stretch followed by resampling, so
//! a song can play in the capo key or the singer's key at the same tempo.

use crate::dsp;
use crate::spectral::Spectral;
use rustfft::num_complex::Complex;

/// Stretch `x` in time by `factor` (2.0 = twice as long) at constant pitch.
pub fn time_stretch(x: &[f32], factor: f32) -> Vec<f32> {
    if x.is_empty() || (factor - 1.0).abs() < 1e-4 {
        return x.to_vec();
    }
    let s = Spectral::new(2048, 512);
    let spec = s.forward(x);
    let bins = s.bins();
    let frames = spec.len() / bins;
    let hs = s.hop as f32;
    let out_frames = ((frames as f32 - 1.0) * factor).floor() as usize + 1;
    let two_pi = 2.0 * std::f32::consts::PI;
    let omega: Vec<f32> = (0..bins)
        .map(|b| two_pi * b as f32 * hs / s.n_fft as f32)
        .collect();
    let mut out = vec![Complex::new(0.0f32, 0.0); out_frames * bins];
    let mut phase = vec![0.0f32; bins];
    let mut prev_pos = 0usize;
    for k in 0..out_frames {
        let pos = ((k as f32 / factor).round() as usize).min(frames - 1);
        let cur = &spec[pos * bins..(pos + 1) * bins];
        if k == 0 {
            for b in 0..bins {
                phase[b] = cur[b].arg();
                out[b] = cur[b];
            }
        } else {
            let prev = &spec[prev_pos * bins..(prev_pos + 1) * bins];
            let dist = (pos.max(prev_pos) - pos.min(prev_pos)).max(1) as f32;
            for b in 0..bins {
                let expected = omega[b] * dist;
                let mut delta = cur[b].arg() - prev[b].arg() - expected;
                delta -= two_pi * (delta / two_pi).round();
                let true_omega = omega[b] + delta / dist;
                phase[b] += true_omega;
                out[k * bins + b] = Complex::from_polar(cur[b].norm(), phase[b]);
            }
        }
        prev_pos = pos;
    }
    let len = ((x.len() as f32) * factor) as usize;
    s.inverse(&out, len)
}

/// Shift by `semitones` keeping the duration.
pub fn pitch_shift(x: &[f32], sr: u32, semitones: f32) -> Vec<f32> {
    if semitones.abs() < 0.01 {
        return x.to_vec();
    }
    let ratio = 2f32.powf(semitones / 12.0);
    let stretched = time_stretch(x, ratio);
    let mut y = dsp::resample(&stretched, (sr as f32 * ratio).round() as u32, sr);
    y.resize(x.len(), 0.0);
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak_hz(x: &[f32], sr: u32) -> f32 {
        let s = Spectral::new(8192, 8192);
        let mid = &x[x.len() / 2 - 4096..x.len() / 2 + 4096];
        let spec = s.forward(mid);
        let bins = s.bins();
        let frame = &spec[bins..2 * bins];
        let b = (1..bins)
            .max_by(|&a, &c| frame[a].norm().partial_cmp(&frame[c].norm()).unwrap())
            .unwrap();
        b as f32 * sr as f32 / 8192.0
    }

    #[test]
    fn octave_up_doubles_the_frequency_and_keeps_the_length() {
        let sr = 22050u32;
        let x: Vec<f32> = (0..sr as usize * 3)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let y = pitch_shift(&x, sr, 12.0);
        assert_eq!(y.len(), x.len());
        let f = peak_hz(&y, sr);
        assert!((f / 880.0 - 1.0).abs() < 0.02, "peak at {f}");
        let d = pitch_shift(&x, sr, -5.0);
        let f2 = peak_hz(&d, sr);
        let want = 440.0 * 2f32.powf(-5.0 / 12.0);
        assert!(
            (f2 / want - 1.0).abs() < 0.02,
            "peak at {f2}, wanted {want}"
        );
    }
}
