//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  The Ear's own small pieces: chord labels moved by a capo, a saved listen, and the history.
//  The hearing itself is chordmap (see Chordmap.swift).

import Foundation

enum Pitch {
    static let sharp = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
    static let flat = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"]
    static func name(_ pc: Int, flats: Bool) -> String { (flats ? flat : sharp)[((pc % 12) + 12) % 12] }
    /// "Bb" -> (10, ""), "F#m7" -> (6, "m7"), "C/E" -> (0, "/E"). "N" (no chord) is nil.
    static func parse(_ label: String) -> (root: Int, suffix: String)? {
        guard let first = label.first, let i = "CDEFGAB".firstIndex(of: first) else { return nil }
        var root = [0, 2, 4, 5, 7, 9, 11]["CDEFGAB".distance(from: "CDEFGAB".startIndex, to: i)]
        var rest = label.dropFirst()
        if rest.first == "#" { root += 1; rest = rest.dropFirst() } else if rest.first == "b" { root -= 1; rest = rest.dropFirst() }
        return ((root + 12) % 12, String(rest))
    }
    /// A chord label moved down by `fret` semitones: what the hand plays with a capo there.
    static func shape(_ label: String, capo fret: Int, flats: Bool) -> String {
        guard fret != 0, let (root, suffix) = parse(label) else { return label }
        var out = name(root - fret, flats: flats)
        if let slash = suffix.firstIndex(of: "/"), let (bass, _) = parse(String(suffix[suffix.index(after: slash)...])) {
            out += String(suffix[..<slash]) + "/" + name(bass - fret, flats: flats)
        } else { out += suffix }
        return out
    }
    /// Keys spelt with flats spell their chords with flats.
    static func usesFlats(_ key: Analysis.Key) -> Bool { key.tonic.contains("b") || key.tonic == "F" || (key.minor && ["D", "G", "C"].contains(key.tonic)) }
}

/// A saved listen, for the history.
struct Listen: Codable, Identifiable, Equatable {
    var id = UUID()
    var started: Date
    var title: String
    var seconds: Double
    var analysis: Analysis
    var chordNames: [String] { analysis.chordNames }
}

enum History {
    nonisolated(unsafe) static var overrideDir: URL?
    static var dir: URL {
        if let overrideDir { return overrideDir }
        if let h = ProcessInfo.processInfo.environment["CHORDMAC_HOME"], !h.isEmpty { return URL(fileURLWithPath: h) }
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("Chordmap for Mac")
    }
    static var file: URL { dir.appendingPathComponent("listens.json") }
    static let encoder: JSONEncoder = { let e = JSONEncoder(); e.outputFormatting = [.prettyPrinted, .sortedKeys]; e.dateEncodingStrategy = .iso8601; return e }()
    static let decoder: JSONDecoder = { let d = JSONDecoder(); d.dateDecodingStrategy = .iso8601; return d }()
    static func load() -> [Listen] { (try? decoder.decode([Listen].self, from: Data(contentsOf: file))) ?? [] }
    static func save(_ list: [Listen]) {
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try? encoder.encode(Array(list.suffix(200))).write(to: file, options: .atomic)
    }
    static func add(_ l: Listen) { var list = load(); list.append(l); save(list) }
}
