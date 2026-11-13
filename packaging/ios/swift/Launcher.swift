// Horizon iOS: the SwiftUI launcher, shown over the game's window before the game starts
// (crates/horizon-client/src/game/ios_boot.rs). The game drives it through the C functions below
// (crates/horizon-threads/src/ios_launcher.rs): it shows and hides it, sends it its state as JSON
// whenever that changes, and hears the player's taps back as actions (`hz_launcher_action`).
// The build compiles this file into libHorizonUI.a (.github/workflows/ios.yml), which the Rust
// build links (crates/horizon-threads/build.rs). Without it the game's own start screen shows.
import SwiftUI
import UIKit

/// The game's side: an action ("continue", ...) for the game to act on.
@_silgen_name("hz_launcher_action")
func hzLauncherAction(_ action: UnsafePointer<CChar>)

/// What the game says about itself.
struct LauncherState: Decodable {
    var version: String?
}

final class LauncherModel: ObservableObject {
    static let shared = LauncherModel()
    @Published var state = LauncherState()

    func apply(_ json: String) {
        if let data = json.data(using: .utf8),
           let state = try? JSONDecoder().decode(LauncherState.self, from: data) {
            self.state = state
        }
    }

    func send(_ action: String) {
        action.withCString { hzLauncherAction($0) }
    }
}

struct LauncherView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [Color(red: 0.10, green: 0.12, blue: 0.22), Color(red: 0.04, green: 0.05, blue: 0.09)],
                startPoint: .top, endPoint: .bottom
            )
            .ignoresSafeArea()
            VStack(spacing: 18) {
                Text("Horizon")
                    .font(.system(size: 44, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
                Text("Minecraft: Java Edition on iPhone. Unofficial.")
                    .font(.subheadline)
                    .foregroundStyle(.white.opacity(0.7))
                Button {
                    model.send("continue")
                } label: {
                    Text("Continue")
                        .font(.headline)
                        .frame(minWidth: 220)
                        .padding(.vertical, 12)
                }
                .buttonStyle(.borderedProminent)
                .tint(Color(red: 0.36, green: 0.61, blue: 0.25))
                if let version = model.state.version {
                    Text("Version \(version)")
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.5))
                }
            }
            .padding()
        }
    }
}

/// Landscape, with nothing of the system over it, as the game's own window.
final class LauncherController: UIHostingController<LauncherView> {
    override var supportedInterfaceOrientations: UIInterfaceOrientationMask { .landscape }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
}

private var shown: LauncherController?

/// The controller on top of the game's window, to present over.
private func topController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    let windows = scenes.flatMap { $0.windows }
    var top = (windows.first { $0.isKeyWindow } ?? windows.first)?.rootViewController
    while let presented = top?.presentedViewController {
        top = presented
    }
    return top
}

/// Shows the launcher over the game's window. False when there is no window to show it over.
@_cdecl("hz_launcher_show")
public func hzLauncherShow() -> Bool {
    if shown != nil {
        return true
    }
    guard let top = topController() else {
        return false
    }
    let controller = LauncherController(rootView: LauncherView(model: LauncherModel.shared))
    controller.modalPresentationStyle = .fullScreen
    top.present(controller, animated: false)
    shown = controller
    return true
}

/// Takes the launcher away: the game's screen shows again.
@_cdecl("hz_launcher_hide")
public func hzLauncherHide() {
    shown?.dismiss(animated: true)
    shown = nil
}

/// The game's state, as JSON (`LauncherState`).
@_cdecl("hz_launcher_update")
public func hzLauncherUpdate(_ json: UnsafePointer<CChar>) {
    LauncherModel.shared.apply(String(cString: json))
}
