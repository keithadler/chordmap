//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  chordmap (https://github.com/keithadler/chordmap) is the Ear's brain: tempo, key, chords,
//  bars, sections and the capo, from audio, pure Rust, linked in as a static library.
//  This file is the Swift side of that bridge.

import Foundation
import CChordmap

/// chordmap's `Analysis`, as it comes back over the bridge.
struct Analysis: Codable, Equatable {
    struct Tempo: Codable, Equatable { var bpm: Float; var confidence: Float; var alternatives: [Float] }
    struct Key: Codable, Equatable { var name: String; var tonic: String; var minor: Bool; var confidence: Float; var alternative: String }
    struct ChordSpan: Codable, Equatable, Identifiable { var start: Float; var end: Float; var label: String; var id: Float { start } }
    struct Bar: Codable, Equatable, Identifiable { var start: Float; var end: Float; var beats: [String]; var id: Float { start } }
    struct Section: Codable, Equatable, Identifiable { var start: Float; var end: Float; var label: String; var guess: String; var bar: Int; var id: Float { start } }
    struct CapoOption: Codable, Equatable, Identifiable { var capo: Int; var hardSeconds: Float; var score: Float; var id: Int { capo } }
    struct Guitar: Codable, Equatable { var capo: Int; var shapes: [String: String]; var options: [CapoOption] }
    var version: String
    var duration: Float
    var tuningCents: Float
    var meter: String
    var beatsPerBar: Int
    var genre: String
    var harmonicity: Float
    var tempo: Tempo
    var key: Key
    var beats: [Float]
    var downbeats: [Float]
    var chords: [ChordSpan]
    var bars: [Bar]
    var sections: [Section]
    var guitar: Guitar
    var warnings: [String]

    /// The chord sounding at `t` seconds.
    func chord(at t: Float) -> ChordSpan? { chords.last { $0.start <= t && t < $0.end } ?? (t >= duration ? chords.last : nil) }
    /// Chord labels in order, repeats merged.
    var chordNames: [String] { var out: [String] = []; for c in chords where c.label != "N" && out.last != c.label { out.append(c.label) }; return out }
    var capoOptionsByEase: [CapoOption] { guitar.options.sorted { $0.score != $1.score ? $0.score < $1.score : $0.capo < $1.capo } }
}

struct ChordmapError: LocalizedError { let message: String; var errorDescription: String? { message } }

enum Chordmap {
    static let decoder = JSONDecoder()
    static var version: String { take(chordmap_version()) ?? "unknown" }

    /// Mono samples in, analysis out. chordmap wants at least four seconds.
    static func analyze(_ samples: [Float], sampleRate: Double, bpmHint: Float? = nil, genre: String? = nil) throws -> Analysis {
        var opts: [String: Any] = [:]
        if let bpmHint { opts["bpmHint"] = bpmHint }
        if let genre { opts["genre"] = genre }
        let optJSON = opts.isEmpty ? "" : (String(data: (try? JSONSerialization.data(withJSONObject: opts)) ?? Data(), encoding: .utf8) ?? "")
        let json: String? = samples.withUnsafeBufferPointer { buf in
            optJSON.withCString { o in take(chordmap_analyze(buf.baseAddress, buf.count, UInt32(sampleRate.rounded()), o)) }
        }
        guard let json, let data = json.data(using: .utf8) else { throw ChordmapError(message: "chordmap returned nothing.") }
        if let err = try? JSONSerialization.jsonObject(with: data) as? [String: Any], let e = err["error"] as? String { throw ChordmapError(message: e) }
        do { return try decoder.decode(Analysis.self, from: data) }
        catch { throw ChordmapError(message: "Could not read chordmap's answer: \(error)") }
    }

    /// chordmap's plain-text chord sheet for an analysis.
    static func chordSheet(_ a: Analysis) -> String {
        guard let d = try? JSONEncoder().encode(a), let s = String(data: d, encoding: .utf8) else { return "" }
        return s.withCString { take(chordmap_chord_sheet($0)) } ?? ""
    }

    /// A strummed progression from chordmap's own synth, at its sample rate.
    static func synth(_ chords: String = "C G Am F", bpm: Float = 100, loops: Int = 2) -> (samples: [Float], sampleRate: Double) {
        var n = 0
        let p = chords.withCString { chordmap_synth($0, bpm, loops, &n) }
        guard let p, n > 0 else { return ([], Double(chordmap_sample_rate())) }
        defer { chordmap_free_samples(p, n) }
        return (Array(UnsafeBufferPointer(start: p, count: n)), Double(chordmap_sample_rate()))
    }

    private static func take(_ p: UnsafeMutablePointer<CChar>?) -> String? {
        guard let p else { return nil }
        defer { chordmap_free(p) }
        return String(cString: p)
    }
}
