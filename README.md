# chordmap

**Use it: [keithadler.github.io/chordmap](https://keithadler.github.io/chordmap/)**. Drop a song, get the chart. Nothing is uploaded.

Tempo, key, chords, sections and a capo suggestion from any audio file.
Pure Rust, compiles to WebAssembly, MIT licensed. The audio never leaves
your browser: there is no server, no account, no analytics.

- **Web app**: drop an MP3, M4A, WAV, FLAC or OGG and get the BPM with half and double buttons and tap tempo, the key with its runner-up, a chord chart per bar grouped by section, a clickable section map that follows playback, and the capo fret that turns the most chords into open shapes. Copy or download the chord sheet.
- **Rust crate** `chordmap`: the engine, no I/O, no `unsafe`, no models to download.
- **npm package** `chordmap`: WebAssembly bindings with TypeScript types.
- **CLI** `chordmap`: one binary for scripts and shells.

```bash
chordmap analyze song.mp3            # chord sheet, sections, capo
chordmap analyze song.mp3 --json     # everything, for other tools
chordmap analyze song.mp3 --bpm 84   # prefer the tempo nearest a hint
chordmap synth test.wav --chords "C G Am F" --bpm 100 --loops 8
```

```ts
import init, { analyze } from "chordmap";
await init();
const a = JSON.parse(analyze(monoFloat32Samples, sampleRate, "{}"));
a.tempo.bpm;            // 89.5
a.key.name;             // "Bb major"
a.guitar.capo;          // 3
a.guitar.shapes;        // { "Bb": "G", "F": "D", "Gm": "Em", "Eb": "C" }
a.bars[4].beats;        // ["Bb", "Bb", "F", "F"]
a.sections[1].guess;    // "chorus"
```

```rust
let a = chordmap::analyze(&samples, sample_rate, &chordmap::Options::default())?;
println!("{}", chordmap::chord_sheet(&a));
```

## How it works

Every step is classic music information retrieval, the same family of
methods librosa and Essentia implement, written from scratch in Rust so it
runs anywhere WebAssembly runs.

1. Audio is resampled to 22.05 kHz and analysed with a 4096-point STFT every 23 ms.
2. **Tempo**: spectral flux on 40 mel bands gives an onset curve; its autocorrelation, weighted by a prior around 120 BPM, ranks tempo candidates. A hint from tap tempo or the half and double buttons picks the octave.
3. **Beats**: dynamic programming over the onset curve (Ellis 2007).
4. **Chords**: a 60-semitone spectrogram is median-filtered in time to drop drum hits, folded to 12 pitch classes, averaged per beat, matched against 24 major and minor templates, and smoothed with Viterbi decoding so chords do not flicker.
5. **Key**: Krumhansl-Kessler profile correlation on the whole song's chroma.
6. **Downbeats**: the beat phase where chord changes and low-end energy line up.
7. **Sections**: chroma plus timbre per bar, a self-similarity matrix, checkerboard novelty for boundaries, then repeated segments are grouped by diagonal similarity. Names like "verse" and "chorus" are guesses from repetition and loudness.
8. **Capo**: every fret from 0 to 7 is scored by how many seconds of the song land on barre chords; the fret that removes the most wins, and a capo has a small cost of its own so it only wins when it removes real barre time.

A four-minute song takes about two seconds natively and five in the browser.

## Limits

- Chords are major and minor triads. Sevenths, suspended chords, inversions and jazz voicings come back as the nearest triad.
- Chord accuracy on a full mix is roughly three in four on pop, rock and folk. Dense arrangements, heavy distortion and solo voice do worse.
- The beat grid assumes a steady tempo and 4/4. Rubato, tempo changes and odd meters produce a wrong grid; `--beats-per-bar` helps for 3/4 and 6/8.
- Half or double tempo happens. The alternatives are always listed; tap tempo settles it.
- Section boundaries land within a bar or so; the verse and chorus labels are heuristics, not detection.
- The first beat of a song is often missed, so the chart may begin with a short pickup bar.

More in [docs/limitations.md](docs/limitations.md).

## How it is tested

No copyrighted audio is used. `chordmap::synth` renders deterministic click
tracks and strummed chord progressions with a bass note and drum clicks. The
test suite checks tempo within 2 percent (or an octave) at 90, 128 and 174
BPM, that a tempo hint picks the octave, that an eight-loop C G Am F
progression comes back as C major with at least 90 percent of beats labelled
correctly, that an A B A arrangement with different timbres returns three
sections labelled A B A with boundaries within six beats, and that the capo
picker sends a Bb song to fret 3 and leaves a C song alone.

```bash
cargo test --release --workspace
./scripts/build-wasm.sh && python3 -m http.server --directory examples/web
```

## Layout

- `crates/chordmap`: the engine (`dsp`, `features`, `tempo`, `chroma`, `chords`, `key`, `sections`, `guitar`, `synth`, `analysis`).
- `crates/chordmap-wasm`: wasm-bindgen wrapper; `scripts/build-wasm.sh` writes `pkg/`.
- `crates/chordmap-cli`: `chordmap analyze` and `chordmap synth`, decoding through symphonia.
- `examples/web`: the static web app deployed to GitHub Pages.

## License

MIT. See [LICENSE](LICENSE).
