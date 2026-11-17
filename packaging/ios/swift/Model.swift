// What the game sends the launcher (crates/horizon-client/src/game/ios_boot.rs, `launcher_state`):
// the same state its own start screen draws. Every field is optional, so an older or newer game
// still decodes.
import Foundation

struct LauncherState: Decodable {
    var version: String?
    var account: AccountState?
    var accounts: [AccountInfo]?
    var setup: SetupState?
    var unpacking: Counts?
    var update: UpdateInfo?
    var settings: SettingsState?
    var signer: SignerState?
    var skin: SkinUploadState?
    /// A pairing file is in Files > Ferrite > pairing (Refresh now needs one).
    var pairing: Bool?
    /// The last run ended while playing (crates/horizon-client/src/game/crash_watch.rs).
    var last_crash: LastCrash?
    /// Profiles (crates/horizon-client/src/game/ios_instances.rs).
    var instances: InstancesState?
}

/// The profiles there are, and the chosen one (none: Default).
struct InstancesState: Decodable {
    var list: [String]
    var current: String?
}

/// What the last run was doing when it ended while playing.
struct LastCrash: Decodable {
    var time: Double?
    var in_ram_mb: Double?
    var left_mb: Double?
    var distance: Int?
    var chosen_distance: Int?
    var `where`: String?
}

/// A saved Microsoft account: its Minecraft name and uuid (without dashes), for its skin.
struct AccountInfo: Decodable {
    var name: String
    var uuid: String
}

/// The skin changer's last upload. `status`: working, done or failed (with `message`).
struct SkinUploadState: Decodable {
    var status: String
    var message: String?
}

/// Signing Ferrite again with an Apple ID (crates/ferrite-signer). `status`: signed_out,
/// signing_in, needs_code, working, signed_in or failed.
struct SignerState: Decodable {
    var status: String
    var message: String?
    var email: String?
    var team: String?
    var sms_to: String?
    var checked_bundle_id: String?
    var checked_expires: Double?
    /// How far Refresh now is, 0 to 1, while it runs.
    var progress: Double?
    /// The iPhone's name, once Ferrite reached its installer.
    var device: String?
    /// How the last Test connection or Refresh now ended.
    var refresh_ok: Bool?
    var refresh_note: String?
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
