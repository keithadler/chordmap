# Changelog

## Unreleased

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
- Installable app: PNG icons, maskable icons, install button, iOS hint.
- Resampler tabulates its kernel (about ten times faster).
- Live panel under the waveform runs the same visualizer inline while the
  song plays, with its own style switch and a full-screen button.
