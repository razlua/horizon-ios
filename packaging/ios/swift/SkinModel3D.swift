// The player in 3D, drawn with Core Graphics: the skin's parts as boxes, each face cut from its
// place in the 64 x 64 texture as the game lays it out, turned and tilted, shaded by which way it
// faces (the top lightest, as in the game's menus), and drawn back to front, pixel-sharp. The
// second layer (hat, jacket, sleeves, trousers) is a little bigger around the first. A finger
// turns the player. No GPU: it looks the same on a phone as in the Simulator's screenshots, and
// it is drawn only when it turns.
import SwiftUI
import UIKit

/// A point or a direction in the model: x to the player's left (the viewer's right), y up, z out
/// of the screen. One unit is one skin pixel.
struct Vec3 {
    var x: Double
    var y: Double
    var z: Double

    static func + (a: Vec3, b: Vec3) -> Vec3 { Vec3(x: a.x + b.x, y: a.y + b.y, z: a.z + b.z) }
    static func - (a: Vec3, b: Vec3) -> Vec3 { Vec3(x: a.x - b.x, y: a.y - b.y, z: a.z - b.z) }

    func dot(_ o: Vec3) -> Double { x * o.x + y * o.y + z * o.z }

    /// Turned about the z axis by `angle` around `pivot` (an arm out from its shoulder).
    func swung(_ angle: Double, around pivot: Vec3) -> Vec3 {
        let (s, c) = (sin(angle), cos(angle))
        let dx = x - pivot.x
        let dy = y - pivot.y
        return Vec3(x: pivot.x + dx * c - dy * s, y: pivot.y + dx * s + dy * c, z: z)
    }

    /// Turned about y by `yaw`, then tipped toward the viewer about x by `tilt`.
    func viewed(yaw: Double, tilt: Double) -> Vec3 {
        let x1 = x * cos(yaw) + z * sin(yaw)
        let z1 = -x * sin(yaw) + z * cos(yaw)
        return Vec3(x: x1, y: y * cos(tilt) - z1 * sin(tilt), z: y * sin(tilt) + z1 * cos(tilt))
    }
}

/// One box of the model: its middle, half its size, its six faces' pictures and how it hangs.
struct SkinBox {
    let center: Vec3
    let half: Vec3
    /// Front, back, +x (the player's left), -x (the player's right), top, bottom.
    let faces: [CGImage?]
    let overlay: Bool
    /// An arm turns out from its shoulder by `swing` (radians, about z).
    let pivot: Vec3?
    let swing: Double
}

enum SkinModel {
    /// Each face's corners as signs of the box's half size: top left, top right and bottom left
    /// of its picture, seen from outside the box. Then its outward direction.
    private static let corners: [(Vec3, Vec3, Vec3, Vec3)] = [
        (Vec3(x: -1, y: 1, z: 1), Vec3(x: 1, y: 1, z: 1), Vec3(x: -1, y: -1, z: 1), Vec3(x: 0, y: 0, z: 1)),
        (Vec3(x: 1, y: 1, z: -1), Vec3(x: -1, y: 1, z: -1), Vec3(x: 1, y: -1, z: -1), Vec3(x: 0, y: 0, z: -1)),
        (Vec3(x: 1, y: 1, z: 1), Vec3(x: 1, y: 1, z: -1), Vec3(x: 1, y: -1, z: 1), Vec3(x: 1, y: 0, z: 0)),
        (Vec3(x: -1, y: 1, z: -1), Vec3(x: -1, y: 1, z: 1), Vec3(x: -1, y: -1, z: -1), Vec3(x: -1, y: 0, z: 0)),
        (Vec3(x: -1, y: 1, z: -1), Vec3(x: 1, y: 1, z: -1), Vec3(x: -1, y: 1, z: 1), Vec3(x: 0, y: 1, z: 0)),
        (Vec3(x: -1, y: -1, z: 1), Vec3(x: 1, y: -1, z: 1), Vec3(x: -1, y: -1, z: -1), Vec3(x: 0, y: -1, z: 0)),
    ]

    /// Where the light comes from: the front, above and a little to the left.
    private static let light: Vec3 = {
        let l = Vec3(x: -0.35, y: 0.75, z: 0.55)
        let n = (l.x * l.x + l.y * l.y + l.z * l.z).squareRoot()
        return Vec3(x: l.x / n, y: l.y / n, z: l.z / n)
    }()

    /// The last skin's boxes, so turning doesn't cut the texture again.
    private static var cached: (image: ObjectIdentifier, slim: Bool, boxes: [SkinBox])?

    /// The player's boxes, its middle at the origin (y from -16 to 16). Empty for a texture that
    /// isn't a skin.
    static func boxes(for skin: Skin) -> [SkinBox] {
        let key = ObjectIdentifier(skin.image)
        if let hit = cached, hit.image == key, hit.slim == skin.slim {
            return hit.boxes
        }
        let built = build(skin)
        cached = (key, skin.slim, built)
        return built
    }

    private static func build(_ skin: Skin) -> [SkinBox] {
        let texture = skin.image
        let unit = texture.width / 64
        guard unit >= 1, texture.height == 64 * unit || texture.height == 32 * unit else {
            return []
        }
        let legacy = texture.height == 32 * unit
        let arm = skin.slim ? 3 : 4
        let armX = 4 + Double(arm) / 2
        var boxes: [SkinBox] = []

        /// A part (and its second layer at `layer`) of `w x h x d` pixels, its middle at `center`.
        func part(
            _ base: (Int, Int), _ layer: (Int, Int)?,
            _ w: Int, _ h: Int, _ d: Int,
            grow: Double, center: Vec3,
            mirrored: Bool = false, pivot: Vec3? = nil, swing: Double = 0
        ) {
            let half = Vec3(x: Double(w) / 2, y: Double(h) / 2, z: Double(d) / 2)
            boxes.append(SkinBox(
                center: center, half: half,
                faces: faces(texture, unit: unit, u: base.0, v: base.1, w: w, h: h, d: d, mirrored: mirrored),
                overlay: false, pivot: pivot, swing: swing
            ))
            if let layer {
                boxes.append(SkinBox(
                    center: center,
                    half: Vec3(x: half.x + grow, y: half.y + grow, z: half.z + grow),
                    faces: faces(texture, unit: unit, u: layer.0, v: layer.1, w: w, h: h, d: d, mirrored: mirrored),
                    overlay: true, pivot: pivot, swing: swing
                ))
            }
        }

        // The head and its hat. An old skin with its hat area filled solid (Notch's is black) has
        // no hat, as the game decides (`doNotchTransparencyHack`): it would hide the whole head.
        let hat: (Int, Int)? = legacy && !seeThrough(texture, unit: unit, 32, 0, 32, 16) ? nil : (32, 0)
        part((0, 0), hat, 8, 8, 8, grow: 0.5, center: Vec3(x: 0, y: 12, z: 0))
        // The body and its jacket.
        part((16, 16), legacy ? nil : (16, 32), 8, 12, 4, grow: 0.25, center: Vec3(x: 0, y: 2, z: 0))
        // The player's right arm and leg are on the viewer's left (-x). Arms hang from their
        // shoulders turned out a little, as the inventory shows the player.
        let rightShoulder = Vec3(x: -armX, y: 8, z: 0)
        let leftShoulder = Vec3(x: armX, y: 8, z: 0)
        part((40, 16), legacy ? nil : (40, 32), arm, 12, 4, grow: 0.25,
             center: Vec3(x: -armX, y: 2, z: 0), pivot: rightShoulder, swing: -0.06)
        if legacy {
            part((40, 16), nil, arm, 12, 4, grow: 0.25,
                 center: Vec3(x: armX, y: 2, z: 0), mirrored: true, pivot: leftShoulder, swing: 0.06)
        } else {
            part((32, 48), (48, 48), arm, 12, 4, grow: 0.25,
                 center: Vec3(x: armX, y: 2, z: 0), pivot: leftShoulder, swing: 0.06)
        }
        part((0, 16), legacy ? nil : (0, 32), 4, 12, 4, grow: 0.25, center: Vec3(x: -2, y: -10, z: 0))
        if legacy {
            part((0, 16), nil, 4, 12, 4, grow: 0.25, center: Vec3(x: 2, y: -10, z: 0), mirrored: true)
        } else {
            part((16, 48), (0, 48), 4, 12, 4, grow: 0.25, center: Vec3(x: 2, y: -10, z: 0))
        }
        return boxes
    }

    /// A box's faces from the skin, in `corners`' order. A part at `u, v` of `w x h x d` pixels
    /// lies in the texture as the game lays it out: top and bottom above, then the right side,
    /// front, left side and back in a row. A mirrored part (an old skin's left arm and leg) is
    /// its right one flipped.
    private static func faces(
        _ texture: CGImage, unit: Int,
        u: Int, v: Int, w: Int, h: Int, d: Int,
        mirrored: Bool
    ) -> [CGImage?] {
        func cut(_ x: Int, _ y: Int, _ cw: Int, _ ch: Int) -> CGImage? {
            let rect = CGRect(x: x * unit, y: y * unit, width: cw * unit, height: ch * unit)
            guard let part = texture.cropping(to: rect) else {
                return nil
            }
            return mirrored ? flipped(part) : part
        }
        let top = cut(u + d, v, w, d)
        let bottom = cut(u + d + w, v, w, d)
        let playerRight = cut(u, v + d, d, h)
        let front = cut(u + d, v + d, w, h)
        let playerLeft = cut(u + d + w, v + d, d, h)
        let back = cut(u + d + w + d, v + d, w, h)
        if mirrored {
            return [front, back, playerRight, playerLeft, top, bottom]
        }
        return [front, back, playerLeft, playerRight, top, bottom]
    }

    /// `image` mirrored left to right.
    private static func flipped(_ image: CGImage) -> CGImage? {
        guard let context = CGContext(
            data: nil,
            width: image.width,
            height: image.height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            return nil
        }
        context.interpolationQuality = .none
        context.translateBy(x: CGFloat(image.width), y: 0)
        context.scaleBy(x: -1, y: 1)
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return context.makeImage()
    }

    /// Whether the texture's area at `x, y` (`w` x `h` skin pixels) has a see-through pixel.
    static func seeThrough(_ texture: CGImage, unit: Int, _ x: Int, _ y: Int, _ w: Int, _ h: Int) -> Bool {
        let rect = CGRect(x: x * unit, y: y * unit, width: w * unit, height: h * unit)
        guard let area = texture.cropping(to: rect) else {
            return true
        }
        let width = area.width
        let height = area.height
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        return pixels.withUnsafeMutableBytes { buffer -> Bool in
            guard let context = CGContext(
                data: buffer.baseAddress,
                width: width,
                height: height,
                bitsPerComponent: 8,
                bytesPerRow: width * 4,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
            ) else {
                return true
            }
            context.draw(area, in: CGRect(x: 0, y: 0, width: width, height: height))
            var i = 3
            while i < buffer.count {
                if buffer[i] < 128 {
                    return true
                }
                i += 4
            }
            return false
        }
    }

    /// The player drawn into a picture of `size` points at `scale` pixels a point, turned by
    /// `yaw` and seen a little from above.
    static func render(_ skin: Skin, yaw: Double, tilt: Double = 0.14, size: CGSize, scale: CGFloat) -> CGImage? {
        let parts = SkinModel.boxes(for: skin)
        let width = Int(size.width * scale)
        let height = Int(size.height * scale)
        guard !parts.isEmpty, width > 0, height > 0, let context = CGContext(
            data: nil,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            return nil
        }
        context.interpolationQuality = .none
        context.setAllowsAntialiasing(false)
        context.setShouldAntialias(false)

        // Skin pixels to picture pixels: the player (33 tall with the hat) fills most of it.
        let pixel = Double(height) / 38
        let middle = (x: Double(width) / 2, y: Double(height) / 2 + pixel * 0.5)
        func onScreen(_ p: Vec3) -> CGPoint {
            CGPoint(x: middle.x + p.x * pixel, y: middle.y + p.y * pixel)
        }

        struct Drawn {
            let image: CGImage
            let topLeft: CGPoint
            let topRight: CGPoint
            let bottomLeft: CGPoint
            let depth: Double
            let shade: Double
        }
        var drawn: [Drawn] = []
        for box in parts {
            func place(_ signs: Vec3) -> Vec3 {
                var p = box.center + Vec3(x: signs.x * box.half.x, y: signs.y * box.half.y, z: signs.z * box.half.z)
                if let pivot = box.pivot {
                    p = p.swung(box.swing, around: pivot)
                }
                return p.viewed(yaw: yaw, tilt: tilt)
            }
            for (index, corner) in corners.enumerated() {
                guard let image = box.faces[index] else {
                    continue
                }
                var normal = corner.3
                if box.pivot != nil {
                    normal = normal.swung(box.swing, around: Vec3(x: 0, y: 0, z: 0))
                }
                normal = normal.viewed(yaw: yaw, tilt: tilt)
                // Only the faces turned toward the viewer.
                if normal.z <= 0.001 {
                    continue
                }
                let topLeft = place(corner.0)
                let topRight = place(corner.1)
                let bottomLeft = place(corner.2)
                let bottomRight = topRight + bottomLeft - topLeft
                let depth = (topLeft.z + bottomRight.z) / 2 + (box.overlay ? 0.01 : 0)
                let shade = 0.48 + 0.52 * max(0, normal.dot(light))
                drawn.append(Drawn(
                    image: image,
                    topLeft: onScreen(topLeft),
                    topRight: onScreen(topRight),
                    bottomLeft: onScreen(bottomLeft),
                    depth: depth,
                    shade: shade
                ))
            }
        }
        // Back to front.
        drawn.sort { $0.depth < $1.depth }
        let unitSquare = CGRect(x: 0, y: 0, width: 1, height: 1)
        for face in drawn {
            // The unit square onto the face: (0, 0) its bottom left, (1, 0) its bottom right,
            // (0, 1) its top left. Core Graphics draws a picture upright in the square.
            let bottomRight = CGPoint(
                x: face.topRight.x + face.bottomLeft.x - face.topLeft.x,
                y: face.topRight.y + face.bottomLeft.y - face.topLeft.y
            )
            let transform = CGAffineTransform(
                a: bottomRight.x - face.bottomLeft.x,
                b: bottomRight.y - face.bottomLeft.y,
                c: face.topLeft.x - face.bottomLeft.x,
                d: face.topLeft.y - face.bottomLeft.y,
                tx: face.bottomLeft.x,
                ty: face.bottomLeft.y
            )
            context.saveGState()
            context.concatenate(transform)
            context.clip(to: unitSquare)
            context.beginTransparencyLayer(auxiliaryInfo: nil)
            context.draw(face.image, in: unitSquare)
            // Darker the more it faces away from the light, only where the face has pixels.
            context.setBlendMode(.sourceAtop)
            context.setFillColor(gray: 0, alpha: CGFloat(1 - face.shade))
            context.fill(unitSquare)
            context.endTransparencyLayer()
            context.restoreGState()
        }
        return context.makeImage()
    }
}

/// The player in 3D, turned by a finger, on a soft shadow; a faint figure until the skin is in.
struct SkinStage: View {
    let skin: Skin?
    var height: CGFloat = 220
    /// Three quarters: the face and one side, as players are shown in the game's menus.
    @State private var yaw: Double = -0.5
    @State private var dragged: Double = 0

    var body: some View {
        let size = CGSize(width: (height * 0.62).rounded(), height: height)
        ZStack(alignment: .bottom) {
            Ellipse()
                .fill(Color.black.opacity(0.35))
                .frame(width: height * 0.3, height: height * 0.05)
                .blur(radius: 4)
                .offset(y: -height * 0.03)
            if let skin, let picture = SkinModel.render(skin, yaw: yaw + dragged, size: size, scale: 3) {
                Image(decorative: picture, scale: 3)
                    .interpolation(.none)
                    .frame(width: size.width, height: size.height)
            } else {
                Image(systemName: "figure.stand")
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .foregroundStyle(Color.white.opacity(0.2))
                    .padding(height * 0.08)
                    .frame(width: size.width, height: size.height)
            }
        }
        .frame(width: size.width, height: size.height)
        .contentShape(Rectangle())
        .gesture(
            DragGesture(minimumDistance: 2)
                .onChanged { drag in
                    dragged = Double(drag.translation.width) / 55
                }
                .onEnded { drag in
                    yaw += Double(drag.translation.width) / 55
                    dragged = 0
                }
        )
        .accessibilityLabel("Your player. Drag to turn it.")
    }
}
