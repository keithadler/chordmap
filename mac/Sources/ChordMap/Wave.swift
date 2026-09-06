//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  Reading and writing audio files for the offline `render` and `tone` commands, through AVAudioFile.

import Foundation
import AVFoundation

enum Wave {
    /// First channel only, as Float, plus the file's sample rate.
    static func read(_ url: URL) throws -> (samples: [Float], sampleRate: Double) {
        let file = try AVAudioFile(forReading: url)
        let fmt = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: file.fileFormat.sampleRate, channels: file.fileFormat.channelCount, interleaved: false)!
        guard let buf = AVAudioPCMBuffer(pcmFormat: fmt, frameCapacity: AVAudioFrameCount(file.length)) else { throw AppError(message: "Could not read \(url.lastPathComponent).") }
        try file.read(into: buf)
        let n = Int(buf.frameLength)
        guard let ch = buf.floatChannelData else { return ([], file.fileFormat.sampleRate) }
        var out = [Float](repeating: 0, count: n)
        // Mono mix of every channel, so a stereo overhead pair still works.
        let chs = Int(fmt.channelCount)
        for c in 0..<chs { for i in 0..<n { out[i] += ch[c][i] / Float(chs) } }
        return (out, file.fileFormat.sampleRate)
    }

    /// Stereo 24-bit WAV (or whatever the extension says: .aiff, .caf, .m4a are accepted by AVAudioFile).
    static func write(_ url: URL, left: [Float], right: [Float], sampleRate: Double) throws {
        let fmt = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: sampleRate, channels: 2, interleaved: false)!
        let settings: [String: Any] = [AVFormatIDKey: kAudioFormatLinearPCM, AVSampleRateKey: sampleRate, AVNumberOfChannelsKey: 2, AVLinearPCMBitDepthKey: 24, AVLinearPCMIsFloatKey: false, AVLinearPCMIsNonInterleaved: false]
        let file = try AVAudioFile(forWriting: url, settings: url.pathExtension.lowercased() == "m4a" ? [AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: sampleRate, AVNumberOfChannelsKey: 2] : settings)
        let n = min(left.count, right.count)
        guard let buf = AVAudioPCMBuffer(pcmFormat: fmt, frameCapacity: AVAudioFrameCount(n)) else { throw AppError(message: "Could not make a buffer.") }
        buf.frameLength = AVAudioFrameCount(n)
        left.withUnsafeBufferPointer { buf.floatChannelData![0].update(from: $0.baseAddress!, count: n) }
        right.withUnsafeBufferPointer { buf.floatChannelData![1].update(from: $0.baseAddress!, count: n) }
        try file.write(from: buf)
    }
}
