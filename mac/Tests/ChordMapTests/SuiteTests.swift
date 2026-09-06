import XCTest
@testable import ChordMap

final class SuiteTests: XCTestCase {
    @MainActor private func runSuite(_ name: String) {
        let results = TestKit.run(filter: name + "/")
        XCTAssertFalse(results.isEmpty)
        for r in results { for f in r.failures { XCTFail("\(r.suite)/\(r.name): \(f)") } }
    }
    @MainActor func testChordmap() { runSuite("Chordmap") }
    @MainActor func testDevices() { runSuite("Devices") }
    @MainActor func testCLI() { runSuite("CLI") }
}
