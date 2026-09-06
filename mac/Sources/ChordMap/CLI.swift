//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  The command-line face. Exit codes: 0 fine, 1 something to look at, 2 problem, 64 usage.

import Foundation
import AppKit

enum CLI {
    static let usage = """
    chordmac — key, tempo, chords and capo from what the Mac plays (command-line face of Chordmap for Mac)

    USAGE
      chordmac <file> [--capo N] [--json] [--bpm hint] [--genre band|hiphop|dance]
                                                      chart a recording: key, tempo, capo, the chord sheet
      chordmac listen [--source mac|<device>] [--seconds N] [--json]
                                                      chart what the Mac plays, or an input, live
      chordmac synth <out.wav> ["C G Am F"] [--bpm N] [--loops N]
                                                      write a strummed progression to try it with
      chordmac status [--json]                        engine version, permissions, devices
      chordmac devices [--json]                       every audio device with an input
      chordmac history [--json]                       every listen kept
      chordmac screenshots <dir> [--announce]         render windows and promo cards from demo data
      chordmac selftest [--filter S] [--list] [--json]
      chordmac help | version

    The Rust command `chordmap` from the same repository charts files without the app; this one adds
    live listening and the history. Set CHORDMAC_HOME to isolate settings, as the tests do.
    """

    static var version: String {
        if Bundle.main.bundleIdentifier == "com.keithadler.chordmac", let v = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String { return v }
        var url = (Bundle.main.executableURL ?? URL(fileURLWithPath: CommandLine.arguments[0])).resolvingSymlinksInPath()
        while url.path != "/" {
            if url.pathExtension == "app", let b = Bundle(url: url), b.bundleIdentifier == "com.keithadler.chordmac", let v = b.infoDictionary?["CFBundleShortVersionString"] as? String { return v }
            url = url.deletingLastPathComponent()
        }
        return "dev"
    }

    static func runIfRequested() {
        let env = ProcessInfo.processInfo.environment
        if let h = env["CHORDMAC_HOME"], !h.isEmpty {
            Prefs.defaults = UserDefaults(suiteName: "com.keithadler.chordmac.test")!
            Prefs.defaults.removePersistentDomain(forName: "com.keithadler.chordmac.test")
        }
        if env["CHORDMAC_DEMO_DEVICES"] == "1" { Devices.override = Devices.demo }
        let args = Array(CommandLine.arguments.dropFirst())
        guard let cmd = args.first, !cmd.hasPrefix("-psn") else { return }
        exit(run(cmd, Array(args.dropFirst())))
    }

    static func flag(_ n: String, _ a: [String]) -> Bool { a.contains(n) }
    static func value(_ n: String, _ a: [String]) -> String? { guard let i = a.firstIndex(of: n), i + 1 < a.count else { return nil }; return a[i + 1] }
    static let valued = ["--filter", "--seconds", "--capo", "--source", "--bpm", "--genre", "--loops"]
    static func positional(_ a: [String]) -> [String] {
        var out: [String] = []; var skip = false
        for x in a { if skip { skip = false; continue }; if valued.contains(x) { skip = true; continue }; if x.hasPrefix("--") { continue }; out.append(x) }
        return out
    }
    nonisolated(unsafe) static var quiet = false
    nonisolated(unsafe) static var sink: [String]?
    static func out(_ s: String) { if sink != nil { sink?.append(s) } else if !quiet { print(s) } }
    static func err(_ s: String) { if sink != nil { sink?.append(s) } else if !quiet { fputs(s + "\n", stderr) } }
    static func json(_ o: Any) -> String {
        guard JSONSerialization.isValidJSONObject(o), let d = try? JSONSerialization.data(withJSONObject: o, options: [.prettyPrinted, .sortedKeys]) else { return "{}" }
        return String(decoding: d, as: UTF8.self)
    }
    static func dict(_ d: AudioDevice) -> [String: Any] { ["name": d.name, "uid": d.uid, "manufacturer": d.manufacturer, "inputs": d.inputs, "outputs": d.outputs, "sampleRate": d.sampleRate] }

    static func run(_ cmd: String, _ args: [String]) -> Int32 {
        let js = flag("--json", args)
        let pos = positional(args)
        switch cmd {
        case "help", "--help", "-h": out(usage); return 0
        case "version", "--version": out("chordmac \(version), chordmap \(Chordmap.version)"); return 0
        case "status":
            let inputs = Devices.inputs()
            if js { out(json(["version": version, "chordmap": Chordmap.version, "macSound": Capture.macSoundAvailable, "inputs": inputs.map(dict), "listens": History.load().count])) }
            else {
                out("Engine: chordmap \(Chordmap.version)")
                out("The Mac's own sound: " + (Capture.macSoundAvailable ? "available (macOS 14.2 or later)" : "needs macOS 14.2 or later; pick an input"))
                out("Inputs: " + (inputs.isEmpty ? "none" : inputs.map(\.name).joined(separator: ", ")))
                out("Listens kept: \(History.load().count)")
            }
            return 0
        case "devices":
            let list = Devices.inputs()
            if js { out(json(list.map(dict))) } else { if list.isEmpty { out("No inputs."); return 1 }; for d in list { out(String(format: "%-34@ in %d  %.0f Hz", d.name, d.inputs, d.sampleRate)) } }
            return 0
        case "history":
            let list = History.load()
            if js { let e = JSONEncoder(); e.outputFormatting = [.prettyPrinted, .sortedKeys]; e.dateEncodingStrategy = .iso8601; out(String(decoding: (try? e.encode(list)) ?? Data(), as: UTF8.self)) }
            else {
                if list.isEmpty { out("Nothing kept yet."); return 1 }
                for l in list.reversed() { out("\(l.started.formatted(date: .abbreviated, time: .shortened))  \(l.title): \(l.analysis.key.name), \(Int(l.analysis.tempo.bpm.rounded())) bpm, \(l.chordNames.prefix(8).joined(separator: " "))") }
            }
            return 0
        case "synth":
            guard let path = pos.first else { err("chordmac synth <out.wav> [\"C G Am F\"] [--bpm N] [--loops N]"); return 64 }
            let prog = pos.count > 1 ? pos[1] : "C G Am F"
            let bpm = Float(value("--bpm", args) ?? "") ?? 100, loops = Int(value("--loops", args) ?? "") ?? 2
            let (x, sr) = Chordmap.synth(prog, bpm: bpm, loops: loops)
            guard !x.isEmpty else { err("chordmap could not read the progression \"\(prog)\". Try: \"C G Am F\""); return 64 }
            do { try Wave.write(URL(fileURLWithPath: path), left: x, right: x, sampleRate: sr); out(String(format: "%@: %@ at %.0f bpm, %d loops, %.1f s", path, prog, bpm, loops, Double(x.count) / sr)); return 0 }
            catch { err("synth failed: \(error.localizedDescription)"); return 2 }
        case "listen":
            let src: Capture.Source
            if let n = value("--source", args), n != "mac" {
                guard let d = Devices.inputs().first(where: { $0.name.localizedCaseInsensitiveContains(n) }) else { err("No input called \(n)."); return 1 }
                src = .device(d.uid)
            } else { src = .mac }
            let capture = Capture()
            do { try capture.start(src) } catch { err(error.localizedDescription); return 2 }
            let secs = Double(value("--seconds", args) ?? "") ?? 0
            let began = Date()
            out("Listening to \(src.label). Ctrl-C stops.")
            signal(SIGINT) { _ in exit(0) }
            let until = secs > 0 ? began.addingTimeInterval(secs) : Date.distantFuture
            var last: String?
            var nextHear = began.addingTimeInterval(4.2)
            while Date() < until {
                RunLoop.main.run(until: Date().addingTimeInterval(0.25))
                guard Date() >= nextHear else { continue }
                nextHear = Date().addingTimeInterval(EarModel.everySeconds)
                let x = capture.latest(seconds: EarModel.windowSeconds); guard x.count >= Int(4 * capture.sampleRate) else { continue }
                guard let a = try? Chordmap.analyze(x, sampleRate: capture.sampleRate) else { continue }
                let now = a.chord(at: a.duration - 0.5)?.label
                if now != last, let now, !js { out(String(format: "%6.1f s  %@   (%@, %.0f bpm)", capture.seconds, now, a.key.name, a.tempo.bpm)) }
                last = now
            }
            capture.stop()
            let all = capture.all()
            guard all.count >= Int(4 * capture.sampleRate) else { err("Too short to chart: chordmap needs four seconds."); return 1 }
            do {
                let a = try Chordmap.analyze(all, sampleRate: capture.sampleRate)
                let f = DateFormatter(); f.timeStyle = .short
                History.add(Listen(started: began, title: "Listen at \(f.string(from: began))", seconds: capture.seconds, analysis: a))
                return report(a, capo: nil, json: js)
            } catch { err(error.localizedDescription); return 2 }
        case "screenshots":
            guard let dir = pos.first else { err("chordmac screenshots <dir>"); return 64 }
            do { let files = try MainActor.assumeIsolated { try Screenshots.render(to: URL(fileURLWithPath: dir), announce: flag("--announce", args)) }; for f in files { out(f.path) }; return 0 }
            catch { err("screenshots failed: \(error)"); return 2 }
        case "selftest":
            if flag("--list", args) { TestKit.list(); return 0 }
            let results = MainActor.assumeIsolated { TestKit.run(filter: value("--filter", args)) }
            return TestKit.report(results, json: js)
        default:
            // Anything else is a file to chart.
            guard !cmd.hasPrefix("-"), FileManager.default.fileExists(atPath: cmd) else { err(usage); return 64 }
            do {
                let (samples, sr) = try Wave.read(URL(fileURLWithPath: cmd))
                let a = try Chordmap.analyze(samples, sampleRate: sr, bpmHint: Float(value("--bpm", args) ?? ""), genre: value("--genre", args))
                return report(a, capo: Int(value("--capo", args) ?? ""), json: js)
            } catch { err("could not chart \(cmd): \(error.localizedDescription)"); return 2 }
        }
    }

    static func report(_ a: Analysis, capo: Int?, json js: Bool) -> Int32 {
        if js {
            let e = JSONEncoder(); e.outputFormatting = [.prettyPrinted, .sortedKeys]
            out(String(decoding: (try? e.encode(a)) ?? Data(), as: UTF8.self)); return 0
        }
        let flats = Pitch.usesFlats(a.key)
        out("Key: \(a.key.name)" + (a.key.confidence < 0.1 ? " (or \(a.key.alternative))" : ""))
        out(String(format: "Tempo: %.0f bpm, %@", a.tempo.bpm, a.meter))
        let pick = a.guitar.capo
        out(pick == 0 ? "Capo: none, every shape open" : "Capo \(pick): " + a.guitar.shapes.sorted { $0.key < $1.key }.map { "\($0.key) → \($0.value)" }.joined(separator: ", "))
        for o in a.capoOptionsByEase.prefix(3) where o.capo != pick { out(String(format: "  or capo %d, %.0f s of barre chords", o.capo, o.hardSeconds)) }
        for w in a.warnings { out("Note: \(w)") }
        if let capo, capo != 0 {
            out("Shapes with capo \(capo):")
            var names: [String] = []
            for c in a.chords where c.label != "N" { let n = Pitch.shape(c.label, capo: capo, flats: flats); if names.last != n { names.append(n) } }
            out(names.joined(separator: "  "))
        } else { out(""); out(Chordmap.chordSheet(a)) }
        return a.chords.isEmpty ? 1 : 0
    }
}
