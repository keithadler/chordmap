//! Spectrogram layout for MDX-Net separation models (KUIELab, MIT): the
//! model sees `[4, dim_f, dim_t]` = L real, L imaginary, R real, R imaginary
//! over the lowest `dim_f` bins of a `torch.stft` with `center=True`.
//! Inference itself runs in the browser through onnxruntime-web; this
//! module does the transforms on both sides of it.

use crate::spectral::Spectral;
use rustfft::num_complex::Complex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MdxSpec {
    pub n_fft: usize,
    pub hop: usize,
    pub dim_f: usize,
    pub dim_t: usize,
}

impl MdxSpec {
    /// KUIELab MDX-Net "B" models.
    pub const KUIELAB_B: MdxSpec = MdxSpec {
        n_fft: 6144,
        hop: 1024,
        dim_f: 2048,
        dim_t: 256,
    };

    /// Samples per model chunk.
    pub fn chunk_size(&self) -> usize {
        self.hop * (self.dim_t - 1)
    }
    /// Samples discarded at each end of a chunk (edge effects).
    pub fn trim(&self) -> usize {
        self.n_fft / 2
    }
    /// New samples each chunk contributes.
    pub fn gen_size(&self) -> usize {
        self.chunk_size() - 2 * self.trim()
    }

    /// `[4, dim_f, dim_t]` flat, from one stereo chunk of `chunk_size` samples.
    pub fn stft(&self, l: &[f32], r: &[f32]) -> Vec<f32> {
        let s = Spectral::new(self.n_fft, self.hop);
        let bins = s.bins();
        let mut out = vec![0.0f32; 4 * self.dim_f * self.dim_t];
        for (ch, x) in [l, r].iter().enumerate() {
            let spec = s.forward(x);
            let frames = spec.len() / bins;
            for t in 0..self.dim_t.min(frames) {
                for f in 0..self.dim_f {
                    let c = spec[t * bins + f];
                    out[((ch * 2) * self.dim_f + f) * self.dim_t + t] = c.re;
                    out[((ch * 2 + 1) * self.dim_f + f) * self.dim_t + t] = c.im;
                }
            }
        }
        out
    }

    /// Stereo chunk back from a `[4, dim_f, dim_t]` spectrogram; bins above
    /// `dim_f` are zero, as in the reference implementation.
    pub fn istft(&self, spec: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let s = Spectral::new(self.n_fft, self.hop);
        let bins = s.bins();
        let len = self.chunk_size();
        let mut chans = Vec::with_capacity(2);
        for ch in 0..2 {
            let mut full = vec![Complex::new(0.0f32, 0.0); self.dim_t * bins];
            for t in 0..self.dim_t {
                for f in 0..self.dim_f {
                    let re = spec[((ch * 2) * self.dim_f + f) * self.dim_t + t];
                    let im = spec[((ch * 2 + 1) * self.dim_f + f) * self.dim_t + t];
                    full[t * bins + f] = Complex::new(re, im);
                }
            }
            chans.push(s.inverse(&full, len));
        }
        let r = chans.pop().unwrap();
        (chans.pop().unwrap(), r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_round_trip_keeps_the_low_band() {
        let m = MdxSpec::KUIELAB_B;
        assert_eq!(m.chunk_size(), 261120);
        let n = m.chunk_size();
        let l: Vec<f32> = (0..n).map(|i| (i as f32 * 0.02).sin() * 0.5).collect();
        let r: Vec<f32> = (0..n).map(|i| (i as f32 * 0.013).cos() * 0.3).collect();
        let spec = m.stft(&l, &r);
        assert_eq!(spec.len(), 4 * 2048 * 256);
        let (l2, r2) = m.istft(&spec);
        let (a, b) = (m.trim(), n - m.trim());
        let err = (a..b)
            .map(|i| (l[i] - l2[i]).abs().max((r[i] - r2[i]).abs()))
            .fold(0.0f32, f32::max);
        assert!(err < 2e-3, "max error {err}");
    }
}
