//! chordmap: tempo, key, chords and sections from audio, in pure Rust.
//!
//! The crate does no I/O. Hand [`analyze`] mono samples and a sample rate and
//! it returns an [`Analysis`] that serialises to JSON. Every step is classic
//! music information retrieval, no models to download: onset autocorrelation
//! for tempo, dynamic-programming beat tracking, chroma templates with Viterbi
//! smoothing for chords, Krumhansl profiles for key, and a self-similarity
//! matrix for sections.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod chords;
pub mod chroma;
pub mod dsp;
pub mod features;
pub mod guitar;
pub mod key;
pub mod mdx;
pub mod notes;
pub mod pitch;
pub mod sections;
pub mod separate;
pub mod spectral;
pub mod synth;
pub mod tempo;

pub use analysis::{
    analyze, chord_sheet, Analysis, Bar, ChordSpan, Error, Key, Options, Section, Tempo,
};
pub use guitar::{CapoOption, Guitar};
