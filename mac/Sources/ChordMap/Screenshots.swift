//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  `chordmac screenshots <dir>`: the window mid-song on chordmap's own synthesised strums. No device is opened.

import AppKit
import SwiftUI

enum Screenshots {
    @MainActor
    static func render(to dir: URL, announce: Bool) throws -> [URL] {
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let app = NSApplication.shared
        app.setActivationPolicy(.regular); app.activate(ignoringOtherApps: true)
        if app.applicationIconImage.size.width == 0 || Bundle.main.bundleIdentifier == nil, let icon = NSImage(contentsOfFile: FileManager.default.currentDirectoryPath + "/AppIcon.icns") { app.applicationIconImage = icon }
        Prefs.defaults = UserDefaults(suiteName: "com.keithadler.chordmac.screenshots")!
        Prefs.defaults.removePersistentDomain(forName: "com.keithadler.chordmac.screenshots")
        Devices.override = Devices.demo
        History.overrideDir = TestKit.tempDir()
        defer { Prefs.defaults = .standard; Devices.override = nil; History.overrideDir = nil }
        let ear = EarModel.shared; ear.demo = true
        let (x, xr) = Chordmap.synth("Eb Ab Bb Cm", bpm: 96, loops: 2)
        let live = try? Chordmap.analyze(x, sampleRate: xr)
        let (g, gr) = Chordmap.synth("G C D G", bpm: 120, loops: 1), (m, mr) = Chordmap.synth("Am F C G", bpm: 84, loops: 1)
        if let ga = try? Chordmap.analyze(g, sampleRate: gr), let ma = try? Chordmap.analyze(m, sampleRate: mr) {
            ear.listens = [Listen(started: Date().addingTimeInterval(-86400 * 2), title: "Sam's song, Tuesday", seconds: 212, analysis: ga),
                           Listen(started: Date().addingTimeInterval(-3600), title: "Radio, this morning", seconds: 95, analysis: ma)]
        }
        var written: [URL] = []
        for (suffix, appearance) in [("", NSAppearance.Name.darkAqua), ("-light", .aqua)] {
            app.appearance = NSAppearance(named: appearance)
            for (name, viewing) in [("live", false), ("listen", true)] {
                if viewing { ear.viewing = ear.listens.last; ear.whole = nil; ear.window = nil; ear.listening = false; ear.chord = nil }
                else { ear.viewing = nil; ear.whole = nil; ear.window = live; ear.listening = true; ear.capo = nil; ear.chord = "Bb"; ear.elapsed = 38 }
                let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1000, height: 700), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
                w.title = "Chordmap for Mac"
                w.contentView = NSHostingView(rootView: MainView().environmentObject(ear).frame(width: 1000, height: 700))
                w.center(); w.makeKeyAndOrderFront(nil)
                settle(); written.append(try capture(w, to: dir.appendingPathComponent("\(name)\(suffix).png"))); w.orderOut(nil)
            }
        }
        ear.listening = false; ear.demo = false; ear.window = nil; ear.chord = nil; ear.viewing = nil; ear.listens = History.load()
        if announce { written += try Promo.render(to: dir, screenshots: dir) }
        return written
    }
    @MainActor static func settle() { let until = Date().addingTimeInterval(0.8); while Date() < until { RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02)) } }
    @MainActor static func image(of window: NSWindow, retina: Bool = false) throws -> CGImage {
        typealias Fn = @convention(c) (CGRect, UInt32, UInt32, UInt32) -> Unmanaged<CGImage>?
        guard let sym = dlsym(dlopen(nil, RTLD_NOW), "CGWindowListCreateImage") else { throw NSError(domain: "shots", code: 1) }
        let fn = unsafeBitCast(sym, to: Fn.self)
        guard let i = fn(.null, 1 << 3, UInt32(window.windowNumber), 1 << 0 | (retina ? 1 << 3 : 1 << 4))?.takeRetainedValue() else { throw NSError(domain: "shots", code: 2) }
        return i
    }
    @MainActor static func capture(_ window: NSWindow, to url: URL, retina: Bool = false) throws -> URL {
        let img = try image(of: window, retina: retina)
        guard let png = NSBitmapImageRep(cgImage: img).representation(using: .png, properties: [:]) else { throw NSError(domain: "shots", code: 3) }
        try png.write(to: url); return url
    }
}
