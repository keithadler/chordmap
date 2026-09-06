//  Chordmap for Mac — MIT licensed. See LICENSE.
//  Promo cards for the announcement, 1600×900 at 2×.

import AppKit
import SwiftUI

enum Promo {
    @MainActor static func render(to dir: URL, screenshots: URL) throws -> [URL] {
        let out = dir.appendingPathComponent("promo"); try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
        var written: [URL] = []
        let cards: [(String, String, String, String?)] = [
            ("1-hero", "Play any song on the Mac.\nSee the chords.", "Key, tempo, the chords bar by bar, the sections, and where the capo goes so the open shapes play it. Live, while it plays.", "live.png"),
            ("2-card", "Eb major.\nCapo 3, play in C.", "chordmap scores every fret by how many seconds land on barre chords and picks the one with the least. Choose another and every chord turns into the shape your hand makes.", "live.png"),
            ("3-honest", "It hears a mix.\nIt records nothing.", "Not a song database: it does not know the title, only what it hears. A clear mix charts well, a wall of distortion less well, and sparse harmony is flagged, not invented.", "listen.png"),
            ("4-free", "Free. Open source. Works on a file too.", "chordmac song.wav --capo 3   Every listen kept, with the chord sheet to copy. Rust engine, MIT licensed, no account, nothing uploaded.", nil),
        ]
        for (name, title, sub, shot) in cards {
            let view = Card(title: title, subtitle: sub, image: shot.flatMap { NSImage(contentsOf: screenshots.appendingPathComponent($0)) })
            let host = NSHostingView(rootView: view); host.frame = NSRect(x: 0, y: 0, width: 1600, height: 900)
            let w = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false); w.contentView = host; w.orderFront(nil)
            Screenshots.settle()
            written.append(try Screenshots.capture(w, to: out.appendingPathComponent("\(name).png"))); w.orderOut(nil)
        }
        return written
    }
    struct Card: View {
        let title: String, subtitle: String, image: NSImage?
        var body: some View {
            ZStack {
                LinearGradient(colors: [Color(red: 0.05, green: 0.12, blue: 0.16), Color(red: 0.10, green: 0.45, blue: 0.50)], startPoint: .topLeading, endPoint: .bottomTrailing)
                HStack(spacing: 40) {
                    VStack(alignment: .leading, spacing: 22) {
                        HStack(spacing: 12) { Image(systemName: "music.note.list").font(.system(size: 34)); Text("Chordmap for Mac").font(.system(size: 30, weight: .semibold)) }.foregroundStyle(.white.opacity(0.85))
                        Text(title).font(.system(size: image == nil ? 60 : 48, weight: .bold, design: .rounded)).foregroundStyle(.white).fixedSize(horizontal: false, vertical: true)
                        Text(subtitle).font(.system(size: 26)).foregroundStyle(.white.opacity(0.8)).fixedSize(horizontal: false, vertical: true)
                    }.frame(width: image == nil ? 1300 : 620, alignment: .leading)
                    if let image { Image(nsImage: image).resizable().aspectRatio(contentMode: .fit).frame(width: 820).clipShape(RoundedRectangle(cornerRadius: 14)).shadow(radius: 30, y: 12) }
                }.padding(80)
            }.frame(width: 1600, height: 900)
        }
    }
}
