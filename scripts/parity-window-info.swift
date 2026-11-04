import CoreGraphics
import Foundation
import AppKit
let activePID = NSWorkspace.shared.frontmostApplication.map { Int($0.processIdentifier) } ?? -1
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String:Any]] ?? []
for window in windows {
    let owner = window[kCGWindowOwnerName as String] as? String ?? ""
    let title = window[kCGWindowName as String] as? String ?? ""
    let layer = window[kCGWindowLayer as String] as? Int ?? -1
    if (layer == 0 || CommandLine.arguments.contains("--all-layers")) && (owner == "Vela Native" || owner == "Vela Native QA" || owner == "Vela Reference QA" || owner == "Vela" || (owner == "Electron" && title == "Vela")) {
        let pid = window[kCGWindowOwnerPID as String] as? Int ?? 0
        let item:[String:Any] = ["id":window[kCGWindowNumber as String] ?? 0,"pid":pid,"owner":owner,"title":title,"layer":layer,"active":activePID == pid,"bounds":window[kCGWindowBounds as String] ?? [:]]
        print(String(data:try! JSONSerialization.data(withJSONObject:item,options:[.sortedKeys]),encoding:.utf8)!)
    }
}
