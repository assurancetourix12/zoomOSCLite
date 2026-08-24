import AppKit

guard CommandLine.arguments.count == 2 else {
    fatalError("uso: generate-icon <output.png>")
}

let size = NSSize(width: 1024, height: 1024)
let image = NSImage(size: size)
image.lockFocus()

let bounds = NSRect(origin: .zero, size: size).insetBy(dx: 36, dy: 36)
let background = NSBezierPath(roundedRect: bounds, xRadius: 210, yRadius: 210)
let gradient = NSGradient(colors: [
    NSColor(calibratedRed: 0.08, green: 0.18, blue: 0.48, alpha: 1),
    NSColor(calibratedRed: 0.08, green: 0.48, blue: 0.92, alpha: 1),
    NSColor(calibratedRed: 0.35, green: 0.16, blue: 0.76, alpha: 1),
])!
gradient.draw(in: background, angle: -42)

let center = NSPoint(x: 512, y: 512)
NSColor.white.setFill()
NSBezierPath(ovalIn: NSRect(x: 452, y: 452, width: 120, height: 120)).fill()

NSColor.white.withAlphaComponent(0.96).setStroke()
for (radius, width) in [(175.0, 46.0), (285.0, 44.0), (395.0, 42.0)] {
    for (start, end) in [(-48.0, 48.0), (132.0, 228.0)] {
        let arc = NSBezierPath()
        arc.appendArc(withCenter: center, radius: radius, startAngle: start, endAngle: end)
        arc.lineWidth = width
        arc.lineCapStyle = .round
        arc.stroke()
    }
}

image.unlockFocus()

guard let tiff = image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("não foi possível gerar PNG")
}
try png.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
