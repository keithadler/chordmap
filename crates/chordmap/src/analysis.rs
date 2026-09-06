//! The whole pipeline, from samples to a report.

use crate::chords::{self, Chord, Quality};
use crate::chroma;
use crate::dsp::{self, SR};
use crate::features::{self, N_MELS};
use crate::guitar::{self, Guitar};
use crate::key;
use crate::notes;
use crate::sections;
use crate::tempo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    /// Prefer the tempo candidate nearest this BPM (from a tap or a
    /// half/double button).
    pub bpm_hint: Option<f32>,
    /// Beats per bar. Leave unset to choose between 4 and 3 automatically.
    pub beats_per_bar: Option<usize>,
    /// "band" (default), "hiphop" or "dance": moves the tempo prior and the
    /// section vocabulary.
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tempo {
    pub bpm: f32,
    /// 0..1, how much the winner beats the runner-up.
    pub confidence: f32,
    /// Other plausible tempos, best first.
    pub alternatives: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    /// "G major", "E minor".
    pub name: String,
    pub tonic: String,
    pub minor: bool,
    /// Gap in correlation to the runner-up; small means unsure.
    pub confidence: f32,
    pub alternative: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChordSpan {
    pub start: f32,
    pub end: f32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bar {
    pub start: f32,
    pub end: f32,
    /// One label per beat.
    pub beats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub start: f32,
    pub end: f32,
    /// "A", "B", ...; equal letters are repeats of the same material.
    pub label: String,
    /// "verse", "chorus", "intro", "outro", "bridge", "section".
    pub guess: String,
    /// Index of the first bar in this section.
    pub bar: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub version: String,
    pub duration: f32,
    /// Estimated tuning offset from A440 in cents, already corrected for.
    pub tuning_cents: f32,
    /// "4/4" or "3/4" (or "n/4" when `beatsPerBar` was given).
    pub meter: String,
    pub beats_per_bar: usize,
    /// "band", "hiphop" or "dance", as analysed.
    pub genre: String,
    /// 0..1: how much of the song has clear chord tones. Low means a beat
    /// with sparse harmony; trust the key and the tempo more than the chords.
    pub harmonicity: f32,
    pub tempo: Tempo,
    pub key: Key,
    pub beats: Vec<f32>,
    pub downbeats: Vec<f32>,
    pub chords: Vec<ChordSpan>,
    pub bars: Vec<Bar>,
    pub sections: Vec<Section>,
    pub guitar: Guitar,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
    TooShort,
    BadSampleRate,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Empty => write!(f, "no audio"),
            Error::TooShort => write!(f, "audio shorter than four seconds"),
            Error::BadSampleRate => write!(f, "sample rate must be between 8000 and 384000"),
        }
    }
}
impl std::error::Error for Error {}

/// Analyse mono samples at `sample_rate`.
pub fn analyze(samples: &[f32], sample_rate: u32, opts: &Options) -> Result<Analysis, Error> {
    if samples.is_empty() {
        return Err(Error::Empty);
    }
    if !(8000..=384000).contains(&sample_rate) {
        return Err(Error::BadSampleRate);
    }
    if (samples.len() as f32) < 4.0 * sample_rate as f32 {
        return Err(Error::TooShort);
    }
    let mut x = dsp::resample(samples, sample_rate, SR);
    dsp::normalize_peak(&mut x);
    let duration = x.len() as f32 / SR as f32;
    let feat = features::extract(&x);
    let tuning_cents = feat.tuning_cents;
    let fps = feat.fps;
    let n = feat.n;
    let mut warnings = Vec::new();

    // Tempo and beats.
    let genre = match opts.genre.as_deref() {
        Some("hiphop") | Some("hip hop") | Some("hip-hop") => "hiphop",
        Some("dance") | Some("edm") => "dance",
        _ => "band",
    };
    let centre = match genre {
        "hiphop" => 90.0,
        "dance" => 128.0,
        _ => 120.0,
    };
    let cands = tempo::candidates(&feat.onset, fps, centre, opts.bpm_hint);
    let bpm = cands[0].bpm;
    let confidence = if cands.len() > 1 && cands[0].score > 0.0 {
        ((cands[0].score - cands[1].score) / cands[0].score).clamp(0.0, 1.0)
    } else {
        1.0
    };
    if cands.len() > 1 && cands[1].score > 0.8 * cands[0].score {
        warnings.push("Tempo is close between two candidates; check the alternatives.".into());
    }
    let beat_frames = tempo::track_beats(&feat.onset, fps, bpm);
    if beat_frames.len() < 4 {
        warnings.push("Too few beats found; chords are per second instead.".into());
    }
    let beat_frames = if beat_frames.len() >= 4 {
        beat_frames
    } else {
        (0..n).step_by(fps as usize).collect()
    };
    // Spectral flux peaks about a quarter window before the frame centre
    // reaches an onset, so beat times are pushed forward by that much.
    let onset_lag = dsp::N_FFT as f32 / 4.0 / SR as f32;
    let beat_times: Vec<f32> = beat_frames
        .iter()
        .map(|&f| (f as f32 / fps + onset_lag).min(duration))
        .collect();

    // Harmony per beat.
    let harm = chroma::harmonic(&feat.pitch, n, 9);
    let (chroma_frames, energy_frames) = chroma::chroma(&harm, n);
    let mut beat_chroma = features::segment_mean(&chroma_frames, 12, n, &beat_frames);
    chroma::normalize_rows(&mut beat_chroma, 12);
    let beat_energy = features::segment_mean(&energy_frames, 1, n, &beat_frames);
    let states = chords::decode(&beat_chroma, &beat_energy);
    let nb = beat_frames.len();
    let harmonicity = chords::harmonicity(&beat_chroma, &beat_energy);
    if harmonicity < 0.45 {
        warnings.push("Sparse harmony: this sounds like a beat with few chord tones, so the chords are implied at best. Trust the tempo and key.".into());
    }

    // Key from duration-weighted chroma.
    let mut global = [0.0f32; 12];
    for b in 0..nb {
        let len = if b + 1 < nb {
            beat_frames[b + 1] - beat_frames[b]
        } else {
            n - beat_frames[b]
        } as f32;
        for k in 0..12 {
            global[k] += beat_chroma[b * 12 + k] * len;
        }
    }
    let keys = key::rank(&global);
    let (tonic, minor) = (keys[0].tonic, keys[0].minor);
    let key_name = |k: &key::KeyCandidate| {
        format!(
            "{} {}",
            notes::name(k.tonic, notes::key_uses_flats(k.tonic, k.minor)),
            if k.minor { "minor" } else { "major" }
        )
    };
    let key_conf = (keys[0].correlation - keys[1].correlation).max(0.0);
    if key_conf < 0.05 {
        warnings.push(format!(
            "Key is a close call between {} and {}.",
            key_name(&keys[0]),
            key_name(&keys[1])
        ));
    }
    let key = Key {
        name: key_name(&keys[0]),
        tonic: notes::name(tonic, notes::key_uses_flats(tonic, minor)).to_string(),
        minor: keys[0].minor,
        confidence: key_conf,
        alternative: key_name(&keys[1]),
    };

    // Downbeat phase per meter: chord changes and low-end energy prefer beat
    // one. The meter whose best phase stands out most from its other phases
    // wins, with 4/4 favoured because it is by far the most common.
    let bass_beats = features::segment_mean(&feat.bass, 1, n, &beat_frames);
    let bass_min = bass_beats.iter().cloned().fold(f32::MAX, f32::min);
    let bass_max = bass_beats.iter().cloned().fold(f32::MIN, f32::max);
    let phase_scores = |bpb: usize| -> Vec<f32> {
        (0..bpb.min(nb))
            .map(|p| {
                let mut s = 0.0;
                let mut i = p;
                while i < nb {
                    if i > 0 && states[i] % 12 != states[i - 1] % 12 {
                        s += 1.0;
                    }
                    if bass_max > bass_min {
                        s += 0.5 * (bass_beats[i] - bass_min) / (bass_max - bass_min);
                    }
                    i += bpb;
                }
                s
            })
            .collect()
    };
    let peakiness = |scores: &[f32]| {
        let best = scores.iter().cloned().fold(f32::MIN, f32::max);
        let mean = scores.iter().sum::<f32>() / scores.len().max(1) as f32;
        if mean > 0.0 {
            (best - mean) / mean
        } else {
            0.0
        }
    };
    let (bpb, meter_auto) = match opts.beats_per_bar {
        Some(b) => (b.clamp(2, 12), false),
        None => {
            let p4 = peakiness(&phase_scores(4));
            let p3 = peakiness(&phase_scores(3));
            if p3 > 1.4 * p4 && p3 > 0.15 {
                (3, true)
            } else {
                (4, true)
            }
        }
    };
    let scores = phase_scores(bpb);
    let best_phase = scores
        .iter()
        .enumerate()
        .fold((0, f32::MIN), |m, (i, &v)| if v > m.1 { (i, v) } else { m })
        .0;
    let meter = format!("{bpb}/4");
    let _ = meter_auto;
    let label = |s: usize| {
        let c = Chord::from_state(s);
        notes::chord_label(c.root, c.quality, tonic, minor)
    };
    let beat_end = |b: usize| {
        if b + 1 < nb {
            beat_times[b + 1]
        } else {
            duration
        }
    };

    // Chord spans (merged runs) and bars.
    let mut chord_spans: Vec<ChordSpan> = Vec::new();
    for b in 0..nb {
        let l = label(states[b]);
        match chord_spans.last_mut() {
            Some(last) if last.label == l => last.end = beat_end(b),
            _ => chord_spans.push(ChordSpan {
                start: beat_times[b],
                end: beat_end(b),
                label: l,
            }),
        }
    }
    let mut bars: Vec<Bar> = Vec::new();
    let mut downbeats = Vec::new();
    let mut b = 0usize;
    while b < nb {
        let bar_end = if b < best_phase {
            best_phase
        } else {
            (b + bpb).min(nb)
        };
        if b >= best_phase {
            downbeats.push(beat_times[b]);
        }
        bars.push(Bar {
            start: beat_times[b],
            end: beat_end(bar_end - 1),
            beats: (b..bar_end).map(|i| label(states[i])).collect(),
        });
        b = bar_end;
    }

    // Sections over bars: chroma plus timbre per bar.
    let full_bars: Vec<usize> = (0..bars.len())
        .filter(|&i| bars[i].beats.len() == bpb)
        .collect();
    let bar_first_beat: Vec<usize> = full_bars
        .iter()
        .map(|&i| best_phase + (i - if best_phase > 0 { 1 } else { 0 }) * bpb)
        .collect();
    let bar_frames: Vec<usize> = bar_first_beat.iter().map(|&bt| beat_frames[bt]).collect();
    let mut bar_chroma = features::segment_mean(&chroma_frames, 12, n, &bar_frames);
    chroma::normalize_rows(&mut bar_chroma, 12);
    let bar_mel = features::segment_mean(&feat.mel, N_MELS, n, &bar_frames);
    let bar_energy: Vec<f32> = (0..bar_frames.len())
        .map(|i| (0..N_MELS).map(|m| bar_mel[i * N_MELS + m]).sum::<f32>() / N_MELS as f32)
        .collect();
    // Standardise timbre across bars so loudness does not dominate.
    let mut timbre = vec![0.0f32; bar_frames.len() * N_MELS];
    for m in 0..N_MELS {
        let col: Vec<f32> = (0..bar_frames.len())
            .map(|i| bar_mel[i * N_MELS + m])
            .collect();
        let mean = col.iter().sum::<f32>() / col.len().max(1) as f32;
        let sd = (col.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / col.len().max(1) as f32)
            .sqrt()
            .max(1e-6);
        for i in 0..bar_frames.len() {
            timbre[i * N_MELS + m] = (col[i] - mean) / sd;
        }
    }
    chroma::normalize_rows(&mut timbre, N_MELS);
    // Harmony (unit norm) plus timbre (unit norm, weighted) per bar.
    let dim = 12 + N_MELS;
    let mut bar_feat = vec![0.0f32; bar_frames.len() * dim];
    for i in 0..bar_frames.len() {
        bar_feat[i * dim..i * dim + 12].copy_from_slice(&bar_chroma[i * 12..i * 12 + 12]);
        for m in 0..N_MELS {
            bar_feat[i * dim + 12 + m] = 0.7 * timbre[i * N_MELS + m];
        }
    }
    let structure = sections::segment(&bar_feat, dim, bar_frames.len());
    let mut segs = structure.segments;
    if segs.is_empty() {
        segs.push(sections::Segment {
            start: 0,
            end: bar_frames.len(),
            group: 0,
        });
    }
    let seg_energy: Vec<f32> = segs
        .iter()
        .map(|s| {
            (s.start..s.end).map(|i| bar_energy[i]).sum::<f32>() / (s.end - s.start).max(1) as f32
        })
        .collect();
    let mut guesses = sections::guesses(&segs, &seg_energy);
    if genre == "hiphop" {
        for g in guesses.iter_mut() {
            if *g == "chorus" {
                *g = "hook";
            }
        }
    }
    if genre == "dance" {
        for g in guesses.iter_mut() {
            if *g == "chorus" {
                *g = "drop";
            } else if *g == "verse" {
                *g = "break";
            }
        }
    }
    let letters = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"];
    let sections_out: Vec<Section> = segs
        .iter()
        .zip(guesses)
        .map(|(s, g)| {
            let first_bar = full_bars[s.start];
            let last_bar = full_bars[s.end - 1];
            Section {
                start: bars[first_bar].start,
                end: bars[last_bar].end,
                label: letters[s.group.min(letters.len() - 1)].to_string(),
                guess: g.to_string(),
                bar: first_bar,
            }
        })
        .collect();

    // Capo: shapes are spelled in the key the capo makes.
    let chord_list: Vec<(Option<usize>, Quality, f32)> = (0..nb)
        .map(|b| {
            let c = Chord::from_state(states[b]);
            (c.root, c.quality, beat_end(b) - beat_times[b])
        })
        .collect();
    let guitar = guitar::recommend(
        &chord_list,
        |r, q| notes::chord_label(Some(r), q, tonic, minor),
        |r, q| notes::chord_label(Some(r), q, tonic, minor),
    );
    let capo = guitar.capo;
    let shape_tonic = (tonic + 12 - capo) % 12;
    let guitar = Guitar {
        shapes: guitar
            .shapes
            .keys()
            .map(|k| {
                let (root, quality) = notes::parse_label(k).unwrap_or((0, Quality::Major));
                (
                    k.clone(),
                    notes::chord_label(Some((root + 12 - capo) % 12), quality, shape_tonic, minor),
                )
            })
            .collect(),
        ..guitar
    };

    let alternatives: Vec<f32> = cands
        .iter()
        .skip(1)
        .take(3)
        .map(|c| (c.bpm * 10.0).round() / 10.0)
        .collect();
    if std::env::var_os("CHORDMAP_DEBUG").is_some() {
        for b in (0..nb.min(16)).step_by(2) {
            let row: Vec<String> = (0..12)
                .map(|k| format!("{:.2}", beat_chroma[b * 12 + k]))
                .collect();
            eprintln!(
                "beat {b} {} chroma C..B {}",
                label(states[b]),
                row.join(" ")
            );
        }
        eprintln!(
            "novelty per bar: {:?}",
            structure
                .novelty
                .iter()
                .map(|v| (v * 100.0).round() / 100.0)
                .collect::<Vec<_>>()
        );
        eprintln!("segments: {:?}", segs);
        eprintln!(
            "segment similarity: {:?}",
            structure
                .pair_similarity
                .iter()
                .map(|&(a, b, s)| (a, b, (s * 100.0).round() / 100.0))
                .collect::<Vec<_>>()
        );
    }
    Ok(Analysis {
        version: env!("CARGO_PKG_VERSION").to_string(),
        duration,
        tuning_cents,
        meter,
        beats_per_bar: bpb,
        genre: genre.to_string(),
        harmonicity,
        tempo: Tempo {
            bpm: (bpm * 10.0).round() / 10.0,
            confidence,
            alternatives,
        },
        key,
        beats: beat_times,
        downbeats,
        chords: chord_spans,
        bars,
        sections: sections_out,
        guitar,
        warnings,
    })
}

/// Plain-text chord sheet: sections as headings, four bars per line.
pub fn chord_sheet(a: &Analysis) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} BPM, {}, {}\n",
        a.tempo.bpm, a.meter, a.key.name
    ));
    if a.guitar.capo > 0 {
        let shapes: Vec<String> = a
            .guitar
            .shapes
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        out.push_str(&format!(
            "Guitar: capo {} and play {}\n",
            a.guitar.capo,
            shapes.join(" ")
        ));
    } else if !a.guitar.shapes.is_empty() {
        out.push_str("Guitar: no capo needed\n");
    }
    for (si, s) in a.sections.iter().enumerate() {
        let end_bar = a.sections.get(si + 1).map_or(a.bars.len(), |n| n.bar);
        out.push_str(&format!("\n[{} {}]\n", s.label, s.guess));
        let mut line = String::new();
        for (i, bar) in a.bars[s.bar..end_bar].iter().enumerate() {
            let mut cell = String::new();
            let mut prev = "";
            for l in &bar.beats {
                if l != prev {
                    if !cell.is_empty() {
                        cell.push(' ');
                    }
                    cell.push_str(l);
                    prev = l;
                }
            }
            line.push_str(&format!("| {:<7}", cell));
            if (i + 1) % 4 == 0 {
                line.push_str("|\n");
                out.push_str(&line);
                line.clear();
            }
        }
        if !line.is_empty() {
            line.push_str("|\n");
            out.push_str(&line);
        }
    }
    out
}
