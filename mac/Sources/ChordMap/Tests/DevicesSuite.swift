//  Chordmap for Mac — MIT licensed. See LICENSE.

import Foundation

enum DevicesSuite {
    static var suite: TestSuite { TestSuite(name: "Devices", cases: [
        TestCase(name: "the interface is chosen for both sides") { t in
            Devices.override = Devices.demo; defer { Devices.override = nil }
            let d = Devices.defaultChoice(Devices.all())
            t.equal(d?.input.name, "Scarlett Solo USB", "input")
            t.equal(d?.output.name, "Scarlett Solo USB", "output")
            t.equal(Devices.inputs().count, 2, "two inputs"); t.equal(Devices.outputs().count, 2, "two outputs")
            t.equal(Devices.find(nameContains: "scarlett")?.uid, "demo-solo", "find by part of a name")
        },
        TestCase(name: "without an interface the Mac's own input and output are used") { t in
            Devices.override = Array(Devices.demo.dropFirst()); defer { Devices.override = nil }
            let d = Devices.defaultChoice(Devices.all())
            t.equal(d?.input.name, "MacBook Pro Microphone", "input")
            t.equal(d?.output.name, "MacBook Pro Speakers", "output")
            Devices.override = []
            t.check(Devices.defaultChoice(Devices.all()) == nil, "nothing gives nil")
        },
        TestCase(name: "a Solo listens on the instrument jack by default") { t in
            let solo = Devices.demo[0]
            t.equal(solo.defaultInputChannel, 1, "input 2")
            t.equal(solo.channelLabel(1), "Input 2 (instrument jack)", "label")
            t.equal(solo.channelLabel(0), "Input 1 (XLR)", "label")
            t.equal(Devices.demo[1].defaultInputChannel, 0, "mic is channel 1")
        },
    ]) }
}
