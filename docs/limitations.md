# Limitations

Honest notes on what chordmap does and does not do, so the chart is read
with the right amount of trust.

## Tempo and beats
- Tempo comes from onset autocorrelation with a prior around 120 BPM, so
  slow songs are sometimes reported at double speed and fast ones at half.
  `tempo.alternatives` always lists the other candidates; `bpmHint` (tap
  tempo in the app, `--bpm` on the CLI) picks between them.
- The beat tracker assumes a steady tempo. Rubato, ritardandos and tempo
  changes mid-song produce a drifting grid, and the chord chart drifts with it.
- 4/4 is assumed unless `beatsPerBar` says otherwise. There is no meter
  detection.
- Onsets are detected from spectral flux, which is late by about a quarter
  of the analysis window; beat times are corrected for that but can still sit
  20 to 30 ms off the true attack.
- The very first beat is often trimmed as "weak" because the onset curve has
  no history yet, so charts frequently start with a pickup bar.

## Chords
- Only 24 chords exist: 12 major and 12 minor triads, plus "N" for silence.
  Dominant sevenths, major sevenths, suspended and diminished chords, power
  chords and slash chords are mapped to the nearest triad.
- Chroma comes from the full mix. Drums are attenuated by a median filter in
  time, not removed; bass lines and melodies still colour the chroma. Expect
  roughly three of four beats right on pop, rock, folk and worship music,
  fewer on dense or distorted mixes, jazz and solo voice.
- Viterbi smoothing prefers holding a chord, so fast changes (two per beat)
  are lost and a change can land one beat late.
- Chords are spelled by the detected key (flats in flat keys), not by voice
  leading.

## Key
- Krumhansl-Kessler profile correlation on the whole song. Relative major
  and minor are the usual confusion; `key.alternative` shows the runner-up
  and `key.confidence` is the gap between them. Songs that modulate report
  the dominant key only.

## Sections
- Boundaries come from a checkerboard novelty kernel over a bar-level
  self-similarity matrix and are snapped to bars, so they are typically
  within one bar of the true change.
- Repeats are grouped by similarity of chroma and timbre along a diagonal,
  allowing up to three bars of offset. Two sections with the same chords and
  the same instrumentation will be grouped even if a listener would call
  them verse and chorus.
- "verse", "chorus", "intro", "outro" and "bridge" are heuristics: the most
  repeated and loudest group is the chorus, the other repeated group the
  verse, single sections at the ends are intro and outro.
- Segments longer than 16 bars are split at their strongest inner novelty
  peak, which can cut a long section in an arbitrary place.

## Capo
- Difficulty is a fixed table: C, D, E, G, A, Em, Am and Dm are open; F is
  easy; Bm, F#m, B and Bb are moderate barres; everything else is a full
  barre. Every fret from 0 to 7 is scored by seconds of hard shapes and a
  small per-fret cost, so the capo only wins when it removes real barre time.
- The table is for standard tuning. No drop tunings, no ukulele, no piano.

## Input
- The web app relies on the browser's decoder: Chrome and Firefox play MP3,
  M4A/AAC, WAV, FLAC and OGG; Safari lacks OGG and some FLAC. The CLI uses
  symphonia and reads MP3, AAC/M4A, FLAC, OGG Vorbis and WAV.
- Audio shorter than four seconds is rejected.
