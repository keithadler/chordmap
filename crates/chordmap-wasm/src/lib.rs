//! WebAssembly bindings for [chordmap](https://github.com/keithadler/chordmap).
//!
//! `analyze(samples, sampleRate, options)` takes a mono `Float32Array` and
//! returns the analysis as a JSON string (see the TypeScript types).

use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface Options {
  /** Prefer the tempo candidate nearest this BPM. */
  bpmHint?: number;
  /** Beats per bar; unset picks 4 or 3. */
  beatsPerBar?: number;
  /** "band" (default), "hiphop" or "dance". */
  genre?: string;
}
export interface Tempo { bpm: number; confidence: number; alternatives: number[] }
export interface Key { name: string; tonic: string; minor: boolean; confidence: number; alternative: string }
export interface ChordSpan { start: number; end: number; label: string }
export interface Bar { start: number; end: number; beats: string[] }
export interface Section { start: number; end: number; label: string; guess: string; bar: number }
export interface Analysis {
  version: string; duration: number; tuningCents: number; meter: string; beatsPerBar: number;
  genre: string; harmonicity: number; tempo: Tempo; key: Key;
  beats: number[]; downbeats: number[]; chords: ChordSpan[]; bars: Bar[];
  sections: Section[]; warnings: string[];
}
"#;

/// Analyse mono samples. Returns the JSON text of an `Analysis`.
#[wasm_bindgen]
pub fn analyze(
    samples: &[f32],
    sample_rate: u32,
    options: Option<String>,
) -> Result<String, JsError> {
    let opts: chordmap::Options = match options {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(&s).map_err(|e| JsError::new(&format!("bad options: {e}")))?
        }
        _ => chordmap::Options::default(),
    };
    let a =
        chordmap::analyze(samples, sample_rate, &opts).map_err(|e| JsError::new(&e.to_string()))?;
    serde_json::to_string(&a).map_err(|e| JsError::new(&e.to_string()))
}

/// Plain-text chord sheet from an `Analysis` JSON string.
#[wasm_bindgen(js_name = chordSheet)]
pub fn chord_sheet(analysis_json: &str) -> Result<String, JsError> {
    let a: chordmap::Analysis =
        serde_json::from_str(analysis_json).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(chordmap::chord_sheet(&a))
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Centre-channel split. Returns `[instL, instR, vocL, vocR]` concatenated,
/// each `left.length` long. `strength` 0..1.
#[wasm_bindgen(js_name = centerSplit)]
pub fn center_split(left: &[f32], right: &[f32], sample_rate: u32, strength: f32) -> Vec<f32> {
    let opts = chordmap::separate::CenterOptions {
        strength: strength.clamp(0.0, 1.0),
        ..Default::default()
    };
    let out = chordmap::separate::center_split(left, right, sample_rate, &opts);
    let mut v = Vec::with_capacity(out.inst_l.len() * 4);
    v.extend(out.inst_l);
    v.extend(out.inst_r);
    v.extend(out.voc_l);
    v.extend(out.voc_r);
    v
}

/// Pitch shift one channel by `semitones` at the same length.
#[wasm_bindgen(js_name = pitchShift)]
pub fn pitch_shift(samples: &[f32], sample_rate: u32, semitones: f32) -> Vec<f32> {
    chordmap::pitch::pitch_shift(samples, sample_rate, semitones)
}

/// MDX-Net model input for one stereo chunk: `[4, dimF, dimT]` flat.
#[wasm_bindgen(js_name = mdxStft)]
pub fn mdx_stft(
    n_fft: usize,
    hop: usize,
    dim_f: usize,
    dim_t: usize,
    left: &[f32],
    right: &[f32],
) -> Vec<f32> {
    chordmap::mdx::MdxSpec {
        n_fft,
        hop,
        dim_f,
        dim_t,
    }
    .stft(left, right)
}

/// Stereo chunk back from a model output: `[left..., right...]`.
#[wasm_bindgen(js_name = mdxIstft)]
pub fn mdx_istft(n_fft: usize, hop: usize, dim_f: usize, dim_t: usize, spec: &[f32]) -> Vec<f32> {
    let (l, r) = chordmap::mdx::MdxSpec {
        n_fft,
        hop,
        dim_f,
        dim_t,
    }
    .istft(spec);
    let mut v = l;
    v.extend(r);
    v
}
