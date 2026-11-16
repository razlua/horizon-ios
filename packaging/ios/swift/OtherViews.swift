// The Refresh, Settings and About tabs. Refresh: how long Ferrite keeps opening, and the Apple
// ID that signs it again (crates/ferrite-signer). Settings: the game's own settings files. About:
// the credits, the links and Send logs.
import SwiftUI
import UIKit

// MARK: Refresh

/// How long Ferrite keeps opening: the days left as a big number and as 7 blocks, the date and
/// the reminders.
struct ExpiryCard: View {
    var body: some View {
        HStack(alignment: .center, spacing: 28) {
            if let expiry = Signing.expiry, let days = Signing.daysLeft {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text("\(days)")
                            .font(.system(size: 48, weight: .semibold))
                            .monospacedDigit()
                            .foregroundStyle(days <= 1 ? Theme.bad : Theme.text)
                        Text(days == 1 ? "day left" : "days left")
                            .font(.subheadline)
                            .foregroundStyle(Theme.secondary)
                    }
                    DaysBlocks(block: 16)
                }
                VStack(alignment: .leading, spacing: 8) {
                    Text(days == 0 ? "Ferrite has run out" : "Signed until \(Signing.format(expiry))")
                        .font(.body.weight(.medium))
                        .foregroundStyle(Theme.text)
                    if let team = Signing.team {
                        Text("Signed by \(team)")
                            .font(.subheadline)
                            .foregroundStyle(Theme.secondary)
                    }
                    Text("A reminder comes a day before, and again on the day")
                        .font(.footnote)
                        .foregroundStyle(Theme.muted)
                }
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Ferrite can't tell when it runs out")
                        .font(.body.weight(.medium))
                        .foregroundStyle(Theme.text)
                    Text("This copy wasn't signed by a sideloading tool")
                        .font(.subheadline)
                        .foregroundStyle(Theme.secondary)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(20)
        .card()
    }
}

struct RefreshView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            PageTitle(title: "Refresh")
            ExpiryCard()
            AppleIDCard(model: model)
            Text("Sign in and Check work now. Refresh now, signing and installing Ferrite again over itself, comes in the next builds and also needs a pairing file and LocalDevVPN. Until then, refresh with Sideloadly, AltStore or SideStore.")
                .font(.footnote)
                .foregroundStyle(Theme.muted)
                .padding(.horizontal, 4)
        }
    }
}

/// Swift's side of crates/ferrite-signer: the Apple ID and its password from the secure field,
/// and the 2FA code. They go straight to the signer, never through the actions (which are logged).
@_silgen_name("hz_signer_sign_in")
func hzSignerSignIn(_ email: UnsafePointer<CChar>, _ password: UnsafePointer<CChar>)
@_silgen_name("hz_signer_code")
func hzSignerCode(_ code: UnsafePointer<CChar>)

/// The Apple ID that signs Ferrite again: sign in (with the 2FA code), then Check.
struct AppleIDCard: View {
    @ObservedObject var model: LauncherModel
    @State private var email = ""
    @State private var password = ""
    @State private var code = ""

    var body: some View {
        let signer = model.state.signer
        let status = signer?.status ?? "signed_out"
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Text("Apple ID")
                    .font(.body.weight(.medium))
                    .foregroundStyle(Theme.text)
                Spacer()
                if status == "signed_in" {
                    Chip(text: "Signed in", symbol: "checkmark.circle", color: Theme.good)
                }
            }
            switch status {
            case "signing_in", "working":
                HStack(spacing: 12) {
                    ProgressView().tint(Theme.accent)
                    Text(signer?.message ?? "Signing in to Apple...")
                }
            case "needs_code":
                Text("Apple sent a code to your devices. Type it here.")
                    .foregroundStyle(Theme.secondary)
                if let message = signer?.message {
                    Label(message, systemImage: "exclamationmark.triangle")
                        .font(.footnote)
                        .foregroundStyle(Theme.bad)
                }
                TextField("6-digit code", text: $code)
                    .keyboardType(.numberPad)
                    .font(.title2.monospaced())
                    .fieldStyle()
                Button {
                    feelTap()
                    code.withCString { hzSignerCode($0) }
                    code = ""
                } label: {
                    Text("Submit code")
                        .font(.body.weight(.medium))
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 4)
                }
                .ferriteButton(prominent: true)
                HStack(spacing: 8) {
                    Button {
                        "#resend".withCString { hzSignerCode($0) }
                    } label: {
                        Text("Send again")
                    }
                    .ferriteButton(prominent: false)
                    if let to = signer?.sms_to {
                        Button {
                            "#sms".withCString { hzSignerCode($0) }
                        } label: {
                            Text("Text \(to) instead")
                        }
                        .ferriteButton(prominent: false)
                    }
                }
            case "signed_in":
                Text(signer?.email ?? "")
                    .foregroundStyle(Theme.secondary)
                if let team = signer?.team {
                    Text("Team: \(team)")
                        .font(.footnote)
                        .foregroundStyle(Theme.secondary)
                }
                if let expires = signer?.checked_expires {
                    Label(
                        "Check: Apple would sign Ferrite again until \(Signing.format(Date(timeIntervalSince1970: expires))).",
                        systemImage: "checkmark.circle"
                    )
                    .foregroundStyle(Theme.good)
                }
                HStack(spacing: 8) {
                    Button {
                        feelTap()
                        model.send("signer_check")
                    } label: {
                        Text("Check")
                            .font(.body.weight(.medium))
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 4)
                    }
                    .ferriteButton(prominent: true)
                    Button {
                        model.send("signer_sign_out")
                    } label: {
                        Text("Sign out")
                    }
                    .ferriteButton(prominent: false)
                }
                Text("Check makes Ferrite's signing certificate (the first time) and a new 7-day profile, without installing anything. It never removes another app's certificate.")
                    .font(.footnote)
                    .foregroundStyle(Theme.secondary)
            default:
                if status == "failed", let message = signer?.message {
                    Label(message, systemImage: "exclamationmark.triangle")
                        .font(.footnote)
                        .foregroundStyle(Theme.bad)
                }
                HStack(spacing: 8) {
                    TextField("Apple ID (email)", text: $email)
                        .textContentType(.username)
                        .keyboardType(.emailAddress)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .fieldStyle()
                    SecureField("Password", text: $password)
                        .textContentType(.password)
                        .fieldStyle()
                }
                Button {
                    feelTap()
                    email.withCString { e in password.withCString { p in hzSignerSignIn(e, p) } }
                    password = ""
                } label: {
                    Text("Sign in")
                        .font(.body.weight(.medium))
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 4)
                }
                .ferriteButton(prominent: true)
                Label("Your password goes only to Apple and isn't saved", systemImage: "lock")
                    .font(.footnote)
                    .foregroundStyle(Theme.muted)
            }
        }
        .padding(20)
        .card()
    }
}

// MARK: Settings

struct SettingsView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            PageTitle(title: "Settings")
            if let settings = model.state.settings {
                section(title: "Video") {
                    Stepper(value: Binding(
                        get: { settings.render_distance },
                        set: { model.send("set:render_distance=\($0)") }
                    ), in: 2...64) {
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            Text("Render distance")
                            Text("\(settings.render_distance) chunks")
                                .font(.body.monospacedDigit())
                                .foregroundStyle(Theme.secondary)
                        }
                    }
                    Text("A world starts at 8 and climbs to this while the iPhone has memory for it.")
                        .font(.footnote)
                        .foregroundStyle(Theme.muted)
                    Divider().overlay(Theme.hairline)
                    HStack {
                        Text("Max frame rate")
                        Spacer()
                        Picker("Max frame rate", selection: Binding(
                            get: { settings.max_fps },
                            set: { model.send("set:max_fps=\($0)") }
                        )) {
                            Text("30").tag(30)
                            Text("60").tag(60)
                            Text("120").tag(120)
                        }
                        .pickerStyle(.segmented)
                        .frame(maxWidth: 240)
                    }
                }
                section(title: "Battery") {
                    Toggle(isOn: Binding(
                        get: { settings.battery },
                        set: { model.send("set:battery=\($0)") }
                    )) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Battery mode")
                            Text("60 fps in worlds, 30 in menus, fewer threads")
                                .font(.footnote)
                                .foregroundStyle(Theme.muted)
                        }
                    }
                    .tint(Theme.accent)
                }
                section(title: "Extra settings") {
                    ForEach(Array(settings.extras.enumerated()), id: \.offset) { index, extra in
                        if index > 0 {
                            Divider().overlay(Theme.hairline)
                        }
                        HStack(alignment: .center, spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(extra.label)
                                Text(extra.help).font(.footnote).foregroundStyle(Theme.muted)
                            }
                            Spacer()
                            Button {
                                feelTap(.light)
                                model.send("set:extra=\(index)")
                            } label: {
                                Text("Change")
                                    .font(.subheadline.weight(.medium))
                            }
                            .ferriteButton(prominent: false)
                        }
                    }
                }
                Text("The rest is in the game: Options, then Video Settings")
                    .font(.footnote)
                    .foregroundStyle(Theme.muted)
                    .padding(.horizontal, 4)
            } else {
                HStack {
                    Spacer()
                    ProgressView().tint(Theme.secondary)
                    Spacer()
                }
                .padding(32)
                .card()
            }
        }
    }

    private func section<Content: View>(
        title: String,
        @ViewBuilder _ content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            CardHeading(title)
            content()
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .card()
    }
}

// MARK: About

struct AboutView: View {
    @ObservedObject var model: LauncherModel
    @State private var noLog = false

    var body: some View {
        tabPage {
            HStack(spacing: 20) {
                FerriteLogo(size: 88)
                VStack(alignment: .leading, spacing: 8) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text("Ferrite")
                            .font(.system(size: 32, weight: .semibold))
                            .foregroundStyle(Theme.text)
                        if let version = model.state.version {
                            Text(version)
                                .font(.footnote.monospaced())
                                .foregroundStyle(Theme.muted)
                        }
                    }
                    Text("Minecraft: Java Edition on iPhone. An unofficial, experimental port of Horizon, the Rust rewrite of Java Edition.")
                        .font(.subheadline)
                        .foregroundStyle(Theme.secondary)
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
            .card()
            VStack(alignment: .leading, spacing: 8) {
                CardHeading("Credits")
                credit("Ferrite", "Made by razlua (@4rnz on Discord)")
                credit("Horizon", "By Ludovic Marie and the Horizon contributors, MIT")
                credit("Microsoft sign-in", "Cazza Launcher's Azure application")
                Text("Not affiliated with Horizon, Mojang or Microsoft. Minecraft is a trademark of Mojang Synergies AB.")
                    .font(.footnote)
                    .foregroundStyle(Theme.muted)
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
            .card()
            VStack(alignment: .leading, spacing: 12) {
                CardHeading("Found a bug?")
                Text("Send the log to @4rnz on Discord, with what you did before it went wrong.")
                    .font(.subheadline)
                    .foregroundStyle(Theme.secondary)
                HStack(spacing: 8) {
                    Button {
                        feelTap(.light)
                        noLog = !shareLog()
                    } label: {
                        Label("Send logs", systemImage: "square.and.arrow.up")
                            .font(.body.weight(.medium))
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 4)
                    }
                    .ferriteButton(prominent: true)
                    if let url = URL(string: "https://github.com/razlua/horizon-ios") {
                        Link(destination: url) {
                            Text("GitHub")
                                .font(.body.weight(.medium))
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 4)
                        }
                        .ferriteButton(prominent: false)
                    }
                }
                if noLog {
                    Text("There's no log yet: play once first")
                        .font(.footnote)
                        .foregroundStyle(Theme.muted)
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
            .card()
        }
    }

    private func credit(_ name: String, _ text: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(name).font(.subheadline.weight(.medium))
            Text(text).font(.subheadline).foregroundStyle(Theme.secondary)
        }
    }
}

/// The share sheet with the game's log (Documents/home/logs/latest.log). False without a log.
@discardableResult
func shareLog() -> Bool {
    guard let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
    else { return false }
    let log = documents.appendingPathComponent("home/logs/latest.log")
    guard FileManager.default.fileExists(atPath: log.path), let top = topController() else {
        return false
    }
    let sheet = UIActivityViewController(activityItems: [log], applicationActivities: nil)
    sheet.popoverPresentationController?.sourceView = top.view
    top.present(sheet, animated: true)
    return true
}
