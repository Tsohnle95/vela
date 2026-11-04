// QA-only Accessibility input. No screen capture or recording session.
// Usage: swift parity-accessibility.swift native|original state
//        ... native|original press|focus|click PATH [EXPECTED_LABEL]
//        ... native|original value PATH TEXT
//        ... native|original key cmd+o
//        ... native|original bounds X Y WIDTH HEIGHT
// PATH comes from a fresh state response, e.g. window/0/3 or menu/2/0/1.
// Never launches an app, changes permissions, or targets a production bundle.
import AppKit
import ApplicationServices
import Foundation

enum Failure: Error, CustomStringConvertible {
    case message(String)
    var description: String { switch self { case .message(let text): return text } }
}
func fail(_ text: String) throws -> Never { throw Failure.message(text) }
func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}
func children(_ element: AXUIElement) -> [AXUIElement] {
    attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
}
func string(_ element: AXUIElement, _ name: String) -> String? {
    attribute(element, name) as? String
}
func check(_ result: AXError, _ operation: String) throws {
    if result != .success { try fail("\(operation) failed: AX error \(result.rawValue)") }
}
func output(_ value: Any) throws {
    let data = try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys])
    print(String(decoding: data, as: UTF8.self))
}
func position(_ element: AXUIElement) -> [Double]? {
    guard let raw = attribute(element, kAXPositionAttribute), CFGetTypeID(raw) == AXValueGetTypeID() else { return nil }
    let value = unsafeBitCast(raw, to: AXValue.self)
    var point = CGPoint.zero
    guard AXValueGetValue(value, .cgPoint, &point) else { return nil }
    return [point.x, point.y]
}
func dimensions(_ element: AXUIElement) -> [Double]? {
    guard let raw = attribute(element, kAXSizeAttribute), CFGetTypeID(raw) == AXValueGetTypeID() else { return nil }
    let value = unsafeBitCast(raw, to: AXValue.self)
    var size = CGSize.zero
    guard AXValueGetValue(value, .cgSize, &size) else { return nil }
    return [size.width, size.height]
}

func main() throws {
    let args = Array(CommandLine.arguments.dropFirst())
    guard args.count >= 2 else { try fail("Expected native|original and a command") }
    let bundle: String
    switch args[0] {
    case "native": bundle = "dev.vela.native.parity"
    case "original": bundle = "dev.vela.reference.parity"
    default: try fail("Only the native and original isolated QA bundles are allowed")
    }
    guard AXIsProcessTrusted() else {
        try fail("Accessibility access is unavailable. No permission prompt was opened.")
    }
    let matches = NSRunningApplication.runningApplications(withBundleIdentifier: bundle)
    guard matches.count == 1, let running = matches.first else {
        try fail("Expected exactly one already-running \(bundle) process; no app was launched")
    }
    let app = AXUIElementCreateApplication(running.processIdentifier)
    let windows = attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
    let documents = windows.filter { string($0, kAXTitleAttribute) == "Vela" }
    guard documents.count == 1, let window = documents.first else {
        try fail("Expected exactly one Vela document window in the QA process")
    }
    var roots = ["window": window]
    if let menu = attribute(app, kAXMenuBarAttribute), CFGetTypeID(menu) == AXUIElementGetTypeID() {
        roots["menu"] = unsafeBitCast(menu, to: AXUIElement.self)
    }
    func resolve(_ path: String) throws -> AXUIElement {
        let parts = path.split(separator: "/").map(String.init)
        guard let root = parts.first, var current = roots[root] else { try fail("Unknown QA root") }
        if root == "menu", parts.dropFirst().first == "0" { try fail("The system Apple menu is outside QA scope") }
        for part in parts.dropFirst() {
            guard let index = Int(part), index >= 0 else { try fail("Invalid AX path") }
            let nodes = children(current)
            guard nodes.indices.contains(index) else { try fail("Stale AX path; request a fresh state") }
            current = nodes[index]
        }
        return current
    }
    func activate() throws {
        // Keyboard events are permitted only after the intended QA app is frontmost.
        if AXUIElementSetAttributeValue(app, kAXFrontmostAttribute as CFString, kCFBooleanTrue) != .success {
            running.activate(options: [])
        }
        for _ in 0..<20 {
            if NSWorkspace.shared.frontmostApplication?.processIdentifier == running.processIdentifier { return }
            RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.05))
        }
        try fail("QA app did not activate; no keyboard events were sent")
    }
    switch args[1] {
    case "state", "brief":
        guard args.count <= 4 else { try fail("Expected state|brief [window|menu] [label substring]") }
        let requestedRoot = args.count >= 3 ? args[2] : "window"
        guard roots[requestedRoot] != nil else { try fail("Unknown QA state root") }
        var nodes: [[String: Any]] = []
        func walk(_ element: AXUIElement, _ path: String, _ depth: Int) throws {
            guard depth < 32, nodes.count < 6000 else { try fail("AX tree exceeds the inspection limit") }
            var item: [String: Any] = ["path": path]
            for (key, name) in [("role", kAXRoleAttribute), ("title", kAXTitleAttribute), ("description", kAXDescriptionAttribute), ("help", kAXHelpAttribute)] {
                if let value = string(element, name) { item[key] = value }
            }
            if let value = attribute(element, kAXValueAttribute) {
                if let text = value as? String { item["value"] = text }
                else if let number = value as? NSNumber { item["value"] = number }
            }
            if let point = position(element) { item["position"] = point }
            if let size = dimensions(element) { item["size"] = size }
            var actions: CFArray?
            if AXUIElementCopyActionNames(element, &actions) == .success { item["actions"] = actions as? [String] ?? [] }
            if args[1] == "brief" {
                let label = ["description", "title", "value"].compactMap { item[$0] as? String }.first { !$0.isEmpty } ?? ""
                let actions = item["actions"] as? [String] ?? []
                let matches = args.count < 4 || (args[3].hasPrefix("role:")
                    ? item["role"] as? String == String(args[3].dropFirst(5))
                    : label.localizedCaseInsensitiveContains(args[3]))
                if (path == requestedRoot || !label.isEmpty || actions.contains(kAXPressAction)) && matches {
                    var brief: [String: Any] = ["path": path, "role": item["role"] ?? "", "name": String(label.prefix(180)), "actions": actions]
                    brief["position"] = item["position"]
                    brief["size"] = item["size"]
                    brief["value"] = item["value"]
                    nodes.append(brief)
                }
            } else { nodes.append(item) }
            for (index, child) in children(element).enumerated() {
                if path == "menu" && index == 0 { continue }
                try walk(child, "\(path)/\(index)", depth + 1)
            }
        }
        try walk(roots[requestedRoot]!, requestedRoot, 0)
        try output(["bundle": bundle, "pid": running.processIdentifier, "nodes": nodes])
    case "press", "focus", "click", "hover":
        guard args.count == 3 || args.count == 4 else { try fail("Expected a fresh AX path and optional label guard") }
        let element = try resolve(args[2])
        if args.count == 4 {
            let labels = [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].compactMap { string(element, $0) }
            guard labels.contains(args[3]) else { try fail("AX label changed; request a fresh state") }
        }
        if args[1] == "press" { try check(AXUIElementPerformAction(element, kAXPressAction as CFString), "Press") }
        else if args[1] == "focus" { try check(AXUIElementSetAttributeValue(element, kAXFocusedAttribute as CFString, kCFBooleanTrue), "Focus") }
        else {
            try activate()
            guard CGPreflightPostEventAccess() else { try fail("QA input event access is unavailable; no permission prompt was opened") }
            try check(AXUIElementPerformAction(window, kAXRaiseAction as CFString), "Raise QA window")
            guard let p = position(element), let s = dimensions(element), s[0] > 0, s[1] > 0,
                  let wp = position(window), let ws = dimensions(window) else { try fail("QA target has no visible bounds") }
            let point = CGPoint(x: p[0] + s[0]/2, y: p[1] + s[1]/2)
            guard CGRect(x: wp[0], y: wp[1], width: ws[0], height: ws[1]).contains(point) else { try fail("Click is outside the QA document window") }
            if args[1] == "hover" {
                guard let move = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left) else { try fail("Cannot create QA pointer movement") }
                move.post(tap: .cghidEventTap)
                RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.15))
                return
            }
            guard let down = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left),
                  let up = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left) else { try fail("Cannot create QA click") }
            // Mouse events need WindowServer hit testing. Activation and the
            // verified QA window rectangle restrict their destination.
            down.setIntegerValueField(.mouseEventClickState, value: 1)
            up.setIntegerValueField(.mouseEventClickState, value: 1)
            down.post(tap: .cghidEventTap)
            RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.05))
            up.post(tap: .cghidEventTap)
            RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.15))
        }
    case "verify-copy":
        guard args.count == 4 else { try fail("Expected Copy button path and raw code label path") }
        let button = try resolve(args[2])
        let code = try resolve(args[3])
        let labels = [kAXDescriptionAttribute, kAXTitleAttribute, kAXValueAttribute]
        guard labels.contains(where: { string(button, $0) == "Copy" }),
              let expected = labels.compactMap({ string(code, $0) }).first(where: { !$0.isEmpty }) else {
            try fail("Expected a QA Copy control and nonempty raw code label")
        }
        try activate()
        try check(AXUIElementPerformAction(button, kAXPressAction as CFString), "Copy QA code")
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.2))
        guard labels.contains(where: { string(button, $0) == "Copied" }) else {
            try fail("QA Copy action was not acknowledged; clipboard was not inspected")
        }
        let actual = NSPasteboard.general.string(forType: .string)
        try output(["matches_raw_code": actual == expected, "expected_tabs": expected.filter { $0 == "\t" }.count])
        guard actual == expected else { try fail("QA copied text differs from its raw code label; contents were not printed") }
    case "value":
        guard args.count == 4 else { try fail("Expected AX path and value") }
        try check(AXUIElementSetAttributeValue(try resolve(args[2]), kAXValueAttribute as CFString, args[3] as CFString), "Set value")
    case "bounds":
        guard args.count == 6, let x = Double(args[2]), let y = Double(args[3]), let width = Double(args[4]), let height = Double(args[5]), width >= 200, height >= 200 else {
            try fail("Expected X Y WIDTH HEIGHT, at least200x200")
        }
        var point = CGPoint(x: x, y: y)
        var size = CGSize(width: width, height: height)
        guard let p = AXValueCreate(.cgPoint, &point), let s = AXValueCreate(.cgSize, &size) else { try fail("Cannot encode QA bounds") }
        try check(AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, s), "Resize QA window")
        try check(AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, p), "Position QA window")
    case "key":
        guard args.count == 3 else { try fail("Expected a key chord") }
        let keys: [String: CGKeyCode] = ["a":0, "s":1, "d":2, "f":3, "h":4, "g":5, "z":6, "x":7, "c":8, "v":9, "b":11, "q":12, "w":13, "e":14, "r":15, "y":16, "t":17, "o":31, "u":32, "i":34, "p":35, "l":37, "j":38, "k":40, "n":45, "m":46, "tab":48, "enter":36, "space":49, "escape":53, "home":115, "end":119, "pageup":116, "pagedown":121, "left":123, "right":124, "down":125, "up":126]
        let modifiers: [String: CGEventFlags] = ["cmd":.maskCommand, "shift":.maskShift, "alt":.maskAlternate, "ctrl":.maskControl]
        let parts = args[2].lowercased().split(separator: "+").map(String.init)
        guard let last = parts.last, let key = keys[last] else { try fail("Unsupported QA key") }
        let held = try parts.dropLast().map { part -> CGEventFlags in
            guard let flag = modifiers[part] else { try fail("Unsupported modifier") }
            return flag
        }
        try activate()
        guard let down = CGEvent(keyboardEventSource: nil, virtualKey: key, keyDown: true),
              let up = CGEvent(keyboardEventSource: nil, virtualKey: key, keyDown: false) else { try fail("Cannot create QA key event") }
        let flags = held.reduce(CGEventFlags()) { $0.union($1) }
        down.flags = flags
        up.flags = flags
        // Target this QA process explicitly; never send keys to a global tap.
        down.postToPid(running.processIdentifier)
        up.postToPid(running.processIdentifier)
        RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.15))
    default: try fail("Unsupported QA command")
    }
}
do { try main() }
catch { fputs("\(error)\n", stderr); exit(1) }
