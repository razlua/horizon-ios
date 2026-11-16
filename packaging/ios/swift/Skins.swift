// The players' skins: fetched from Mojang (the session server's profile, then the texture),
// drawn pixel-exact from the front, and changed with the skin changer (Upload hands the PNG to
// the game, crates/horizon-client/src/game/ios_boot.rs `upload_skin`, which sends it to the
// Minecraft services with the account's own sign-in).
import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// A skin's texture and its model (slim: arms 3 pixels wide).
struct Skin {
    let image: CGImage
    let slim: Bool
}

/// The skins of the saved accounts, by uuid, fetched once each.
final class SkinStore: ObservableObject {
    static let shared = SkinStore()

    @Published private(set) var skins: [String: Skin] = [:]
    /// The uuids being fetched (touched on the main thread only).
    private var loading: Set<String> = []

    /// Fetches the skin of `uuid` if it isn't there yet (or again, `force`). Call on the main thread.
    func load(_ uuid: String, force: Bool = false) {
        if uuid.isEmpty || loading.contains(uuid) || (!force && skins[uuid] != nil) {
            return
        }
        loading.insert(uuid)
        Task {
            let skin = await SkinStore.fetch(uuid)
            await MainActor.run {
                self.loading.remove(uuid)
                if let skin {
                    self.skins[uuid] = skin
                }
            }
        }
    }

    /// Shows `image` as the skin of `uuid` at once (just uploaded: Mojang may still serve the
    /// old one for a minute).
    func set(_ uuid: String, image: CGImage, slim: Bool) {
        skins[uuid] = Skin(image: image, slim: slim)
    }

    /// The profile's textures (base64 JSON), then the skin's PNG. Nil for anything that fails.
    private static func fetch(_ uuid: String) async -> Skin? {
        guard let profileURL = URL(string: "https://sessionserver.mojang.com/session/minecraft/profile/\(uuid)") else {
            return nil
        }
        do {
            let (data, _) = try await URLSession.shared.data(from: profileURL)
            guard let profile = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let properties = profile["properties"] as? [[String: Any]],
                  let textures = properties.first(where: { ($0["name"] as? String) == "textures" }),
                  let encoded = textures["value"] as? String,
                  let decoded = Data(base64Encoded: encoded),
                  let payload = try JSONSerialization.jsonObject(with: decoded) as? [String: Any],
                  let all = payload["textures"] as? [String: Any],
                  let skin = all["SKIN"] as? [String: Any],
                  var link = skin["url"] as? String
            else {
                return nil
            }
            if link.hasPrefix("http://") {
                link = "https://" + String(link.dropFirst(7))
            }
            let metadata = skin["metadata"] as? [String: Any]
            let slim = (metadata?["model"] as? String) == "slim"
            guard let skinURL = URL(string: link) else {
                return nil
            }
            let (png, _) = try await URLSession.shared.data(from: skinURL)
            guard let image = UIImage(data: png)?.cgImage else {
                return nil
            }
            return Skin(image: image, slim: slim)
        } catch {
            return nil
        }
    }
}

/// Skins drawn from their texture: the parts cut out and put together, overlays on top.
enum SkinArt {
    /// The player from the front: 16 x 32 pixels. Nil for a texture that isn't a skin.
    static func front(_ skin: Skin) -> CGImage? {
        let texture = skin.image
        let unit = texture.width / 64
        guard unit >= 1, texture.height == 64 * unit || texture.height == 32 * unit,
              let context = canvas(width: 16, height: 32)
        else {
            return nil
        }
        let legacy = texture.height == 32 * unit
        let arm = skin.slim ? 3 : 4
        // The head and its hat.
        put(texture, unit, 8, 8, 8, 8, at: 4, 0, in: context)
        put(texture, unit, 40, 8, 8, 8, at: 4, 0, in: context)
        // The body, the arms (the player's right one on the viewer's left) and the legs.
        put(texture, unit, 20, 20, 8, 12, at: 4, 8, in: context)
        put(texture, unit, 44, 20, arm, 12, at: 4 - arm, 8, in: context)
        put(texture, unit, 4, 20, 4, 12, at: 4, 20, in: context)
        if legacy {
            // Old skins have one arm and one leg: the left ones are their mirror.
            put(texture, unit, 44, 20, arm, 12, at: 12, 8, in: context, mirrored: true)
            put(texture, unit, 4, 20, 4, 12, at: 8, 20, in: context, mirrored: true)
        } else {
            put(texture, unit, 36, 52, arm, 12, at: 12, 8, in: context)
            put(texture, unit, 20, 52, 4, 12, at: 8, 20, in: context)
            // The second layer: jacket, sleeves, trousers.
            put(texture, unit, 20, 36, 8, 12, at: 4, 8, in: context)
            put(texture, unit, 44, 36, arm, 12, at: 4 - arm, 8, in: context)
            put(texture, unit, 52, 52, arm, 12, at: 12, 8, in: context)
            put(texture, unit, 4, 36, 4, 12, at: 4, 20, in: context)
            put(texture, unit, 4, 52, 4, 12, at: 8, 20, in: context)
        }
        return context.makeImage()
    }

    /// The face and the hat: 8 x 8 pixels.
    static func head(_ skin: Skin) -> CGImage? {
        let texture = skin.image
        let unit = texture.width / 64
        guard unit >= 1, let context = canvas(width: 8, height: 8) else {
            return nil
        }
        put(texture, unit, 8, 8, 8, 8, at: 0, 0, in: context)
        put(texture, unit, 40, 8, 8, 8, at: 0, 0, in: context)
        return context.makeImage()
    }

    private static func canvas(width: Int, height: Int) -> CGContext? {
        let context = CGContext(
            data: nil,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )
        context?.interpolationQuality = .none
        return context
    }

    /// The texture's part at `x, y` (`w` x `h` skin pixels) drawn at `dx, dy` of the context
    /// (counted from its top), mirrored left to right if asked.
    private static func put(
        _ texture: CGImage, _ unit: Int,
        _ x: Int, _ y: Int, _ w: Int, _ h: Int,
        at dx: Int, _ dy: Int,
        in context: CGContext,
        mirrored: Bool = false
    ) {
        let source = CGRect(x: x * unit, y: y * unit, width: w * unit, height: h * unit)
        guard let part = texture.cropping(to: source) else {
            return
        }
        // Core Graphics counts from the bottom.
        let top = CGFloat(context.height - dy - h)
        if mirrored {
            context.saveGState()
            context.translateBy(x: CGFloat(dx + w), y: 0)
            context.scaleBy(x: -1, y: 1)
            context.draw(part, in: CGRect(x: 0, y: top, width: CGFloat(w), height: CGFloat(h)))
            context.restoreGState()
        } else {
            context.draw(part, in: CGRect(x: CGFloat(dx), y: top, width: CGFloat(w), height: CGFloat(h)))
        }
    }
}

/// A skin's picture, pixel-sharp, or a faint figure without one.
struct SkinPicture: View {
    let art: CGImage?
    let height: CGFloat
    var head = false

    var body: some View {
        if let art {
            Image(decorative: art, scale: 1)
                .interpolation(.none)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(height: height)
        } else {
            Image(systemName: head ? "person.crop.square.fill" : "figure.stand")
                .resizable()
                .aspectRatio(contentMode: .fit)
                .foregroundStyle(Color.white.opacity(0.28))
                .padding(head ? 0 : height * 0.06)
                .frame(height: height)
        }
    }
}

/// The player of `uuid`, whole, in 3D (SkinModel3D.swift).
struct SkinFigure: View {
    let uuid: String
    var height: CGFloat = 200
    @ObservedObject private var store = SkinStore.shared

    var body: some View {
        SkinStage(skin: store.skins[uuid], height: height)
            .task(id: uuid) { store.load(uuid) }
    }
}

/// The face of `uuid`, for the account chips.
struct HeadAvatar: View {
    let uuid: String
    var size: CGFloat = 26
    @ObservedObject private var store = SkinStore.shared

    var body: some View {
        SkinPicture(art: store.skins[uuid].flatMap(SkinArt.head), height: size, head: true)
            .frame(width: size, height: size)
            .clipShape(RoundedRectangle(cornerRadius: size * 0.22, style: .continuous))
            .task(id: uuid) { store.load(uuid) }
    }
}

/// The skin changer: a skin file from Files, its model, a preview on the player, and Upload.
struct SkinChanger: View {
    @ObservedObject var model: LauncherModel
    let index: Int
    let account: AccountInfo
    @Environment(\.dismiss) private var dismiss
    @ObservedObject private var store = SkinStore.shared
    @State private var slim = false
    @State private var picked: CGImage? = nil
    @State private var pickedData: Data? = nil
    @State private var choosing = false
    @State private var problem: String? = nil
    @State private var sent = false

    private var status: String? { sent ? model.state.skin?.status : nil }

    var body: some View {
        ZStack {
            LauncherBackground()
            HStack(spacing: 28) {
                preview
                VStack(alignment: .leading, spacing: 12) {
                    Text("Change skin")
                        .font(.system(size: 24, weight: .semibold))
                        .foregroundStyle(Theme.text)
                    Text("For \(account.name). Pick a skin PNG (64 x 64) from Files, then Upload. It becomes your skin everywhere you play Java Edition.")
                        .font(.subheadline)
                        .foregroundStyle(Theme.secondary)
                    Picker("Model", selection: $slim) {
                        Text("Classic arms").tag(false)
                        Text("Slim arms").tag(true)
                    }
                    .pickerStyle(.segmented)
                    Button {
                        choosing = true
                    } label: {
                        Text(picked == nil ? "Choose a skin file" : "Choose another file")
                            .frame(maxWidth: .infinity)
                    }
                    .ferriteButton(prominent: false)
                    statusLine
                    Button {
                        upload()
                    } label: {
                        Text("Upload")
                            .font(.body.weight(.medium))
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 2)
                    }
                    .ferriteButton(prominent: true)
                    .disabled(pickedData == nil || status == "working")
                    Button("Done") { dismiss() }
                        .ferriteButton(prominent: false)
                }
                .frame(maxWidth: 360)
            }
            .padding(28)
        }
        .preferredColorScheme(.dark)
        .fileImporter(isPresented: $choosing, allowedContentTypes: [UTType.png]) { result in
            switch result {
            case .success(let url):
                read(url)
            case .failure(let error):
                problem = error.localizedDescription
            }
        }
        .onChange(of: model.state.skin?.status) { newStatus in
            // Uploaded: shown at once, as Mojang may still serve the old skin for a minute.
            if sent, newStatus == "done", let picked {
                store.set(account.uuid, image: picked, slim: slim)
                feelTap(.light)
            }
        }
    }

    /// The chosen skin on the player, or the current one, in 3D: drag to see its back.
    private var preview: some View {
        let shown = picked.map { Skin(image: $0, slim: slim) } ?? store.skins[account.uuid]
        return VStack(spacing: 8) {
            SkinStage(skin: shown, height: 236)
            Text(picked == nil ? "Current skin, drag to turn" : "Preview, drag to turn")
                .font(.footnote)
                .foregroundStyle(Theme.muted)
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 20)
        .card()
    }

    @ViewBuilder private var statusLine: some View {
        if let problem {
            Label(problem, systemImage: "exclamationmark.triangle")
                .font(.footnote)
                .foregroundStyle(Theme.bad)
        } else if status == "working" {
            HStack(spacing: 8) {
                ProgressView()
                Text("Uploading...").font(.footnote).foregroundStyle(Theme.secondary)
            }
        } else if status == "done" {
            Label("Your new skin is on", systemImage: "checkmark.circle")
                .font(.footnote)
                .foregroundStyle(Theme.good)
        } else if status == "failed" {
            Label(model.state.skin?.message ?? "The upload didn't work", systemImage: "exclamationmark.triangle")
                .font(.footnote)
                .foregroundStyle(Theme.bad)
        }
    }

    /// The file from Files: a PNG of 64 x 64 (or 64 x 32) pixels.
    private func read(_ url: URL) {
        let scoped = url.startAccessingSecurityScopedResource()
        defer {
            if scoped {
                url.stopAccessingSecurityScopedResource()
            }
        }
        guard let data = try? Data(contentsOf: url), let image = UIImage(data: data)?.cgImage else {
            problem = "That file isn't a PNG image"
            return
        }
        guard image.width == 64, image.height == 64 || image.height == 32 else {
            problem = "A skin is 64 x 64 pixels (or the old 64 x 32). This one is \(image.width) x \(image.height)."
            return
        }
        problem = nil
        picked = image
        pickedData = data
        sent = false
    }

    /// The PNG to the game folder for the game to send (`skin_upload:<index>:<model>`).
    private func upload() {
        guard let pickedData,
              let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
        else {
            return
        }
        do {
            try pickedData.write(to: documents.appendingPathComponent(".skin-upload.png"), options: .atomic)
        } catch {
            problem = "The skin couldn't be saved for the upload: \(error.localizedDescription)"
            return
        }
        problem = nil
        sent = true
        feelTap()
        model.send("skin_upload:\(index):\(slim ? "slim" : "classic")")
    }
}
