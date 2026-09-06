//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  The window: what is playing, in what key and tempo, with which capo, bar by bar, and everything
//  it heard before. chordmap does the hearing.

import SwiftUI
import AppKit

/// Settings. Named Prefs because a `Settings` enum collides with SwiftUI's Settings scene.
enum Prefs {
    nonisolated(unsafe) static var defaults = UserDefaults.standard
    static var menuBar: Bool { get { defaults.object(forKey: "menuBar") as? Bool ?? true } set { defaults.set(newValue, forKey: "menuBar") } }
}

final class FlagBox: @unchecked Sendable {
    private let lock = NSLock(); private var v = false
    var value: Bool { get { lock.lock(); defer { lock.unlock() }; return v } set { lock.lock(); v = newValue; lock.unlock() } }
}

@MainActor
final class EarModel: ObservableObject {
    static let shared = EarModel()
    let capture = Capture()
    @Published var listening = false
    @Published var source: Capture.Source = Capture.macSoundAvailable ? .mac : .device("") { didSet { if let d = try? JSONEncoder().encode(source) { Prefs.defaults.set(d, forKey: "earSource") }; if listening { stop(); start() } } }
    /// The last few seconds, re-heard every couple of seconds while listening.
    @Published var window: Analysis?
    /// The whole listen, once stopped.
    @Published var whole: Analysis?
    @Published var chord: String?
    /// The user's capo choice; nil means chordmap's own pick.
    @Published var capo: Int? { didSet { Prefs.defaults.set(capo ?? -1, forKey: "capo") } }
    @Published var elapsed: Double = 0
    @Published var problem: String?
    @Published var busy = false
    @Published var listens: [Listen] = History.load()
    @Published var viewing: Listen?
    /// Screenshots: fixed state, no capture.
    var demo = false
    private var started = Date()
    private var timer: DispatchSourceTimer?
    private let working = FlagBox()
    nonisolated static let windowSeconds = 24.0, everySeconds = 2.0

    init() {
        if let d = Prefs.defaults.data(forKey: "earSource"), let s = try? JSONDecoder().decode(Capture.Source.self, from: d) { source = s }
        let c = Prefs.defaults.integer(forKey: "capo"); capo = Prefs.defaults.object(forKey: "capo") != nil && c >= 0 ? c : nil
    }

    var analysis: Analysis? { viewing?.analysis ?? whole ?? window }
    var flats: Bool { analysis.map { Pitch.usesFlats($0.key) } ?? false }
    var effectiveCapo: Int { capo ?? analysis?.guitar.capo ?? 0 }
    /// A sounding chord as the shape the hand makes with the chosen capo.
    func shown(_ label: String) -> String {
        guard label != "N" else { return "–" }
        let fret = effectiveCapo
        if let a = analysis, fret == a.guitar.capo, let s = a.guitar.shapes[label] { return s }
        return Pitch.shape(label, capo: fret, flats: flats)
    }
    var statusLine: String {
        if let problem { return problem }
        if listening { return String(format: "Listening to %@. %d s.%@", source.label, Int(elapsed), elapsed < 4 ? " The first chart comes after four seconds." : "") }
        if busy { return "Hearing the whole listen…" }
        return "Off. Press the button and play something on the Mac, or through the input."
    }

    func toggle() { listening ? stop() : start() }
    func start() {
        problem = nil; viewing = nil; whole = nil; window = nil; chord = nil; elapsed = 0
        do { try capture.start(source) } catch { problem = error.localizedDescription; return }
        started = Date(); listening = true
        let t = DispatchSource.makeTimerSource(queue: DispatchQueue.global(qos: .userInitiated))
        t.schedule(deadline: .now() + 4.2, repeating: EarModel.everySeconds)
        t.setEventHandler { [weak self] in self?.hear() }
        t.resume(); timer = t
    }
    nonisolated private func hear() {
        let (samples, rate, seconds) = MainActor.assumeIsolatedIfPossible { (self.capture.latest(seconds: EarModel.windowSeconds), self.capture.sampleRate, self.capture.seconds) }
        guard samples.count >= Int(4 * rate), !working.value else { return }
        working.value = true; defer { working.value = false }
        let result = try? Chordmap.analyze(samples, sampleRate: rate)
        Task { @MainActor in
            guard self.listening else { return }
            self.elapsed = seconds
            if let a = result {
                self.window = a
                let now = a.chord(at: a.duration - 0.5)?.label
                if now != self.chord { self.chord = now }
            }
        }
    }
    func stop() {
        timer?.cancel(); timer = nil
        capture.stop(); listening = false; chord = nil
        let all = capture.all(), rate = capture.sampleRate, began = started, secs = capture.seconds
        guard all.count >= Int(4 * rate) else { return }
        busy = true
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let a = try? Chordmap.analyze(all, sampleRate: rate)
            Task { @MainActor in
                guard let self else { return }
                self.busy = false
                guard let a else { self.problem = "chordmap could not hear a chart in that."; return }
                self.whole = a
                let f = DateFormatter(); f.dateStyle = .none; f.timeStyle = .short
                let l = Listen(started: began, title: "Listen at \(f.string(from: began))", seconds: secs, analysis: a)
                History.add(l); self.listens = History.load(); self.viewing = l
            }
        }
    }
    func rename(_ l: Listen, to title: String) {
        guard let i = listens.firstIndex(where: { $0.id == l.id }) else { return }
        listens[i].title = title; History.save(listens); if viewing?.id == l.id { viewing = listens[i] }
    }
    func delete(_ l: Listen) { listens.removeAll { $0.id == l.id }; History.save(listens); if viewing?.id == l.id { viewing = nil } }
    func show(_ l: Listen) { viewing = l; chord = nil }
}

extension MainActor {
    nonisolated static func assumeIsolatedIfPossible<T: Sendable>(_ body: @MainActor () -> T) -> T {
        if Thread.isMainThread { return MainActor.assumeIsolated(body) }
        return DispatchQueue.main.sync { MainActor.assumeIsolated(body) }
    }
}

struct EarPanel: View {
    @EnvironmentObject var ear: EarModel
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                if let v = ear.viewing { ListenCard(listen: v) }
                nowCard
                capoCard
                chartCard
                if let a = ear.analysis, ear.viewing != nil || ear.whole != nil { SheetCard(text: Chordmap.chordSheet(a)) }
                Text("Chordmap hears a clear mix well and a wall of distortion less well, flags sparse harmony instead of inventing chords, and does not know a song's name. It records nothing.")
                    .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
        }.navigationTitle("Chordmap for Mac")
    }
    private var header: some View {
        HStack(alignment: .top, spacing: 18) {
            Button { ear.toggle() } label: {
                Image(systemName: "ear").font(.system(size: 28, weight: .bold)).frame(width: 66, height: 66)
                    .background(ear.listening ? Color.teal : Color.secondary.opacity(0.18), in: Circle())
                    .foregroundStyle(ear.listening ? .white : .secondary)
                    .shadow(color: ear.listening ? .teal.opacity(0.6) : .clear, radius: 12)
            }.buttonStyle(.plain).help(ear.listening ? "Stop listening (⌘E)" : "Listen (⌘E)").accessibilityLabel(ear.listening ? "Stop listening" : "Listen")
            VStack(alignment: .leading, spacing: 6) {
                Text(ear.viewing?.title ?? "Chordmap").font(.title.bold())
                Text(ear.statusLine).foregroundStyle(ear.problem == nil ? Color.secondary : Color.orange).fixedSize(horizontal: false, vertical: true)
                if ear.problem?.contains("Privacy") == true { Button("Open System Settings") { NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture")!) }.buttonStyle(.link).font(.callout) }
            }
            Spacer(minLength: 12)
            Picker("Source", selection: $ear.source) {
                if Capture.macSoundAvailable { Text("The Mac's own sound").tag(Capture.Source.mac) }
                ForEach(Devices.inputs()) { d in Text(d.name).tag(Capture.Source.device(d.uid)) }
            }.frame(width: 260).labelsHidden().controlSize(.small)
        }
    }
    private var nowCard: some View {
        HStack(alignment: .firstTextBaseline, spacing: 28) {
            VStack(alignment: .leading, spacing: 4) {
                Text(ear.viewing != nil ? "First" : "Now").font(.caption).foregroundStyle(.secondary)
                let label = ear.chord ?? ear.analysis?.chordNames.first
                Text(label.map { ear.shown($0) } ?? "–").font(.system(size: 64, weight: .bold, design: .rounded))
                if let label, ear.effectiveCapo != 0 { Text("sounds as \(label)").font(.callout).foregroundStyle(.secondary) }
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Key").font(.caption).foregroundStyle(.secondary)
                Text(ear.analysis?.key.name ?? "–").font(.system(size: 34, weight: .semibold, design: .rounded))
                if let k = ear.analysis?.key, k.confidence < 0.1 { Text("or \(k.alternative)").font(.callout).foregroundStyle(.secondary) }
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Tempo").font(.caption).foregroundStyle(.secondary)
                Text(ear.analysis.map { String(format: "%.0f", $0.tempo.bpm) } ?? "–").font(.system(size: 34, weight: .semibold, design: .rounded))
                if let a = ear.analysis { Text("\(a.meter)").font(.callout).foregroundStyle(.secondary) }
            }
            Spacer()
        }.padding(16).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }
    private var capoCard: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Capo").font(.headline)
            if let a = ear.analysis {
                Picker("Capo", selection: $ear.capo) {
                    ForEach(a.capoOptionsByEase.prefix(4)) { o in
                        Text((o.capo == 0 ? "No capo" : "Capo \(o.capo)") + (o.hardSeconds == 0 ? ", every shape open" : String(format: ", %.0f s of barre chords", o.hardSeconds)) + (o.capo == a.guitar.capo ? "  (chordmap's pick)" : ""))
                            .tag(o.capo == a.guitar.capo ? Int?.none : Int?.some(o.capo))
                    }
                }.pickerStyle(.radioGroup).labelsHidden()
                let shapes = a.chordNames.filter { $0 != "N" }.map { "\($0) → \(ear.shown($0))" }.filter { !$0.hasSuffix("→ " + $0.split(separator: " ")[0]) }
                if ear.effectiveCapo != 0, !shapes.isEmpty { Text(Array(NSOrderedSet(array: shapes)).compactMap { $0 as? String }.prefix(8).joined(separator: "   ")).font(.callout.monospacedDigit()).foregroundStyle(.secondary) }
                Text("With a capo, every chord below is the shape your hand makes.").font(.caption).foregroundStyle(.secondary)
            } else { Text("The key, the tempo and the capo show up after a few bars.").font(.callout).foregroundStyle(.secondary) }
        }.padding(14).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }
    private var chartCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(ear.viewing != nil || ear.whole != nil ? "The chart" : "The last \(Int(EarModel.windowSeconds)) seconds, bar by bar").font(.headline)
            if let a = ear.analysis, !a.bars.isEmpty {
                ForEach(a.sections.isEmpty ? [Analysis.Section(start: 0, end: a.duration, label: "", guess: "", bar: 0)] : a.sections) { s in
                    let bars = a.bars.filter { $0.start >= s.start - 0.01 && $0.start < s.end - 0.01 }
                    if !bars.isEmpty {
                        VStack(alignment: .leading, spacing: 4) {
                            if !s.guess.isEmpty { Text(s.guess.capitalized + (s.label.isEmpty ? "" : " (\(s.label))")).font(.caption.bold()).foregroundStyle(.secondary) }
                            BarFlow(bars: bars.map { bar in Array(NSOrderedSet(array: bar.beats.map { ear.shown($0) })).compactMap { $0 as? String }.joined(separator: " · ") })
                        }
                    }
                }
                if a.harmonicity < 0.3 { Text("Sparse harmony in this stretch: trust the key and the tempo more than the chords.").font(.caption).foregroundStyle(.orange) }
            } else { Text("Nothing yet.").font(.callout).foregroundStyle(.secondary) }
        }.padding(14).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }
}

/// One chip per bar, wrapping.
struct BarFlow: View {
    let bars: [String]
    var body: some View {
        var width: CGFloat = 0, height: CGFloat = 0
        return GeometryReader { g in
            ZStack(alignment: .topLeading) {
                ForEach(Array(bars.enumerated()), id: \.offset) { i, n in
                    Text(n).font(.title3.bold()).lineLimit(1)
                        .padding(.horizontal, 10).padding(.vertical, 6).background(.background.opacity(0.6), in: RoundedRectangle(cornerRadius: 8))
                        .alignmentGuide(.leading) { d in
                            if abs(width - d.width) > g.size.width { width = 0; height -= d.height + 8 }
                            let r = width; if i == bars.count - 1 { width = 0 } else { width -= d.width + 8 }; return r
                        }
                        .alignmentGuide(.top) { _ in let r = height; if i == bars.count - 1 { height = 0 }; return r }
                }
            }
        }.frame(minHeight: CGFloat(max(1, (bars.count + 9) / 10)) * 44)
    }
}

struct SheetCard: View {
    let text: String
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack { Text("Chord sheet").font(.headline); Spacer(); Button("Copy") { NSPasteboard.general.clearContents(); NSPasteboard.general.setString(text, forType: .string) } }
            Text(text).font(.system(.body, design: .monospaced)).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
        }.padding(14).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }
}

struct ListenCard: View {
    @EnvironmentObject var ear: EarModel
    let listen: Listen
    @State private var title = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                TextField("Title", text: $title).textFieldStyle(.roundedBorder).font(.title3).frame(maxWidth: 360)
                    .onAppear { title = listen.title }.onSubmit { ear.rename(listen, to: title) }
                Spacer()
                Button("Back to live") { ear.viewing = nil; ear.whole = nil; ear.window = nil }
                Button(role: .destructive) { ear.delete(listen) } label: { Image(systemName: "trash") }
            }
            Text("\(listen.started.formatted(date: .abbreviated, time: .shortened)), \(Int(listen.seconds)) s, \(listen.analysis.key.name), \(Int(listen.analysis.tempo.bpm.rounded())) bpm").font(.callout).foregroundStyle(.secondary)
        }.padding(14).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }
}

struct HistorySidebar: View {
    @EnvironmentObject var ear: EarModel
    var body: some View {
        List(selection: Binding(get: { ear.viewing?.id }, set: { id in if let l = ear.listens.first(where: { $0.id == id }) { ear.show(l) } })) {
            Section("Heard before") {
                if ear.listens.isEmpty { Text("Nothing yet. Every listen lands here.").foregroundStyle(.secondary) }
                ForEach(ear.listens.reversed()) { l in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(l.title).lineLimit(1)
                        Text("\(l.analysis.key.name) · \(Int(l.analysis.tempo.bpm.rounded())) bpm · " + l.chordNames.prefix(4).joined(separator: " ")).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }.tag(l.id)
                }
            }
        }.navigationSplitViewColumnWidth(min: 190, ideal: 230, max: 300)
    }
}
