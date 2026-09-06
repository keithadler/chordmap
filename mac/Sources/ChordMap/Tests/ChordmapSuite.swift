//  Chordmap for Mac — MIT licensed. See LICENSE.
//  The bridge to chordmap, on chordmap's own synthesised strums.

import Foundation

enum ChordmapSuite {
    static var suite: TestSuite { TestSuite(name: "Chordmap", cases: [
        TestCase(name: "the library is there and says its version") { t in
            t.check(Chordmap.version.hasPrefix("1."), "chordmap version: \(Chordmap.version)")
            let (x, sr) = Chordmap.synth("C G Am F", bpm: 100, loops: 1)
            t.equal(sr, 22050, "synth sample rate")
            t.check(x.count > 22050 * 8, "four chords of four beats at 100 bpm is about 9.6 s: \(x.count) samples")
            t.check(Chordmap.synth("not chords", bpm: 100, loops: 1).samples.isEmpty, "nonsense gives nothing")
        },
        TestCase(name: "hears C G Am F in C major with no capo") { t in
            let (x, sr) = Chordmap.synth("C G Am F", bpm: 100, loops: 2)
            let a = try Chordmap.analyze(x, sampleRate: sr)
            t.equal(a.key.name, "C major", "key")
            t.check(Array(a.chordNames.prefix(4)) == ["C", "G", "Am", "F"], "chords in order: \(a.chordNames)")
            t.equal(a.guitar.capo, 0, "capo")
            t.check([50, 100, 200].contains { abs(a.tempo.bpm - $0) < 3 }, "tempo near 100 or a multiple: \(a.tempo.bpm)")
            t.check(a.bars.count >= 6, "bars: \(a.bars.count)")
            t.check(Chordmap.chordSheet(a).contains("C"), "chord sheet has a C")
            t.check(a.chord(at: 1)?.label == "C", "chord at one second is C: \(a.chord(at: 1)?.label ?? "none")")
        },
        TestCase(name: "a flat key gets a capo and shapes") { t in
            let (x, sr) = Chordmap.synth("Eb Ab Bb Cm", bpm: 96, loops: 2)
            let a = try Chordmap.analyze(x, sampleRate: sr)
            t.check(a.key.name.hasPrefix("Eb") || a.key.name.hasPrefix("D#"), "key: \(a.key.name)")
            t.check(a.guitar.capo == 3 || a.guitar.capo == 1, "capo 3 (C shapes) or 1 (D shapes): \(a.guitar.capo)")
            t.check(!a.guitar.shapes.isEmpty, "shapes listed: \(a.guitar.shapes)")
            t.equal(a.guitar.options.count, 8, "frets 0 to 7 scored")
        },
        TestCase(name: "too short is refused, not guessed") { t in
            do { _ = try Chordmap.analyze([Float](repeating: 0.1, count: 22050), sampleRate: 22050); t.fail("one second should throw") }
            catch { t.check("\(error.localizedDescription)".contains("four seconds"), "says why: \(error.localizedDescription)") }
        },
        TestCase(name: "labels move with the capo") { t in
            t.equal(Pitch.shape("Bb", capo: 3, flats: false), "G", "Bb with capo 3 is a G shape")
            t.equal(Pitch.shape("F#m", capo: 2, flats: false), "Em", "F#m with capo 2 is Em")
            t.equal(Pitch.shape("C/E", capo: 1, flats: false), "B/D#", "slash chords move both notes")
            t.equal(Pitch.shape("Ebmaj7", capo: 3, flats: true), "Cmaj7", "suffix kept")
            t.equal(Pitch.shape("N", capo: 3, flats: true), "N", "no chord stays no chord")
            t.equal(Pitch.shape("G", capo: 0, flats: false), "G", "no capo, no change")
        },
        TestCase(name: "history saves and loads in its own folder") { t in
            History.overrideDir = TestKit.tempDir(); defer { try? FileManager.default.removeItem(at: History.overrideDir!); History.overrideDir = nil }
            let (x, sr) = Chordmap.synth("D A Bm G", bpm: 110, loops: 1)
            let a = try Chordmap.analyze(x, sampleRate: sr)
            History.add(Listen(started: Date(), title: "Tuesday", seconds: 10, analysis: a))
            let back = History.load()
            t.equal(back.count, 1, "one listen"); t.equal(back.first?.title, "Tuesday", "title kept")
            t.equal(back.first?.analysis, a, "the whole analysis comes back")
            t.check((back.first?.chordNames ?? []).contains("D") && (back.first?.chordNames ?? []).contains("G"), "chords kept: \(back.first?.chordNames ?? [])")
        },
    ]) }
}
