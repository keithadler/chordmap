//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  The audio devices macOS knows about, read through CoreAudio. `Devices.override` lets the tests
//  and the screenshots run on a made-up list and never touch real hardware.

import Foundation
import CoreAudio

struct AudioDevice: Identifiable, Equatable, Codable {
    var id: UInt32              // AudioDeviceID; 0 in fixtures
    var uid: String
    var name: String
    var manufacturer: String
    var inputs: Int
    var outputs: Int
    var sampleRate: Double
    var bufferFrames: Int
    var inLatency: Int          // frames, device latency + safety offset on the input side
    var outLatency: Int
    var isInterface: Bool { inputs > 0 && outputs > 0 && !manufacturer.hasPrefix("Apple") }
    /// Zero-based channel to listen on. A Scarlett Solo's instrument jack is input 2.
    var defaultInputChannel: Int { name.localizedCaseInsensitiveContains("Solo") && inputs >= 2 ? 1 : 0 }
    func channelLabel(_ i: Int) -> String {
        if name.localizedCaseInsensitiveContains("Solo") { return i == 0 ? "Input 1 (XLR)" : "Input 2 (instrument jack)" }
        if name.localizedCaseInsensitiveContains("2i2") { return "Input \(i + 1) (combo jack)" }
        return inputs == 1 ? name : "Input \(i + 1)"
    }
}

enum Devices {
    /// Fixtures for tests and screenshots; nil reads CoreAudio.
    nonisolated(unsafe) static var override: [AudioDevice]?

    static let demo: [AudioDevice] = [
        AudioDevice(id: 0, uid: "demo-solo", name: "Scarlett Solo USB", manufacturer: "Focusrite", inputs: 2, outputs: 2, sampleRate: 48000, bufferFrames: 128, inLatency: 41, outLatency: 43),
        AudioDevice(id: 0, uid: "demo-mic", name: "MacBook Pro Microphone", manufacturer: "Apple Inc.", inputs: 1, outputs: 0, sampleRate: 48000, bufferFrames: 512, inLatency: 200, outLatency: 0),
        AudioDevice(id: 0, uid: "demo-spk", name: "MacBook Pro Speakers", manufacturer: "Apple Inc.", inputs: 0, outputs: 2, sampleRate: 48000, bufferFrames: 512, inLatency: 0, outLatency: 220),
    ]

    static func all() -> [AudioDevice] {
        if let override { return override }
        var addr = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDevices, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var size: UInt32 = 0
        guard AudioObjectGetPropertyDataSize(AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size) == noErr, size > 0 else { return [] }
        var ids = [AudioDeviceID](repeating: 0, count: Int(size) / MemoryLayout<AudioDeviceID>.size)
        guard AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size, &ids) == noErr else { return [] }
        return ids.compactMap(describe).filter { !$0.uid.hasPrefix("com.keithadler.chordmac") }
    }
    static func inputs() -> [AudioDevice] { all().filter { $0.inputs > 0 } }
    static func outputs() -> [AudioDevice] { all().filter { $0.outputs > 0 } }
    static func find(uid: String?) -> AudioDevice? { guard let uid else { return nil }; return all().first { $0.uid == uid } }
    static func find(nameContains s: String) -> AudioDevice? { all().first { $0.name.localizedCaseInsensitiveContains(s) } }

    /// What to use when nothing is saved: an interface with both sides, else the system defaults.
    static func defaultChoice(_ list: [AudioDevice]) -> (input: AudioDevice, output: AudioDevice)? {
        if let box = list.first(where: { $0.isInterface }) { return (box, box) }
        guard let i = list.first(where: { $0.inputs > 0 }), let o = list.first(where: { $0.outputs > 0 }) else { return nil }
        return (i, o)
    }

    /// Round trip, in milliseconds: two buffers plus what each side reports.
    static func roundTripMs(input: AudioDevice, output: AudioDevice, bufferFrames: Int) -> Double {
        let sr = max(input.sampleRate, 1)
        return Double(2 * bufferFrames + input.inLatency + output.outLatency) / sr * 1000
    }

    // MARK: CoreAudio reads

    static func string(_ id: AudioObjectID, _ sel: AudioObjectPropertySelector) -> String? {
        var addr = AudioObjectPropertyAddress(mSelector: sel, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var size = UInt32(MemoryLayout<CFString?>.size); var value: Unmanaged<CFString>?
        guard AudioObjectGetPropertyData(id, &addr, 0, nil, &size, &value) == noErr, let v = value?.takeRetainedValue() else { return nil }
        return v as String
    }
    static func uint(_ id: AudioObjectID, _ sel: AudioObjectPropertySelector, scope: AudioObjectPropertyScope) -> UInt32 {
        var addr = AudioObjectPropertyAddress(mSelector: sel, mScope: scope, mElement: kAudioObjectPropertyElementMain)
        var size = UInt32(MemoryLayout<UInt32>.size); var v: UInt32 = 0
        return AudioObjectGetPropertyData(id, &addr, 0, nil, &size, &v) == noErr ? v : 0
    }
    static func channels(_ id: AudioObjectID, scope: AudioObjectPropertyScope) -> Int {
        var addr = AudioObjectPropertyAddress(mSelector: kAudioDevicePropertyStreamConfiguration, mScope: scope, mElement: kAudioObjectPropertyElementMain)
        var size: UInt32 = 0
        guard AudioObjectGetPropertyDataSize(id, &addr, 0, nil, &size) == noErr, size > 0 else { return 0 }
        let raw = UnsafeMutableRawPointer.allocate(byteCount: Int(size), alignment: MemoryLayout<AudioBufferList>.alignment); defer { raw.deallocate() }
        guard AudioObjectGetPropertyData(id, &addr, 0, nil, &size, raw) == noErr else { return 0 }
        let list = UnsafeMutableAudioBufferListPointer(raw.assumingMemoryBound(to: AudioBufferList.self))
        return list.reduce(0) { $0 + Int($1.mNumberChannels) }
    }
    static func sampleRate(_ id: AudioObjectID) -> Double {
        var addr = AudioObjectPropertyAddress(mSelector: kAudioDevicePropertyNominalSampleRate, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var size = UInt32(MemoryLayout<Double>.size); var v: Double = 0
        return AudioObjectGetPropertyData(id, &addr, 0, nil, &size, &v) == noErr ? v : 0
    }
    static func describe(_ id: AudioDeviceID) -> AudioDevice? {
        guard let uid = string(id, kAudioDevicePropertyDeviceUID), let name = string(id, kAudioObjectPropertyName) else { return nil }
        let ins = channels(id, scope: kAudioObjectPropertyScopeInput), outs = channels(id, scope: kAudioObjectPropertyScopeOutput)
        guard ins > 0 || outs > 0 else { return nil }
        let inLat = Int(uint(id, kAudioDevicePropertyLatency, scope: kAudioObjectPropertyScopeInput) + uint(id, kAudioDevicePropertySafetyOffset, scope: kAudioObjectPropertyScopeInput))
        let outLat = Int(uint(id, kAudioDevicePropertyLatency, scope: kAudioObjectPropertyScopeOutput) + uint(id, kAudioDevicePropertySafetyOffset, scope: kAudioObjectPropertyScopeOutput))
        return AudioDevice(id: id, uid: uid, name: name, manufacturer: string(id, kAudioObjectPropertyManufacturer) ?? "", inputs: ins, outputs: outs,
                           sampleRate: sampleRate(id), bufferFrames: Int(uint(id, kAudioDevicePropertyBufferFrameSize, scope: kAudioObjectPropertyScopeGlobal)), inLatency: inLat, outLatency: outLat)
    }

    /// Called on the main queue whenever a device is plugged or unplugged.
    nonisolated(unsafe) static var onChange: (@Sendable () -> Void)?
    private nonisolated(unsafe) static var listening = false
    static func listen() {
        guard !listening else { return }; listening = true
        var addr = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDevices, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        AudioObjectAddPropertyListenerBlock(AudioObjectID(kAudioObjectSystemObject), &addr, .main) { _, _ in Devices.onChange?() }
    }
}
