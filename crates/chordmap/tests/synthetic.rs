use chordmap::dsp::SR;
use chordmap::synth::{self, SynthChord, Timbre};
use chordmap::{analyze, Options};

fn octave_close(a: f32, b: f32) -> bool {
    (a / b - 1.0).abs() < 0.02
}

#[test]
fn click_track_tempo() {
    for bpm in [90.0f32, 128.0, 174.0] {
        let x = synth::click_track(bpm, 30.0);
        let a = analyze(&x, SR, &Options::default()).unwrap();
        assert!(
            octave_close(a.tempo.bpm, bpm)
                || octave_close(a.tempo.bpm, bpm / 2.0)
                || octave_close(a.tempo.bpm, bpm * 2.0),
            "{} for {}",
            a.tempo.bpm,
            bpm
        );
        assert!(a.beats.len() > 20);
    }
}

#[test]
fn tempo_hint_picks_the_octave() {
    let x = synth::click_track(128.0, 30.0);
    let a = analyze(
        &x,
        SR,
        &Options {
            bpm_hint: Some(64.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(octave_close(a.tempo.bpm, 64.0), "{}", a.tempo.bpm);
}

#[test]
fn progression_key_and_chords() {
    let prog = synth::parse_progression("C G Am F", 4).unwrap();
    let loops: Vec<SynthChord> = (0..8).flat_map(|_| prog.clone()).collect();
    let mut x = Vec::new();
    synth::render_progression(&mut x, 0, &loops, 100.0, Timbre::default(), 1);
    let a = analyze(&x, SR, &Options::default()).unwrap();
    assert_eq!(a.key.name, "C major", "{:?}", a.key);
    assert!(
        octave_close(a.tempo.bpm, 100.0) || octave_close(a.tempo.bpm, 50.0),
        "{}",
        a.tempo.bpm
    );
    // Compare the chord at each beat time to the truth.
    let spb = 60.0 / 100.0;
    let truth = ["C", "G", "Am", "F"];
    let mut hits = 0;
    let mut total = 0;
    for span in &a.chords {
        let mut t = span.start + spb / 2.0;
        while t < span.end {
            let beat = (t / spb).floor() as usize;
            let want = truth[(beat / 4) % 4];
            total += 1;
            if span.label == want {
                hits += 1;
            }
            t += spb;
        }
    }
    let acc = hits as f32 / total as f32;
    assert!(
        acc >= 0.9,
        "chord accuracy {acc} ({hits}/{total}): {:?}",
        a.chords
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>()
    );
    assert!(a.bars.iter().filter(|b| b.beats.len() == 4).count() >= 28);
}

#[test]
fn sections_come_back_as_aba() {
    let a_prog: Vec<SynthChord> = (0..4)
        .flat_map(|_| synth::parse_progression("C G Am F", 4).unwrap())
        .collect();
    let b_prog: Vec<SynthChord> = (0..2)
        .flat_map(|_| synth::parse_progression("Dm Bb F C", 4).unwrap())
        .collect();
    let bright = Timbre {
        rolloff: 1.2,
        bass: 0.6,
        click: 0.5,
    };
    let dark = Timbre {
        rolloff: 2.5,
        bass: 0.9,
        click: 0.3,
    };
    let mut x = Vec::new();
    let mut at = 0;
    at += synth::render_progression(&mut x, at, &a_prog, 110.0, bright, 1);
    at += synth::render_progression(&mut x, at, &b_prog, 110.0, dark, 2);
    synth::render_progression(&mut x, at, &a_prog, 110.0, bright, 3);
    let a = analyze(&x, SR, &Options::default()).unwrap();
    let labels: Vec<&str> = a.sections.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, vec!["A", "B", "A"], "{:?}", a.sections);
    let spb = 60.0 / 110.0;
    assert!(
        (a.sections[1].start - 64.0 * spb).abs() < 4.0 * spb,
        "{:?}",
        a.sections
    );
    assert!(
        (a.sections[2].start - 96.0 * spb).abs() < 6.0 * spb,
        "{:?}",
        a.sections
    );
}

#[test]
fn rejects_short_audio() {
    assert!(analyze(&[0.0; 1000], SR, &Options::default()).is_err());
    assert!(analyze(&[], SR, &Options::default()).is_err());
}
