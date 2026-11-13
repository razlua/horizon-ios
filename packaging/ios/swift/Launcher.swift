// Ferrite's SwiftUI launcher, shown over the game's window before the game starts
// (crates/horizon-client/src/game/ios_boot.rs). The game drives it through the C functions below
// (crates/horizon-threads/src/ios_launcher.rs): it shows and hides it, sends it its state as JSON
// whenever that changes (Model.swift), and hears the player's taps back as actions
// (`hz_launcher_action`; crates/horizon-client/src/ios_screen.rs, `parse_launcher_action`).
// The build compiles these files into libHorizonUI.a (.github/workflows/ios.yml), which the Rust
// build links (crates/horizon-threads/build.rs). Without it the game's own start screen shows.
import SwiftUI
import UIKit

/// The game's side: an action ("play:0", "add_account", ...) for the game to act on.
@_silgen_name("hz_launcher_action")
func hzLauncherAction(_ action: UnsafePointer<CChar>)

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

/// Ferrite's colors: the icon's yellow on charcoal.
enum Theme {
    static let accent = Color(red: 1.0, green: 0.76, blue: 0.10)
    static let background = Color(red: 0.07, green: 0.08, blue: 0.11)
    static let card = Color.white.opacity(0.06)
}

struct RootView: View {
    @ObservedObject var model: LauncherModel
    /// The tab shown first: Play, or the one `FERRITE_TAB` names (the simulator job's
    /// screenshots of each tab, .github/workflows/ios.yml).
    @State private var tab = ProcessInfo.processInfo.environment["FERRITE_TAB"] ?? "play"

    var body: some View {
        TabView(selection: $tab) {
            PlayView(model: model)
                .tabItem { Label("Play", systemImage: "play.fill") }
                .tag("play")
            RefreshView()
                .tabItem { Label("Refresh", systemImage: "arrow.clockwise") }
                .tag("refresh")
            SettingsView(model: model)
                .tabItem { Label("Settings", systemImage: "gearshape") }
                .tag("settings")
            AboutView(model: model)
                .tabItem { Label("About", systemImage: "info.circle") }
                .tag("about")
        }
        .tint(Theme.accent)
        // Dark always: the launcher's colors are made for it (LauncherController too).
        .environment(\.colorScheme, .dark)
        .preferredColorScheme(.dark)
        // The reminders follow this install's signature (Signing.swift).
        .onAppear { Signing.scheduleReminders() }
    }
}

/// Landscape, with nothing of the system over it, as the game's own window.
final class LauncherController: UIHostingController<RootView> {
    override func viewDidLoad() {
        super.viewDidLoad()
        // The phone may be in light mode: the launcher stays dark, with light text and tab bar.
        overrideUserInterfaceStyle = .dark
        view.backgroundColor = UIColor(red: 0.07, green: 0.08, blue: 0.11, alpha: 1)
    }
    override var supportedInterfaceOrientations: UIInterfaceOrientationMask { .landscape }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
}

private var shown: LauncherController?

/// The controller on top of the game's window, to present over.
func topController() -> UIViewController? {
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
    let controller = LauncherController(rootView: RootView(model: LauncherModel.shared))
    controller.modalPresentationStyle = .fullScreen
    top.present(controller, animated: false)
    shown = controller
    return true
}

/// Takes the launcher away: the game shows.
@_cdecl("hz_launcher_hide")
public func hzLauncherHide() {
    shown?.dismiss(animated: true)
    shown = nil
}

/// The game's state, as JSON (Model.swift).
@_cdecl("hz_launcher_update")
public func hzLauncherUpdate(_ json: UnsafePointer<CChar>) {
    LauncherModel.shared.apply(String(cString: json))
}
