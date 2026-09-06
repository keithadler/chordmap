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
  /** Beats per bar, default 4. */
  beatsPerBar?: number;
}
export interface Tempo { bpm: number; confidence: number; alternatives: number[] }
export interface Key { name: string; tonic: string; minor: boolean; confidence: number; alternative: string }
export interface ChordSpan { start: number; end: number; label: string }
export interface Bar { start: number; end: number; beats: string[] }
export interface Section { start: number; end: number; label: string; guess: string; bar: number }
export interface Analysis {
  version: string; duration: number; tempo: Tempo; key: Key;
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
