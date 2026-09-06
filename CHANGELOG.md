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
