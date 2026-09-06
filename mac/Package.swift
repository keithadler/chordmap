// swift-tools-version: 5.9
import PackageDescription
import Foundation

// The chordmap bridge (ffi/) is a Rust static library; ffi/build.sh writes ffi/lib/libchordmap_ffi.a.
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "ChordMap",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(name: "CChordmap", path: "ffi/include"),
        .executableTarget(name: "ChordMap", dependencies: ["CChordmap"], path: "Sources/ChordMap",
                          linkerSettings: [.unsafeFlags(["-L", "\(root)/ffi/lib"])]),
        .testTarget(name: "ChordMapTests", dependencies: ["ChordMap"], path: "Tests/ChordMapTests"),
    ]
)
