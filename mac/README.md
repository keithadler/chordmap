# Chordmap for Mac

The chordmap engine as a Mac app: play a song in any app on the Mac and see the key, the tempo, the chords bar by bar, the sections and where the capo goes, live. It hears; it never records. Nothing leaves the Mac.

## Download

**[Download Chordmap-for-Mac-1.0.0.dmg](https://github.com/keithadler/chordmap/releases/download/mac-v1.0.0/Chordmap-for-Mac-1.0.0.dmg)** (macOS 14 or later, Apple Silicon)

Open the DMG, drag the app to Applications, open it. The first time, macOS says the app is from an unidentified developer: right-click the app, choose Open, then Open again. That is once.

One permission, asked for once and explained in the dialog: **System Audio Recording**, which is how macOS describes hearing what the Mac plays. If you pick an input device instead, it asks for the **Microphone** once. The app records nothing with either.

![Chordmap for Mac](docs/screenshots/live.png)

## What it does

Press the button, play a song. Every two seconds chordmap re-hears the last 24 seconds and shows the chord now, the key with its runner-up, the tempo and meter, and the bars of that stretch grouped by section. The Capo card lists the best four frets with how many seconds of barre chords each leaves, the engine's pick marked; choose one and every chord shown becomes the shape your hand makes. Stop, and the whole listen is charted and kept in the sidebar with chordmap's chord sheet to copy.

An input device works too: a guitar into an interface, or a microphone in front of a record player.

![A kept listen with its chart and chord sheet](docs/screenshots/listen.png)

## Honest limits

- It does not know the song's name. No database; it only hears.
- A clear mix charts well; a wall of distortion, a solo voice or a beat with little harmony charts less well, and sparse harmony is flagged, not invented.
- Major and minor chords and sevenths. Four seconds is the least it will chart; the live view lags by about two seconds.
- Hearing the Mac's own sound needs macOS 14.2 or later; on 14.0 and 14.1 pick an input.
- Apple Silicon only for now. The Intel build needs the Rust toolchain's Intel target on the build machine; it follows.

## Command line

`chordmac` is linked into your PATH by the installer, or call `/Applications/Chordmap\ for\ Mac.app/Contents/MacOS/ChordMap`. The Rust command `chordmap` from this repository charts files without the app; `chordmac` adds live listening and the history.

```
chordmac song.wav [--capo 3] [--json]     chart a recording: key, tempo, capo, the chord sheet
chordmac listen [--source mac|Scarlett]   chart what the Mac plays, or an input, live
chordmac synth test.wav "C G Am F"        a strummed progression to try it with
chordmac history                          every listen kept
chordmac status | devices | selftest
```

Exit codes: 0 fine, 1 something to look at, 2 problem, 64 usage.

## Building

Swift Package, macOS 14 or later, the Command Line Tools plus Rust (rustup). `ffi/` is a small C-ABI crate over `../crates/chordmap`, built as a static library by `ffi/build.sh` for every Apple target the toolchain has; `build-app.sh --install` builds the app and installs it, `make-dmg.sh` makes the disk image. `ChordMap selftest`, `swift test` and `tests/integration.sh` are the tests; they run on chordmap's own synthesised strums and never open a device.

## Privacy

See [PRIVACY.md](PRIVACY.md). Sound stays in memory for a listen; the history keeps chord names, not audio; the only connection is the optional daily update check.

## License

MIT, like the rest of chordmap. Made by Keith Adler. More from the same maker at [keithadler.github.io](https://keithadler.github.io).
