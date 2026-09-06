//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  Sound for the Ear: either the Mac's own output through a Core Audio process tap (macOS 14.2 or
//  later; the tap leaves out this app's own process, so the amp is not heard twice) or any input
//  device. Samples are kept in memory for the length of the listen, brought down to 24 kHz
//  (plenty for chords; chordmap works at 22 kHz inside). Nothing is written to disk.

import Foundation
import CoreAudio
import AudioToolbox

struct AppError: LocalizedError { let message: String; var errorDescription: String? { message } }

final class Capture: @unchecked Sendable {
    enum Source: Hashable, Codable {
        case mac
        case device(String)
        var label: String { if case .device(let uid) = self { return Devices.find(uid: uid)?.name ?? "Input" }; return "The Mac's own sound" }
    }
    static var macSoundAvailable: Bool { if #available(macOS 14.2, *) { return true } else { return false } }

    private(set) var running = false
    private(set) var sampleRate: Double = 48000        // after decimation
    private var deviceRate: Double = 48000
    private var factor = 1
    private var device: AudioDeviceID = 0
    private var proc: AudioDeviceIOProcID?
    private var tap: AudioObjectID = 0
    private var aggregate: AudioDeviceID = 0
    private var lock = os_unfair_lock()
    private var store: [Float] = []
    private var acc: Float = 0, accCount = 0
    private(set) var frames: UInt64 = 0
    /// Ten minutes at 24 kHz; after that the oldest minute is dropped.
    static let maxSamples = 24000 * 600

    func start(_ source: Source) throws {
        if running { stop() }
        switch source {
        case .device(let uid):
            guard let d = Devices.find(uid: uid), d.inputs > 0, d.id != 0 else { throw AppError(message: "That input is not here.") }
            device = d.id
        case .mac:
            device = try makeTap()
        }
        deviceRate = Devices.sampleRate(device); if deviceRate == 0 { deviceRate = 48000 }
        factor = max(1, Int((deviceRate / 24000).rounded(.up)))
        sampleRate = deviceRate / Double(factor)
        store = []; store.reserveCapacity(Int(sampleRate * 60)); acc = 0; accCount = 0; frames = 0
        var id: AudioDeviceIOProcID?
        let status = AudioDeviceCreateIOProcIDWithBlock(&id, device, nil) { [weak self] _, input, _, _, _ in self?.ingest(input) }
        guard status == noErr, let id else { tearDownTap(); throw AppError(message: "Could not open the sound for listening (CoreAudio error \(status)).") }
        proc = id
        let s = AudioDeviceStart(device, id)
        guard s == noErr else { stop(); throw AppError(message: "Could not start listening (CoreAudio error \(s)).") }
        running = true
    }

    func stop() {
        if let proc, device != 0 { AudioDeviceStop(device, proc); AudioDeviceDestroyIOProcID(device, proc) }
        proc = nil
        tearDownTap()
        device = 0; running = false
    }

    /// The most recent `seconds` of sound, oldest first.
    func latest(seconds: Double) -> [Float] {
        os_unfair_lock_lock(&lock); defer { os_unfair_lock_unlock(&lock) }
        let n = min(Int(seconds * sampleRate), store.count)
        return n > 0 ? Array(store.suffix(n)) : []
    }
    /// Everything heard since start.
    func all() -> [Float] { os_unfair_lock_lock(&lock); defer { os_unfair_lock_unlock(&lock) }; return store }
    var seconds: Double { os_unfair_lock_lock(&lock); defer { os_unfair_lock_unlock(&lock) }; return Double(store.count) / sampleRate }

    // MARK: Audio thread

    private func ingest(_ input: UnsafePointer<AudioBufferList>) {
        let list = UnsafeMutableAudioBufferListPointer(UnsafeMutablePointer(mutating: input))
        guard list.count > 0, let first = list[0].mData else { return }
        let channels = max(Int(list[0].mNumberChannels), 1)
        let n = Int(list[0].mDataByteSize) / 4 / channels
        let p = first.assumingMemoryBound(to: Float.self)
        os_unfair_lock_lock(&lock)
        for i in 0..<n {
            var m: Float = 0
            for c in 0..<channels { m += p[i * channels + c] }
            // Other buffers are extra streams (non-interleaved channels): fold them in too.
            for b in 1..<list.count { if let d = list[b].mData, i < Int(list[b].mDataByteSize) / 4 { m += d.assumingMemoryBound(to: Float.self)[i] } }
            acc += m / Float(channels); accCount += 1
            if accCount == factor {
                store.append(acc / Float(factor)); acc = 0; accCount = 0; frames &+= 1
                if store.count > Capture.maxSamples { store.removeFirst(Int(sampleRate * 60)) }
            }
        }
        os_unfair_lock_unlock(&lock)
    }

    // MARK: The tap

    private func makeTap() throws -> AudioDeviceID {
        guard #available(macOS 14.2, *) else { throw AppError(message: "Hearing the Mac's own sound needs macOS 14.2 or later. Pick an input instead.") }
        let desc = CATapDescription(stereoGlobalTapButExcludeProcesses: [Capture.selfProcessObject()])
        desc.uuid = UUID(); desc.name = "Chordmap for Mac Ear"; desc.isPrivate = true; desc.muteBehavior = CATapMuteBehavior.unmuted
        var tapID: AudioObjectID = 0
        let s = AudioHardwareCreateProcessTap(desc, &tapID)
        guard s == noErr, tapID != 0 else {
            throw AppError(message: "macOS did not let Chordmap for Mac hear the system sound (error \(s)). Allow it under System Settings › Privacy & Security › Screen & System Audio Recording, then try again.")
        }
        tap = tapID
        var addr = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyDefaultOutputDevice, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        var out: AudioDeviceID = 0; var size = UInt32(MemoryLayout<AudioDeviceID>.size)
        AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size, &out)
        guard let outUID = Devices.string(out, kAudioDevicePropertyDeviceUID) else { tearDownTap(); throw AppError(message: "No output device to listen to.") }
        let agg: [String: Any] = [
            kAudioAggregateDeviceNameKey: "Chordmap for Mac Ear",
            kAudioAggregateDeviceUIDKey: "com.keithadler.chordmac.ear.\(getpid())",
            kAudioAggregateDeviceIsPrivateKey: true,
            kAudioAggregateDeviceTapAutoStartKey: true,
            kAudioAggregateDeviceSubDeviceListKey: [[kAudioSubDeviceUIDKey: outUID]],
            kAudioAggregateDeviceTapListKey: [[kAudioSubTapDriftCompensationKey: true, kAudioSubTapUIDKey: desc.uuid.uuidString]],
        ]
        var aggID: AudioDeviceID = 0
        let a = AudioHardwareCreateAggregateDevice(agg as CFDictionary, &aggID)
        guard a == noErr, aggID != 0 else { tearDownTap(); throw AppError(message: "Could not attach to the system sound (CoreAudio error \(a)).") }
        aggregate = aggID
        return aggID
    }
    private func tearDownTap() {
        if aggregate != 0 { AudioHardwareDestroyAggregateDevice(aggregate); aggregate = 0 }
        if tap != 0 { if #available(macOS 14.2, *) { AudioHardwareDestroyProcessTap(tap) }; tap = 0 }
    }
    static func selfProcessObject() -> AudioObjectID {
        var pid = getpid(); var obj: AudioObjectID = 0; var size = UInt32(MemoryLayout<AudioObjectID>.size)
        var addr = AudioObjectPropertyAddress(mSelector: kAudioHardwarePropertyTranslatePIDToProcessObject, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
        AudioObjectGetPropertyData(AudioObjectID(kAudioObjectSystemObject), &addr, UInt32(MemoryLayout<pid_t>.size), &pid, &size, &obj)
        return obj
    }
}
