//  Chordmap for Mac — MIT licensed. See LICENSE.

import Foundation

enum CLISuite {
    static func run(_ cmd: String, _ args: [String] = []) -> (code: Int32, out: String) {
        CLI.sink = []; defer { CLI.sink = nil }
        let code = CLI.run(cmd, args)
        return (code, (CLI.sink ?? []).joined(separator: "\n"))
    }
    static var suite: TestSuite { TestSuite(name: "CLI", cases: [
        TestCase(name: "help, version, usage error") { t in
            t.equal(run("help").code, 0, "help"); t.check(run("version").out.contains("chordmap 1."), "version names the engine: \(run("version").out)")
            t.equal(run("nonsense").code, 64, "unknown command is a usage error")
            t.equal(run("synth").code, 64, "synth without a file")
        },
        TestCase(name: "status and devices on demo hardware") { t in
            Devices.override = Devices.demo; defer { Devices.override = nil }
            let (c, o) = run("status", ["--json"])
            t.equal(c, 0, "exit"); t.check(o.contains("\"chordmap\" : \"1."), "engine version: \(o)"); t.check(o.contains("Scarlett Solo USB"), "inputs listed")
            t.check(run("devices").out.contains("MacBook Pro Microphone"), "devices")
        },
        TestCase(name: "synth then chart a file") { t in
            let dir = TestKit.tempDir(); defer { try? FileManager.default.removeItem(at: dir) }
            let f = dir.appendingPathComponent("chords.wav").path
            t.equal(run("synth", [f, "C G Am F", "--loops", "1"]).code, 0, "synth")
            let (c, o) = run(f)
            t.equal(c, 0, "exit"); t.check(o.contains("Key: C major"), "key: \(o)"); t.check(o.contains("Capo: none"), "capo: \(o)"); t.check(o.contains("Tempo:"), "tempo")
            let (c2, o2) = run(f, ["--capo", "3"])
            t.equal(c2, 0, "exit"); t.check(o2.contains("Shapes with capo 3:") && o2.contains(" A "), "capo 3 shows C as an A shape: \(o2)")
            let (c3, o3) = run(f, ["--json"])
            t.equal(c3, 0, "exit"); t.check(o3.contains("\"guitar\"") && o3.contains("\"tonic\" : \"C\""), "json: \(o3.prefix(200))")
            t.equal(run("synth", [f, "nonsense here"]).code, 64, "bad progression is a usage error")
            t.equal(run(dir.appendingPathComponent("missing.wav").path).code, 64, "a missing file is a usage error")
        },
        TestCase(name: "history lists what was kept") { t in
            History.overrideDir = TestKit.tempDir(); defer { try? FileManager.default.removeItem(at: History.overrideDir!); History.overrideDir = nil }
            t.equal(run("history").code, 1, "empty history exits 1")
            let (x, sr) = Chordmap.synth("G D Em C", bpm: 120, loops: 1)
            let a = try Chordmap.analyze(x, sampleRate: sr)
            History.add(Listen(started: Date(), title: "Porch", seconds: 8, analysis: a))
            let (c, o) = run("history")
            t.equal(c, 0, "exit"); t.check(o.contains("Porch") && o.contains("G major"), "lists the listen: \(o)")
            t.check(run("history", ["--json"]).out.contains("\"title\" : \"Porch\""), "json")
        },
    ]) }
}
