// The player in 3D: the skin's parts as boxes, textured pixel-sharp from the skin as the game
// does (each face cut from its place in the 64 x 64 texture), the second layer (hat, jacket,
// sleeves, trousers) a little bigger around them. A finger turns the player. SceneKit draws it
// only when something changes, so it costs nothing while it stands still.
import SceneKit
import SwiftUI
import UIKit

/// Builds the player's model from a skin.
enum SkinModel {
    /// The whole player, its middle at the origin (y from -16 to 16), facing +z. Nil for a
    /// texture that isn't a skin.
    static func node(for skin: Skin) -> SCNNode? {
        let texture = skin.image
        let unit = texture.width / 64
        guard unit >= 1, texture.height == 64 * unit || texture.height == 32 * unit else {
            return nil
        }
        let legacy = texture.height == 32 * unit
        let arm = skin.slim ? 3 : 4
        let armOffset = Float(4) + Float(arm) / 2
        let player = SCNNode()

        func box(
            _ u: Int, _ v: Int, _ w: Int, _ h: Int, _ d: Int,
            grow: CGFloat, overlay: Bool, mirrored: Bool = false
        ) -> SCNNode {
            let geometry = SCNBox(
                width: CGFloat(w) + grow * 2,
                height: CGFloat(h) + grow * 2,
                length: CGFloat(d) + grow * 2,
                chamferRadius: 0
            )
            geometry.materials = faces(texture, unit: unit, u: u, v: v, w: w, h: h, d: d, mirrored: mirrored)
                .map { material($0, overlay: overlay) }
            let node = SCNNode(geometry: geometry)
            // The second layer after the first, so its see-through pixels show the first.
            node.renderingOrder = overlay ? 1 : 0
            return node
        }

        /// A part with its second layer, `center` its middle.
        func part(
            base: (Int, Int), layer: (Int, Int)?,
            _ w: Int, _ h: Int, _ d: Int,
            grow: CGFloat, center: SCNVector3, mirrored: Bool = false
        ) -> SCNNode {
            let node = SCNNode()
            node.position = center
            node.addChildNode(box(base.0, base.1, w, h, d, grow: 0, overlay: false, mirrored: mirrored))
            if let layer {
                node.addChildNode(box(layer.0, layer.1, w, h, d, grow: grow, overlay: true, mirrored: mirrored))
            }
            return node
        }

        /// An arm, hanging from its shoulder and turned out a little, as the inventory shows it.
        func limb(_ child: SCNNode, shoulder: SCNVector3, outward: Float) -> SCNNode {
            let pivot = SCNNode()
            pivot.position = shoulder
            child.position = SCNVector3(0, -6, 0)
            pivot.addChildNode(child)
            pivot.eulerAngles.z = outward
            return pivot
        }

        // The head and its hat.
        player.addChildNode(part(base: (0, 0), layer: (32, 0), 8, 8, 8, grow: 0.5, center: SCNVector3(0, 12, 0)))
        // The body and its jacket.
        player.addChildNode(part(
            base: (16, 16), layer: legacy ? nil : (16, 32), 8, 12, 4,
            grow: 0.25, center: SCNVector3(0, 2, 0)
        ))
        // The player's right arm and leg are on the viewer's left (-x).
        let rightArm = part(base: (40, 16), layer: legacy ? nil : (40, 32), arm, 12, 4, grow: 0.25, center: SCNVector3(0, 0, 0))
        let leftArm = legacy
            ? part(base: (40, 16), layer: nil, arm, 12, 4, grow: 0.25, center: SCNVector3(0, 0, 0), mirrored: true)
            : part(base: (32, 48), layer: (48, 48), arm, 12, 4, grow: 0.25, center: SCNVector3(0, 0, 0))
        player.addChildNode(limb(rightArm, shoulder: SCNVector3(-armOffset, 8, 0), outward: -0.06))
        player.addChildNode(limb(leftArm, shoulder: SCNVector3(armOffset, 8, 0), outward: 0.06))
        player.addChildNode(part(
            base: (0, 16), layer: legacy ? nil : (0, 32), 4, 12, 4,
            grow: 0.25, center: SCNVector3(-2, -10, 0)
        ))
        player.addChildNode(legacy
            ? part(base: (0, 16), layer: nil, 4, 12, 4, grow: 0.25, center: SCNVector3(2, -10, 0), mirrored: true)
            : part(base: (16, 48), layer: (0, 48), 4, 12, 4, grow: 0.25, center: SCNVector3(2, -10, 0)))
        return player
    }

    /// A box's six faces from the skin, in SCNBox's order: front (+z), right (+x, the player's
    /// left), back, left (-x, the player's right), top, bottom. A part at `u, v` of `w x h x d`
    /// pixels lies in the texture as the game lays it out: top and bottom above, then the right
    /// side, front, left side and back in a row. A mirrored part (an old skin's left arm and leg)
    /// is its right one flipped.
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
            return [front, playerRight, back, playerLeft, top, bottom]
        }
        return [front, playerLeft, back, playerRight, top, bottom]
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

    /// A face's material: its pixels sharp. The first layer is solid, as in the game; the second
    /// shows only its drawn pixels.
    private static func material(_ image: CGImage?, overlay: Bool) -> SCNMaterial {
        let material = SCNMaterial()
        material.lightingModel = .lambert
        if let image {
            material.diffuse.contents = image
        } else {
            material.diffuse.contents = UIColor.clear
        }
        material.diffuse.magnificationFilter = .nearest
        material.diffuse.minificationFilter = .nearest
        material.diffuse.mipFilter = .none
        if overlay {
            material.isDoubleSided = true
            material.shaderModifiers = [
                .fragment: "if (_output.color.a < 0.5) { discard_fragment(); }",
            ]
        } else {
            material.blendMode = .replace
        }
        return material
    }
}

/// The SceneKit view: the player, a camera looking at it straight on, and soft light from the
/// front and above. `yaw` turns the player.
struct SkinSceneView: UIViewRepresentable {
    let skin: Skin
    let yaw: Double

    final class Coordinator {
        var image: CGImage?
        var slim = false
        let turn = SCNNode()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> SCNView {
        let scene = SCNScene()
        scene.background.contents = UIColor.clear

        let camera = SCNCamera()
        camera.usesOrthographicProjection = true
        camera.orthographicScale = 18
        camera.zNear = 1
        camera.zFar = 400
        let cameraNode = SCNNode()
        cameraNode.camera = camera
        cameraNode.position = SCNVector3(0, 0, 120)
        scene.rootNode.addChildNode(cameraNode)

        let ambient = SCNNode()
        ambient.light = SCNLight()
        ambient.light?.type = .ambient
        ambient.light?.intensity = 620
        ambient.light?.color = UIColor.white
        scene.rootNode.addChildNode(ambient)

        let key = SCNNode()
        key.light = SCNLight()
        key.light?.type = .directional
        key.light?.intensity = 520
        key.light?.color = UIColor.white
        // From the front, above and a little to the left, as the inventory's light.
        key.eulerAngles = SCNVector3(-0.55, -0.35, 0)
        scene.rootNode.addChildNode(key)

        // Seen a little from above.
        let tilt = SCNNode()
        tilt.eulerAngles.x = 0.12
        tilt.addChildNode(context.coordinator.turn)
        scene.rootNode.addChildNode(tilt)

        let view = SCNView(frame: .zero)
        view.scene = scene
        view.pointOfView = cameraNode
        view.backgroundColor = UIColor.clear
        view.isOpaque = false
        view.antialiasingMode = .multisampling4X
        view.rendersContinuously = false
        view.allowsCameraControl = false
        view.isUserInteractionEnabled = false
        place(skin, in: context.coordinator)
        context.coordinator.turn.eulerAngles.y = Float(yaw)
        return view
    }

    func updateUIView(_ view: SCNView, context: Context) {
        if context.coordinator.image !== skin.image || context.coordinator.slim != skin.slim {
            place(skin, in: context.coordinator)
        }
        context.coordinator.turn.eulerAngles.y = Float(yaw)
    }

    private func place(_ skin: Skin, in coordinator: Coordinator) {
        coordinator.turn.childNodes.forEach { $0.removeFromParentNode() }
        if let player = SkinModel.node(for: skin) {
            coordinator.turn.addChildNode(player)
        }
        coordinator.image = skin.image
        coordinator.slim = skin.slim
    }
}

/// The player in 3D, turned by a finger, on a soft shadow; a faint figure until the skin is in.
struct SkinStage: View {
    let skin: Skin?
    var height: CGFloat = 220
    /// Three quarters: the face and one side, as players are shown in the game's menus.
    @State private var yaw: Double = -0.45
    @State private var dragged: Double = 0

    var body: some View {
        ZStack(alignment: .bottom) {
            Ellipse()
                .fill(Color.black.opacity(0.35))
                .frame(width: height * 0.32, height: height * 0.05)
                .blur(radius: 4)
                .offset(y: -height * 0.02)
            if let skin {
                SkinSceneView(skin: skin, yaw: yaw + dragged)
                    .frame(width: height * 0.62, height: height)
            } else {
                Image(systemName: "figure.stand")
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .foregroundStyle(Color.white.opacity(0.2))
                    .padding(height * 0.08)
                    .frame(width: height * 0.62, height: height)
            }
        }
        .frame(width: height * 0.62, height: height)
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
