//! C face over chordmap for the Mac app. Every returned string is owned by the caller and
//! must go back through `chordmap_free`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn owned(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "")).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Analyse `n` mono samples at `sample_rate`. Returns the `Analysis` as JSON, or `{"error": "..."}`.
/// `options_json` may be null or empty; otherwise `{"bpmHint": 120, "beatsPerBar": 4, "genre": "band"}`.
#[no_mangle]
pub extern "C" fn chordmap_analyze(samples: *const f32, n: usize, sample_rate: u32, options_json: *const c_char) -> *mut c_char {
    if samples.is_null() || n == 0 {
        return owned("{\"error\":\"no audio\"}".to_string());
    }
    let x = unsafe { std::slice::from_raw_parts(samples, n) };
    let opts: chordmap::Options = if options_json.is_null() {
        Default::default()
    } else {
        let s = unsafe { CStr::from_ptr(options_json) }.to_string_lossy();
        if s.trim().is_empty() { Default::default() } else {
            match serde_json::from_str(&s) {
                Ok(o) => o,
                Err(e) => return owned(format!("{{\"error\":\"bad options: {}\"}}", e.to_string().replace('"', "'"))),
            }
        }
    };
    match chordmap::analyze(x, sample_rate, &opts) {
        Ok(a) => owned(serde_json::to_string(&a).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))),
        Err(e) => owned(format!("{{\"error\":\"{}\"}}", e.to_string().replace('"', "'"))),
    }
}

/// Plain-text chord sheet from an `Analysis` JSON string.
#[no_mangle]
pub extern "C" fn chordmap_chord_sheet(analysis_json: *const c_char) -> *mut c_char {
    if analysis_json.is_null() { return owned(String::new()); }
    let s = unsafe { CStr::from_ptr(analysis_json) }.to_string_lossy();
    match serde_json::from_str::<chordmap::Analysis>(&s) {
        Ok(a) => owned(chordmap::chord_sheet(&a)),
        Err(e) => owned(format!("error: {}", e)),
    }
}

/// The chordmap crate version this library was built with.
#[no_mangle]
pub extern "C" fn chordmap_version() -> *mut c_char {
    owned(chordmap_crate_version())
}

fn chordmap_crate_version() -> String {
    // Read from the dependency's metadata at build time.
    env!("CHORDMAP_VERSION").to_string()
}

#[no_mangle]
pub extern "C" fn chordmap_free(p: *mut c_char) {
    if !p.is_null() { unsafe { drop(CString::from_raw(p)) } }
}

/// Render a progression such as "C G Am F" (four beats each) at `bpm`, `loops` times, at
/// chordmap's own 22050 Hz. Writes the sample count to `out_len`; free with `chordmap_free_samples`.
#[no_mangle]
pub extern "C" fn chordmap_synth(chords: *const c_char, bpm: f32, loops: usize, out_len: *mut usize) -> *mut f32 {
    if chords.is_null() || out_len.is_null() { return std::ptr::null_mut(); }
    let text = unsafe { CStr::from_ptr(chords) }.to_string_lossy();
    let prog = match chordmap::synth::parse_progression(&text, 4) { Ok(p) => p, Err(_) => { unsafe { *out_len = 0 }; return std::ptr::null_mut(); } };
    let all: Vec<_> = (0..loops.max(1)).flat_map(|_| prog.clone()).collect();
    let mut x = Vec::new();
    chordmap::synth::render_progression(&mut x, 0, &all, if bpm > 0.0 { bpm } else { 100.0 }, Default::default(), 1);
    let mut b = x.into_boxed_slice();
    unsafe { *out_len = b.len() };
    let p = b.as_mut_ptr();
    std::mem::forget(b);
    p
}

/// Frees a buffer from `chordmap_synth`.
#[no_mangle]
pub extern "C" fn chordmap_free_samples(p: *mut f32, n: usize) {
    if !p.is_null() { unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(p, n))) } }
}

/// chordmap's internal sample rate, which `chordmap_synth` renders at.
#[no_mangle]
pub extern "C" fn chordmap_sample_rate() -> u32 { chordmap::dsp::SR }
