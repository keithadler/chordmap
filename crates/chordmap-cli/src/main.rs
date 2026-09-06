//! `chordmap analyze song.mp3 [--json] [--bpm 120] [--beats-per-bar 3]`
//! `chordmap synth out.wav --chords "C G Am F" [--bpm 100] [--loops 8]`

use std::fs::File;
use std::path::Path;
use std::process::ExitCode;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const USAGE: &str = "chordmap: tempo, key, chords and sections from audio.

USAGE
  chordmap analyze <file> [--json] [--bpm <hint>] [--beats-per-bar <n>]
  chordmap synth <out.wav> --chords \"C G Am F\" [--bpm 100] [--loops 8]
  chordmap version | help

Reads MP3, AAC/M4A, FLAC, OGG and WAV. Nothing is sent anywhere.
Exit codes: 0 done, 2 problem, 64 usage.";

fn decode(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("unsupported or damaged file: {e}"))?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("no audio track")?;
    let track_id = track.id;
    let rate = track
        .codec_params
        .sample_rate
        .ok_or("unknown sample rate")?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("no decoder: {e}"))?;
    let mut mono: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(format!("read error: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("decode error: {e}")),
        };
        push_mono(&decoded, &mut mono);
    }
    Ok((mono, rate))
}

fn push_mono(buf: &AudioBufferRef<'_>, out: &mut Vec<f32>) {
    macro_rules! mix {
        ($b:expr, $conv:expr) => {{
            let ch = $b.spec().channels.count();
            let frames = $b.frames();
            for i in 0..frames {
                let mut s = 0.0f32;
                for c in 0..ch {
                    s += $conv($b.chan(c)[i]);
                }
                out.push(s / ch as f32);
            }
        }};
    }
    match buf {
        AudioBufferRef::F32(b) => mix!(b, |v: f32| v),
        AudioBufferRef::F64(b) => mix!(b, |v: f64| v as f32),
        AudioBufferRef::S16(b) => mix!(b, |v: i16| v as f32 / 32768.0),
        AudioBufferRef::S32(b) => mix!(b, |v: i32| v as f32 / 2147483648.0),
        AudioBufferRef::U8(b) => mix!(b, |v: u8| (v as f32 - 128.0) / 128.0),
        AudioBufferRef::S24(b) => mix!(b, |v: symphonia::core::sample::i24| v.inner() as f32
            / 8388608.0),
        AudioBufferRef::U16(b) => mix!(b, |v: u16| (v as f32 - 32768.0) / 32768.0),
        AudioBufferRef::U24(b) => mix!(b, |v: symphonia::core::sample::u24| (v.inner() as f32
            - 8388608.0)
            / 8388608.0),
        AudioBufferRef::U32(b) => mix!(b, |v: u32| (v as f64 / 2147483648.0 - 1.0) as f32),
        AudioBufferRef::S8(b) => mix!(b, |v: i8| v as f32 / 128.0),
    }
}

fn write_wav(path: &Path, samples: &[f32], rate: u32) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = File::create(path)?;
    let data_len = (samples.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    let peak = samples.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-9);
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        bytes.extend_from_slice(&((s / peak * 0.9 * 32767.0) as i16).to_le_bytes());
    }
    f.write_all(&bytes)
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("version") => {
            println!("chordmap {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("analyze") => {
            let Some(path) = args.get(1) else {
                eprintln!("{USAGE}");
                return ExitCode::from(64);
            };
            let opts = chordmap::Options {
                bpm_hint: flag(&args, "--bpm").and_then(|v| v.parse().ok()),
                beats_per_bar: flag(&args, "--beats-per-bar").and_then(|v| v.parse().ok()),
            };
            let (samples, rate) = match decode(Path::new(path)) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("chordmap: {e}");
                    return ExitCode::from(2);
                }
            };
            match chordmap::analyze(&samples, rate, &opts) {
                Ok(a) => {
                    if args.iter().any(|a| a == "--json") {
                        println!("{}", serde_json::to_string_pretty(&a).unwrap());
                    } else {
                        print!("{}", chordmap::chord_sheet(&a));
                        println!("\nsections:");
                        for s in &a.sections {
                            println!("  {:>6.1}s  {} ({})", s.start, s.label, s.guess);
                        }
                        if !a.tempo.alternatives.is_empty() {
                            println!("other tempos: {:?}", a.tempo.alternatives);
                        }
                        for w in &a.warnings {
                            println!("note: {w}");
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("chordmap: {e}");
                    ExitCode::from(2)
                }
            }
        }
        Some("synth") => {
            let Some(out) = args.get(1) else {
                eprintln!("{USAGE}");
                return ExitCode::from(64);
            };
            let chords = flag(&args, "--chords").unwrap_or("C G Am F");
            let bpm: f32 = flag(&args, "--bpm")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0);
            let loops: usize = flag(&args, "--loops")
                .and_then(|v| v.parse().ok())
                .unwrap_or(8);
            let prog = match chordmap::synth::parse_progression(chords, 4) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("chordmap: {e}");
                    return ExitCode::from(64);
                }
            };
            let all: Vec<_> = (0..loops).flat_map(|_| prog.clone()).collect();
            let mut x = Vec::new();
            chordmap::synth::render_progression(&mut x, 0, &all, bpm, Default::default(), 1);
            match write_wav(Path::new(out), &x, chordmap::dsp::SR) {
                Ok(()) => {
                    println!(
                        "wrote {out} ({:.1}s at {bpm} BPM)",
                        x.len() as f32 / chordmap::dsp::SR as f32
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("chordmap: {e}");
                    ExitCode::from(2)
                }
            }
        }
        _ => {
            println!("{USAGE}");
            ExitCode::from(if args.is_empty() || args[0] == "help" {
                0
            } else {
                64
            })
        }
    }
}
