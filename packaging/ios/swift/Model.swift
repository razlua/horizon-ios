// What the game sends the launcher (crates/horizon-client/src/game/ios_boot.rs, `launcher_state`):
// the same state its own start screen draws. Every field is optional, so an older or newer game
// still decodes.
import Foundation

struct LauncherState: Decodable {
    var version: String?
    var account: AccountState?
    var accounts: [String]?
    var setup: SetupState?
    var unpacking: Counts?
    var update: UpdateInfo?
    var settings: SettingsState?
}

/// The game's settings the Settings tab changes.
struct SettingsState: Decodable {
    var render_distance: Int
    var max_fps: Int
    var battery: Bool
    var extras: [ExtraSetting]
}

/// An Extra Settings (Ferrite) entry: its label with the value ("Safe Area: ON") and what it does.
struct ExtraSetting: Decodable {
    var label: String
    var help: String
}

/// Where the Microsoft sign-in stands. `kind`: choose, resuming, asking, code, signed, failed,
/// offline_name or offline.
struct AccountState: Decodable {
    var kind: String
    var removing: Bool?
    var name: String?
    var code: String?
    var uri: String?
    var link: String?
    var error: String?
    var problem: String?
}

/// The game files' download on the phone. `kind`: not_needed, waiting, running, failed or done.
struct SetupState: Decodable {
    var kind: String
    var error: String?
    var stage: String?
    var done: UInt64?
    var total: UInt64?
}

struct Counts: Decodable {
    var done: UInt64
    var total: UInt64
}

/// A newer release, and the app that installs it in one tap (SideStore or AltStore), if any.
struct UpdateInfo: Decodable {
    var version: String
    var installer: String?
}
