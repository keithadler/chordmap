import AppKit
// Chordmap for Mac icon: a fretboard with a chord shape on a teal tile.
func draw(_ s: CGFloat) -> NSImage {
    let img = NSImage(size: NSSize(width: s, height: s)); img.lockFocus()
    let inset = s * 0.06
    let tile = NSBezierPath(roundedRect: NSRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset), xRadius: s * 0.22, yRadius: s * 0.22)
    NSGradient(colors: [NSColor(calibratedRed: 0.12, green: 0.55, blue: 0.60, alpha: 1), NSColor(calibratedRed: 0.04, green: 0.16, blue: 0.22, alpha: 1)])!.draw(in: tile, angle: -70)
    // strings
    NSColor.white.withAlphaComponent(0.85).setStroke()
    for i in 0..<6 { let x = s * (0.28 + CGFloat(i) * 0.088); let l = NSBezierPath(); l.move(to: NSPoint(x: x, y: s * 0.18)); l.line(to: NSPoint(x: x, y: s * 0.80)); l.lineWidth = s * 0.014; l.stroke() }
    // frets and nut
    for i in 0..<5 { let y = s * (0.80 - CGFloat(i) * 0.155); let l = NSBezierPath(); l.move(to: NSPoint(x: s * 0.26, y: y)); l.line(to: NSPoint(x: s * 0.74, y: y)); l.lineWidth = i == 0 ? s * 0.035 : s * 0.014; l.stroke() }
    // fingers of a C shape
    NSColor(calibratedRed: 1.0, green: 0.62, blue: 0.18, alpha: 1).setFill()
    for (string, fret) in [(1, 3), (2, 2), (4, 1)] {
        let x = s * (0.28 + CGFloat(string) * 0.088), y = s * (0.80 - (CGFloat(fret) - 0.5) * 0.155)
        NSBezierPath(ovalIn: NSRect(x: x - s * 0.05, y: y - s * 0.05, width: s * 0.10, height: s * 0.10)).fill()
    }
    img.unlockFocus(); return img
}
let out = "icon/ChordMap.iconset"
try? FileManager.default.removeItem(atPath: out); try? FileManager.default.createDirectory(atPath: out, withIntermediateDirectories: true)
for (name, px) in [("16x16",16),("16x16@2x",32),("32x32",32),("32x32@2x",64),("128x128",128),("128x128@2x",256),("256x256",256),("256x256@2x",512),("512x512",512),("512x512@2x",1024)] {
    let img = draw(CGFloat(px))
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    img.draw(in: NSRect(x: 0, y: 0, width: px, height: px)); NSGraphicsContext.restoreGraphicsState()
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/icon_\(name).png"))
}
print("iconset written")
