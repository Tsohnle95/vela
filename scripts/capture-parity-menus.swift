// Capture only Vela menu windows. The bounded watcher keeps capture permissions
// outside the menu's lifetime, since a permission prompt dismisses AppKit menus.
import CoreGraphics
import Foundation
guard (4...5).contains(CommandLine.arguments.count),
      let referencePID = Int(CommandLine.arguments[2]),
      let nativePID = Int(CommandLine.arguments[3]) else {
    print("usage: capture-parity-menus.swift evidence-directory reference-pid native-qa-pid [capture-name]")
    exit(2)
}
let destination = CommandLine.arguments[1]
let name = CommandLine.arguments.count == 5 ? CommandLine.arguments[4] : "providers-fixture-system-menu"
var captured = Set<String>()
let end = Date().addingTimeInterval(120)
while Date() < end && captured.count < 2 {
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String:Any]] ?? []
    for window in windows {
        let owner = window[kCGWindowOwnerName as String] as? String ?? ""
        let layer = window[kCGWindowLayer as String] as? Int ?? 0
        let pid = window[kCGWindowOwnerPID as String] as? Int ?? 0
        let expected = (owner == "Vela Native QA" && pid == nativePID) || (owner == "Vela" && pid == referencePID)
        guard layer == 101, expected, !captured.contains(owner) else { continue }
        let id = window[kCGWindowNumber as String] as? Int ?? 0
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/bash")
        process.arguments = ["scripts/capture-parity.sh", owner == "Vela Native QA" ? "native" : "original", name, destination, String(id)]
        try process.run()
        process.waitUntilExit()
        if process.terminationStatus == 0 { captured.insert(owner) }
    }
    Thread.sleep(forTimeInterval: 0.25)
}
if captured.count != 2 { print("Captured \(captured.count) of 2 menu windows; no match is claimed.") }
