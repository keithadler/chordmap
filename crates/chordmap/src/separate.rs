//! Centre-channel split, no model needed: a lead vocal usually sits dead
//! centre while the band is panned. Per STFT bin the mid signal is kept as
//! "vocal" in proportion to how much smaller the side signal is; the low
//! end is left alone so bass and kick survive.

use crate::spectral::Spectral;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterOptions {
    /// Below this the mix is untouched (bass, kick). Default 150 Hz.
    pub low_hz: f32,
    /// Above this the mix is untouched (air, cymbals). Default 9 kHz.
    pub high_hz: f32,
    /// 0 = nothing removed, 1 = full estimate removed.
    pub strength: f32,
}

impl Default for CenterOptions {
    fn default() -> Self {
        CenterOptions {
            low_hz: 150.0,
            high_hz: 9000.0,
            strength: 1.0,
        }
    }
}

pub struct CenterSplit {
    pub inst_l: Vec<f32>,
    pub inst_r: Vec<f32>,
    pub voc_l: Vec<f32>,
    pub voc_r: Vec<f32>,
}

pub fn center_split(l: &[f32], r: &[f32], sr: u32, opts: &CenterOptions) -> CenterSplit {
    let n = l.len().min(r.len());
    let s = Spectral::new(4096, 1024);
    let sl = s.forward(&l[..n]);
    let sr_ = s.forward(&r[..n]);
    let bins = s.bins();
    let frames = sl.len() / bins;
    let hz = |b: usize| b as f32 * sr as f32 / s.n_fft as f32;
    let mut il = sl.clone();
    let mut ir = sr_.clone();
    let mut vl = vec![rustfft::num_complex::Complex::new(0.0f32, 0.0); sl.len()];
    for t in 0..frames {
        for b in 0..bins {
            let f = hz(b);
            if f < opts.low_hz || f > opts.high_hz {
                continue;
            }
            let i = t * bins + b;
            let m = (sl[i] + sr_[i]) * 0.5;
            let side = (sl[i] - sr_[i]) * 0.5;
            let mask = (1.0 - side.norm() / (m.norm() + 1e-9)).clamp(0.0, 1.0) * opts.strength;
            let c = m * mask;
            vl[i] = c;
            il[i] = sl[i] - c;
            ir[i] = sr_[i] - c;
        }
    }
    let voc = s.inverse(&vl, n);
    CenterSplit {
        inst_l: s.inverse(&il, n),
        inst_r: s.inverse(&ir, n),
        voc_l: voc.clone(),
        voc_r: voc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy_at(x: &[f32], sr: u32, hz: f32) -> f32 {
        // Goertzel-style projection over the middle of the signal.
        let n = x.len();
        let (a, b) = (n / 4, 3 * n / 4);
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, &v) in x.iter().enumerate().take(b).skip(a) {
            let ph = 2.0 * std::f32::consts::PI * hz * i as f32 / sr as f32;
            re += v * ph.cos();
            im += v * ph.sin();
        }
        (re * re + im * im).sqrt() / (b - a) as f32
    }

    #[test]
    fn centre_vocal_leaves_and_panned_guitar_stays() {
        let sr = 22050u32;
        let n = sr as usize * 4;
        let mut l = vec![0.0f32; n];
        let mut r = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let voice = 0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let guitar = 0.4 * (2.0 * std::f32::consts::PI * 220.0 * t).sin();
            let bass = 0.4 * (2.0 * std::f32::consts::PI * 55.0 * t).sin();
            l[i] = voice + guitar + bass;
            r[i] = voice + bass;
        }
        let out = center_split(&l, &r, sr, &CenterOptions::default());
        let ratio = |f: f32| energy_at(&out.inst_l, sr, f) / energy_at(&l, sr, f);
        assert!(
            ratio(440.0) < 0.15,
            "voice left in instrumental: {}",
            ratio(440.0)
        );
        assert!(ratio(220.0) > 0.8, "guitar lost: {}", ratio(220.0));
        assert!(ratio(55.0) > 0.9, "bass lost: {}", ratio(55.0));
        assert!(
            energy_at(&out.voc_l, sr, 440.0) / energy_at(&l, sr, 440.0) > 0.8,
            "voice missing from vocal stem"
        );
    }
}
