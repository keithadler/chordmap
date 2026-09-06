**[Download Chordmap-for-Mac-1.0.0.dmg](https://github.com/keithadler/chordmap/releases/download/mac-v1.0.0/Chordmap-for-Mac-1.0.0.dmg)** (macOS 14 or later, Apple Silicon)

Open the DMG, drag the app to Applications, open it. The first time, macOS says the app is from an unidentified developer: right-click the app, choose Open, then Open again. That is once.

**What it does.** Play a song in any app on the Mac and see the chords as they go by: the key with its runner-up, the tempo and meter, the bars grouped by section, and where the capo goes so the open shapes play it. Stop, and the whole listen is charted and kept with chordmap's chord sheet to copy. A guitar into an interface works too. It hears; it never records; it does not know the song's name. The engine is the chordmap crate from this repository, linked in as a static library.

**What it asks for.** System Audio Recording, once, which is how macOS describes hearing what the Mac plays (macOS 14.2 or later); the Microphone, once, only if you pick an input device. Nothing is recorded with either, and nothing leaves the Mac except the optional daily check of this page for a new version.

**Honest limits.** A clear mix charts well, a wall of distortion less well, and sparse harmony is flagged rather than invented. Major and minor chords and sevenths. Four seconds is the least it will chart; the live view lags by about two seconds. Apple Silicon only in this build; the Intel build follows.

SHA-256 of the DMG: `969c4e884fb73b3933bebad19c017876b6bc6200d73fb2a489637f97a385f527`
