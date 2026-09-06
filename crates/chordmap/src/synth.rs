//! Deterministic test signals: click tracks and strummed chord progressions.
//! Used by the test suite and the `chordmap synth` command, so nothing
//! copyrighted is ever needed.

use crate::dsp::SR;

/// One strummed chord in a progression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthChord {
    /// Pitch class 0..12 (C = 0).
    pub root: usize,
    pub minor: bool,
    pub beats: usize,
}

/// Timbre controls for a section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timbre {
    /// Harmonic roll-off exponent; lower is brighter.
    pub rolloff: f32,
    /// Level of the bass note an octave below the root.
    pub bass: f32,
    /// Level of the drum clicks on each beat.
    pub click: f32,
}

impl Default for Timbre {
    fn default() -> Self {
        Timbre {
            rolloff: 1.5,
            bass: 0.6,
            click: 0.5,
        }
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
    }
}

fn midi_hz(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// Render a progression at `bpm` into `out` starting at sample `at`.
/// Returns the number of samples written.
pub fn render_progression(
    out: &mut Vec<f32>,
    at: usize,
    chords: &[SynthChord],
    bpm: f32,
    timbre: Timbre,
    seed: u64,
) -> usize {
    let spb = (60.0 * SR as f32 / bpm) as usize;
    let total: usize = chords.iter().map(|c| c.beats * spb).sum();
    if out.len() < at + total {
        out.resize(at + total, 0.0);
    }
    let mut rng = Lcg(seed);
    let mut pos = at;
    let mut beat_index = 0usize;
    for ch in chords {
        let third = if ch.minor { 3 } else { 4 };
        let notes = [
            60 + ch.root as i32,
            60 + ch.root as i32 + third,
            60 + ch.root as i32 + 7,
            72 + ch.root as i32,
        ];
        for b in 0..ch.beats {
            let start = pos + b * spb;
            let downbeat = beat_index % 4 == 0;
            for i in 0..spb {
                let t = i as f32 / SR as f32;
                let env = (-t * 2.5).exp() * (1.0 - (-t * 200.0).exp());
                let mut s = 0.0f32;
                for (k, &n) in notes.iter().enumerate() {
                    let f = midi_hz(n as f32);
                    let strum = (k as f32 * 0.012 - t).max(0.0);
                    if strum > 0.0 {
                        continue;
                    }
                    for h in 1..=6 {
                        let a = 1.0 / (h as f32).powf(timbre.rolloff);
                        s += a * (2.0 * std::f32::consts::PI * f * h as f32 * t).sin();
                    }
                }
                s *= 0.12 * env;
                let bf = midi_hz((48 + ch.root as i32) as f32);
                s += timbre.bass
                    * 0.25
                    * (-t * 1.5).exp()
                    * (2.0 * std::f32::consts::PI * bf * t).sin();
                if t < 0.02 {
                    let lvl = if downbeat { 1.0 } else { 0.6 };
                    s += timbre.click * lvl * rng.next() * (-t * 300.0).exp();
                }
                out[start + i] += s;
            }
            beat_index += 1;
        }
        pos += ch.beats * spb;
    }
    total
}

/// A bare click track.
pub fn click_track(bpm: f32, seconds: f32) -> Vec<f32> {
    let n = (seconds * SR as f32) as usize;
    let spb = 60.0 * SR as f32 / bpm;
    let mut out = vec![0.0f32; n];
    let mut rng = Lcg(7);
    let mut k = 0.0f32;
    while (k * spb) as usize + 1 < n {
        let start = (k * spb) as usize;
        for i in 0..(0.02 * SR as f32) as usize {
            if start + i < n {
                out[start + i] += rng.next() * (-(i as f32 / SR as f32) * 300.0).exp();
            }
        }
        k += 1.0;
    }
    out
}

/// Parse "C G Am F" style text into chords of `beats` each.
pub fn parse_progression(text: &str, beats: usize) -> Result<Vec<SynthChord>, String> {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let flats = ["Db", "Eb", "Gb", "Ab", "Bb"];
    let flat_pc = [1, 3, 6, 8, 10];
    text.split_whitespace()
        .map(|tok| {
            let (name, minor) = match tok.strip_suffix('m') {
                Some(n) => (n, true),
                None => (tok, false),
            };
            let root = names
                .iter()
                .position(|&x| x == name)
                .or_else(|| flats.iter().position(|&x| x == name).map(|i| flat_pc[i]))
                .ok_or_else(|| format!("unknown chord {tok}"))?;
            Ok(SynthChord { root, minor, beats })
        })
        .collect()
}
