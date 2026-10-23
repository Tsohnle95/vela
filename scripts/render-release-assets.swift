import AppKit
import Foundation

// Release graphics matching the Vela release layout with Vela branding.
let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let output = root.appendingPathComponent("docs/assets/releases")
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255,
            blue: CGFloat(hex & 255) / 255, alpha: 1)
}
func text(_ value: String, at point: NSPoint, size: CGFloat, weight: NSFont.Weight, tint: NSColor) {
    NSAttributedString(string: value, attributes: [.font: NSFont.systemFont(ofSize: size, weight: weight),
        .foregroundColor: tint]).draw(at: point)
}
func canvas(_ name: String, width: Int, height: Int, draw: () -> Void) throws {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSGraphicsContext.current?.imageInterpolation = .high
    draw()
    NSGraphicsContext.restoreGraphicsState()
    try bitmap.representation(using: .png, properties: [:])!.write(to: output.appendingPathComponent(name))
}
try canvas("vela-banner.png", width: 1600, height: 400) {
    let bounds = NSRect(x: 0, y: 0, width: 1600, height: 400)
    let frame = NSBezierPath(roundedRect: bounds, xRadius: 28, yRadius: 28)
    NSGradient(starting: color(0x141C18), ending: color(0x0C110E))!.draw(in: frame, angle: 0)
    let screenshot = NSImage(contentsOf: root.appendingPathComponent("resources/vela.png"))!
    let screenshotBounds = NSRect(x: 930, y: 25, width: 560, height: 350)
    NSGraphicsContext.saveGraphicsState()
    NSBezierPath(roundedRect: screenshotBounds, xRadius: 12, yRadius: 12).addClip()
    screenshot.draw(in: screenshotBounds, from: .zero, operation: .sourceOver, fraction: 1)
    NSGraphicsContext.restoreGraphicsState()
    text("VELA FOR macOS", at: NSPoint(x: 62, y: 298), size: 20, weight: .semibold, tint: color(0x9EB4A1))
    text("Vela", at: NSPoint(x: 57, y: 175), size: 88, weight: .bold, tint: color(0xF5F8F5))
    text("Your code. Your workspace.", at: NSPoint(x: 62, y: 112), size: 32, weight: .regular, tint: color(0xC0CBC2))
    text("Code editor  /  AI agents  /  Live diffs", at: NSPoint(x: 62, y: 58), size: 21,
        weight: .regular, tint: color(0x91A095))
}
try canvas("download-macos.png", width: 640, height: 128) {
    color(0x617A68).setFill()
    NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: 640, height: 128), xRadius: 24, yRadius: 24).fill()
    NSColor.white.setStroke()
    let arrow = NSBezierPath()
    arrow.lineWidth = 5
    arrow.lineCapStyle = .round
    arrow.lineJoinStyle = .round
    arrow.move(to: NSPoint(x: 58, y: 85)); arrow.line(to: NSPoint(x: 58, y: 48))
    arrow.move(to: NSPoint(x: 43, y: 62)); arrow.line(to: NSPoint(x: 58, y: 47)); arrow.line(to: NSPoint(x: 73, y: 62))
    arrow.move(to: NSPoint(x: 38, y: 41)); arrow.line(to: NSPoint(x: 38, y: 32))
    arrow.line(to: NSPoint(x: 78, y: 32)); arrow.line(to: NSPoint(x: 78, y: 41)); arrow.stroke()
    text("Download for macOS", at: NSPoint(x: 108, y: 40), size: 43, weight: .semibold, tint: .white)
}
print("Rendered release banner and download button.")
