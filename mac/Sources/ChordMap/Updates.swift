//  Chordmap for Mac — MIT licensed. See LICENSE.
//
//  Updates without Sparkle: one GET to GitHub's releases API, compare versions, open the release
//  page. Manual from the app menu, or an opt-in daily check (Settings) that shows an alert once per
//  new version. Nothing is downloaded or installed by itself and no identifiers are sent.

import Foundation
import AppKit

enum Updates {
    static let repo = "keithadler/chordmac"
    static let releasesPage = URL(string: "https://github.com/\(repo)/releases/latest")!
    static let interval: TimeInterval = 24 * 3600
    nonisolated(unsafe) static var session: URLSession = .shared
    private static let defaults = UserDefaults.standard

    enum Result: Equatable { case upToDate(String), available(String, URL), unknown(String) }

    static var enabled: Bool { get { defaults.object(forKey: "autoUpdateCheck") as? Bool ?? true } set { defaults.set(newValue, forKey: "autoUpdateCheck") } }
    static var lastCheck: Date? {
        get { let t = defaults.double(forKey: "lastUpdateCheck"); return t > 0 ? Date(timeIntervalSince1970: t) : nil }
        set { defaults.set(newValue?.timeIntervalSince1970 ?? 0, forKey: "lastUpdateCheck") }
    }
    static var skippedVersion: String? {
        get { defaults.string(forKey: "skippedVersion") }
        set { defaults.set(newValue, forKey: "skippedVersion") }
    }

    static func check() async -> Result {
        var req = URLRequest(url: URL(string: "https://api.github.com/repos/\(repo)/releases/latest")!)
        req.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        req.timeoutInterval = 15
        do {
            let (data, resp) = try await session.data(for: req)
            guard let http = resp as? HTTPURLResponse else { return .unknown("no response") }
            return parse(status: http.statusCode, body: data, current: CLI.version)
        } catch { return .unknown(error.localizedDescription) }
    }

    static func parse(status: Int, body: Data, current: String) -> Result {
        if status == 404 { return .unknown(String(localized: "No public release yet.")) }
        guard status == 200, let json = try? JSONSerialization.jsonObject(with: body) as? [String: Any],
              let tag = json["tag_name"] as? String else { return .unknown("HTTP \(status)") }
        let latest = tag.hasPrefix("v") ? String(tag.dropFirst()) : tag
        let page = (json["html_url"] as? String).flatMap(URL.init(string:)) ?? releasesPage
        return isNewer(latest, than: current) ? .available(latest, page) : .upToDate(latest)
    }

    static func isNewer(_ a: String, than b: String) -> Bool {
        let pa = a.split(separator: ".").map { Int($0) ?? 0 }, pb = b.split(separator: ".").map { Int($0) ?? 0 }
        for i in 0..<max(pa.count, pb.count) {
            let x = i < pa.count ? pa[i] : 0, y = i < pb.count ? pb[i] : 0
            if x != y { return x > y }
        }
        return false
    }

    static func shouldCheck(enabled: Bool, last: Date?, now: Date = Date()) -> Bool {
        guard enabled else { return false }
        guard let last else { return true }
        return now.timeIntervalSince(last) >= interval - 3600
    }

    private nonisolated(unsafe) static var timer: Timer?

    /// Called at launch: checks shortly after start when due, then re-evaluates hourly.
    @MainActor
    static func scheduleBackgroundChecks() {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 3600, repeats: true) { _ in Task { @MainActor in await checkIfDue() } }
        DispatchQueue.main.asyncAfter(deadline: .now() + 20) { Task { @MainActor in await checkIfDue() } }
    }

    @MainActor
    static func checkIfDue() async {
        guard shouldCheck(enabled: enabled, last: lastCheck) else { return }
        let r = await check()
        lastCheck = Date()
        if case .available(let v, let url) = r, skippedVersion != v { present(version: v, page: url) }
    }

    @MainActor
    static func checkAndPresent() {
        Task {
            let r = await check()
            lastCheck = Date()
            let alert = NSAlert()
            switch r {
            case .available(let v, let url): present(version: v, page: url); return
            case .upToDate:
                alert.messageText = String(localized: "You're up to date")
                alert.informativeText = String(format: String(localized: "Chordmap for Mac %@ is the latest version."), CLI.version)
            case .unknown(let why):
                alert.messageText = String(localized: "Couldn't check for updates")
                alert.informativeText = why
            }
            NSApp.activate()
            alert.runModal()
        }
    }

    @MainActor
    private static func present(version v: String, page: URL) {
        let alert = NSAlert()
        alert.messageText = String(format: String(localized: "Chordmap for Mac %@ is available"), v)
        alert.informativeText = String(format: String(localized: "You have %@. The release page has the download and what changed. Install it the same way as the first time."), CLI.version)
        alert.addButton(withTitle: String(localized: "Open Release Page"))
        alert.addButton(withTitle: String(localized: "Later"))
        alert.addButton(withTitle: String(localized: "Skip This Version"))
        NSApp.activate()
        switch alert.runModal() {
        case .alertFirstButtonReturn: NSWorkspace.shared.open(page)
        case .alertThirdButtonReturn: skippedVersion = v
        default: break
        }
    }
}
