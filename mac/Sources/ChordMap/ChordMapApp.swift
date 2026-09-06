//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  Play a song on the Mac, or into an interface, and see the key, the tempo, the chords bar by
//  bar and where the capo goes. chordmap does the hearing. Nothing is recorded, nothing leaves the Mac.

import SwiftUI
import AppKit
import ServiceManagement

@main
struct ChordMapApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var delegate
    @StateObject private var ear = EarModel.shared
    init() { CLI.runIfRequested() }
    var body: some Scene {
        WindowGroup("Chordmap for Mac") {
            MainView().environmentObject(ear).frame(minWidth: 860, idealWidth: 1000, minHeight: 600, idealHeight: 700)
        }
        .commands {
            CommandGroup(after: .toolbar) {
                Button(ear.listening ? "Stop Listening" : "Start Listening") { ear.toggle() }.keyboardShortcut("l", modifiers: .command)
                Button("Copy Chord Sheet") { if let a = ear.analysis { NSPasteboard.general.clearContents(); NSPasteboard.general.setString(Chordmap.chordSheet(a), forType: .string) } }
                    .keyboardShortcut("c", modifiers: [.command, .shift]).disabled(ear.analysis == nil)
            }
            CommandGroup(replacing: .help) {
                Button("Chordmap for Mac Help") { Help.open() }
                Button("Check for Updates…") { Updates.checkAndPresent() }
                Button("Report a Problem…") { NSWorkspace.shared.open(URL(string: "https://github.com/keithadler/chordmap/issues")!) }
                Divider()
                Button("More from the Same Maker…") { NSWorkspace.shared.open(URL(string: "https://keithadler.github.io")!) }
            }
        }
        Settings { SettingsView().environmentObject(ear) }
        MenuBarExtra(isInserted: .constant(Prefs.menuBar)) {
            Button(ear.listening ? "Stop Listening" : "Start Listening") { ear.toggle() }
            if let a = ear.analysis { Text("\(a.key.name), \(Int(a.tempo.bpm.rounded())) bpm" + (a.guitar.capo > 0 ? ", capo \(a.guitar.capo)" : "")) }
            if let c = ear.chord { Text("Now: \(ear.shown(c))") }
            Divider()
            Button("Open Chordmap for Mac") { NSApp.activate(ignoringOtherApps: true); NSApp.windows.first { $0.title == "Chordmap for Mac" }?.makeKeyAndOrderFront(nil) }
            Button("Quit") { NSApp.terminate(nil) }
        } label: { Image(systemName: "music.note.list") }
    }
}

struct MainView: View {
    @EnvironmentObject var ear: EarModel
    var body: some View {
        NavigationSplitView { HistorySidebar() } detail: { EarPanel() }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        if ProcessInfo.processInfo.environment["CHORDMAC_HOME"] == nil, !UserDefaults.standard.bool(forKey: "loginItemOffered") {
            UserDefaults.standard.set(true, forKey: "loginItemOffered"); try? SMAppService.mainApp.register()
        }
        Updates.scheduleBackgroundChecks()
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { !Prefs.menuBar && !EarModel.shared.listening }
    func applicationWillTerminate(_ notification: Notification) { EarModel.shared.capture.stop() }
}

enum Help {
    static var pageName: String { (Locale.preferredLanguages.first ?? "en").hasPrefix("es") ? "Help.es" : "Help" }
    static var bundledPage: URL? {
        if let url = Bundle.main.url(forResource: pageName, withExtension: "html") { return url }
        var url = (Bundle.main.executableURL ?? URL(fileURLWithPath: CommandLine.arguments[0])).resolvingSymlinksInPath()
        while url.path != "/" { if url.pathExtension == "app", let u = Bundle(url: url)?.url(forResource: pageName, withExtension: "html") { return u }; url = url.deletingLastPathComponent() }
        return nil
    }
    @MainActor static func open() { NSWorkspace.shared.open(bundledPage ?? URL(string: "https://github.com/keithadler/chordmap/tree/main/mac#readme")!) }
}

struct SettingsView: View {
    @EnvironmentObject var ear: EarModel
    @State private var menuBar = Prefs.menuBar
    @State private var login = SMAppService.mainApp.status == .enabled
    @State private var updates = Updates.enabled
    var body: some View {
        Form {
            Section("Listening") {
                Text("Hearing the Mac's own sound needs macOS 14.2 or later and one permission, System Audio Recording, asked for once. An input device needs the Microphone permission. Neither records anything.").font(.callout).foregroundStyle(.secondary)
                HStack {
                    Text("Engine: chordmap \(Chordmap.version)")
                    Spacer()
                    Button("Open Privacy Settings") { NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture")!) }
                }
            }
            Section("App") {
                Toggle("Open at login", isOn: $login).onChange(of: login) { _, v in if v { try? SMAppService.mainApp.register() } else { try? SMAppService.mainApp.unregister() } }
                Toggle("Show in the menu bar", isOn: $menuBar).onChange(of: menuBar) { _, v in Prefs.menuBar = v }
                Toggle("Check for updates daily", isOn: $updates).onChange(of: updates) { _, v in Updates.enabled = v }
            }
        }.formStyle(.grouped).frame(width: 520).padding(.bottom, 8)
    }
}
