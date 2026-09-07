# Changelog

## Unreleased

- Lyrics in the visualizers: Stage, Flow and Aurora draw the bar being
  sung with the current word lit and the next bar dimmed, full screen and
  in the live panel.
- Fix the words: a lyrics editor with a row per bar, its time and chords,
  Tab down the song; focusing a row cues the player to that bar. Words
  typed in stay inside their bar.
- The page is fetched fresh on every visit with a signal and a bar offers
  a reload when a newer build takes over, so a deploy no longer needs two
  loads to show up.
- Lyrics: Whisper on the separated vocals, on device, words under the
  chords per bar, a follow-along line in the footer, chords placed inside
  the lyric line in ChordPro and the text sheet, per-bar correction in edit
  mode, kept with the chart and the share link.

## Mac app 1.0.0 (2026-09-06)

Chordmap for Mac, in `mac/`: the engine as a macOS app that hears what the Mac plays through a Core Audio tap (or an input device) and charts it live every two seconds, with the capo picker, bars by section, the chord sheet, and a history of listens. Swift over a small C bridge to the crate; Apple Silicon build, signed by its author.

## 1.0.0 (2026-09-06)

First public release, live at https://keithadler.github.io/chordmap/.

- First version: tempo with candidates and hints, beat tracking, major and
  minor chords per beat, key with runner-up, downbeats and bars, sections
  with repeat grouping and plain-English guesses, capo suggestion with
  shapes, chord sheet text, JSON output, CLI, WebAssembly package and web app.
- Seventh chords (dominant, major, minor) with a margin over the plain triad.
- 4/4 versus 3/4 detection, overridable with `beatsPerBar`.
- Tuning estimation and correction for recordings away from A440.
- Peak-based pitch spectrum and harmonic-aware chroma.
- Web app: now-playing footer with the last three, current and next three
  chords, space to play and pause, print stylesheet, offline app shell.
- Web app: guitar fingering diagrams for the shapes, section loop with
  50 to 100 percent speed at the same pitch, Nashville numbers view, MIDI
  and ChordPro downloads, share link with the chart gzipped into the URL,
  arrow keys step through bars.
- Web app: edit mode to correct chords (per beat or whole bar) and rename
  sections, bar-line nudge by one beat, recent-charts library in IndexedDB
  with corrections restored for the same file, multi-file drop, ukulele
  diagrams.
- Web app design pass: waveform timeline tinted by section with beat ticks
  and playhead, chart bars that sweep as they play, now-playing footer with
  a beat pulse and a pitch-class ring, ambient background, gradient
  headline, listening animation, sample chart on the landing page, both
  themes, reduced-motion respected. Still no external fonts or scripts.
- Visualize mode: Stage, Flow and Aurora, full screen, keyboard driven
  (1 2 3, space, arrows, F, Esc), fed by the chart and an AnalyserNode on
  the playing audio.
- Practice mix: original, centre-cancel instrumental or vocals, AI stems
  (KUIELab MDX-Net vocals, drums, bass, other via onnxruntime-web, WebGPU
  when available) with a stem mixer, pitch shift in semitones at the same
  tempo, and "chart again from the stems".
- Genre switch (band, hip hop, dance): tempo prior at 120, 90 or 128 BPM,
  hook and drop section names, sparse-harmony detection with a warning.
- Progress bar for AI stems: model download, separation percent, time
  left, stem steps, all marked as on this device.
- Landing page rewritten around free charts, on-device AI stems and
  play-along from your own music, with six feature cards.
- Every place the AI appears says it runs on the device and uploads nothing.
- Installable app: PNG icons, maskable icons, install button, iOS hint.
- Resampler tabulates its kernel (about ten times faster).
- Live panel under the waveform runs the same visualizer inline while the
  song plays, with its own style switch and a full-screen button.
