// The Refresh, Settings and About tabs. Refresh: how long Ferrite keeps opening, and the Apple
// ID that signs it again (crates/ferrite-signer). Settings: the game's own settings files. About:
// the credits, the links and Send logs.
import SwiftUI
import UIKit

// MARK: Refresh

/// The days left of the signature's 7, as a ring.
struct ExpiryRing: View {
    var size: CGFloat = 116

    var body: some View {
        let days = Signing.daysLeft
        let fraction = days.map { min(1, Double($0) / 7) } ?? 0
        let color: Color = days.map { $0 <= 1 ? Color.red : ($0 <= 2 ? Theme.accent : Color.green) } ?? Color.gray
        ZStack {
            Circle()
                .stroke(Color.white.opacity(0.08), lineWidth: 11)
            Circle()
                .trim(from: 0, to: fraction)
                .stroke(color, style: StrokeStyle(lineWidth: 11, lineCap: .round))
                .rotationEffect(.degrees(-90))
                .shadow(color: color.opacity(0.5), radius: 6)
            VStack(spacing: 0) {
                Text(days.map { "\($0)" } ?? "?")
                    .font(.system(size: 38, weight: .heavy, design: .rounded))
                    .monospacedDigit()
                Text(days == 1 ? "day left" : "days left")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: size, height: size)
    }
}

/// How long Ferrite keeps opening: the ring, the date and the reminders.
struct ExpiryCard: View {
    var body: some View {
        HStack(spacing: 22) {
            ExpiryRing()
            VStack(alignment: .leading, spacing: 6) {
                if let expiry = Signing.expiry, let days = Signing.daysLeft {
                    Text(days == 0 ? "Ferrite has run out" : "Signed until \(Signing.format(expiry))")
                        .font(.title3.weight(.bold))
                    if let team = Signing.team {
                        Label("Signed by \(team)", systemImage: "person.badge.key.fill")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    Label("A reminder comes a day before, and again on the day.", systemImage: "bell.badge.fill")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                } else {
                    Text("Ferrite can't tell when it runs out")
                        .font(.title3.weight(.bold))
                    Text("This copy wasn't signed by a sideloading tool.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(20)
        .glassCard(cornerRadius: 28)
    }
}

struct RefreshView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            PageTitle(title: "Refresh", symbol: "arrow.clockwise.circle.fill")
            ExpiryCard()
            AppleIDCard(model: model)
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: "sparkles")
                    .foregroundStyle(Theme.accent)
                Text("Sign in and Check work now. Refresh now, signing and installing Ferrite again over itself, comes in the next builds: it also needs a pairing file and LocalDevVPN. Until then, refresh with Sideloadly, AltStore or SideStore.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            .padding(16)
            .glassCard(cornerRadius: 22)
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
            HStack(spacing: 10) {
                Image(systemName: "person.crop.circle.fill")
                    .font(.title2)
                    .foregroundStyle(Theme.accent)
                Text("Apple ID").font(.title3.weight(.bold))
                Spacer()
                if status == "signed_in" {
                    Chip(text: "Signed in", symbol: "checkmark.circle.fill", color: Color.green)
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
                    .foregroundStyle(.secondary)
                if let message = signer?.message {
                    Label(message, systemImage: "exclamationmark.circle.fill")
                        .font(.footnote)
                        .foregroundStyle(Color.red)
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
                    Label("Submit code", systemImage: "checkmark.shield.fill")
                        .font(.headline)
                        .foregroundStyle(Color.black)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 4)
                }
                .ferriteButton(prominent: true)
                HStack(spacing: 10) {
                    Button {
                        "#resend".withCString { hzSignerCode($0) }
                    } label: {
                        Label("Send again", systemImage: "arrow.clockwise")
                    }
                    .ferriteButton(prominent: false)
                    if let to = signer?.sms_to {
                        Button {
                            "#sms".withCString { hzSignerCode($0) }
                        } label: {
                            Label("Text \(to) instead", systemImage: "message.fill")
                        }
                        .ferriteButton(prominent: false)
                    }
                }
            case "signed_in":
                Label(signer?.email ?? "", systemImage: "envelope.fill")
                    .foregroundStyle(.secondary)
                if let team = signer?.team {
                    Label("Team: \(team)", systemImage: "person.2.fill")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
                if let expires = signer?.checked_expires {
                    Label(
                        "Check: Apple would sign Ferrite again until \(Signing.format(Date(timeIntervalSince1970: expires))).",
                        systemImage: "checkmark.seal.fill"
                    )
                    .foregroundStyle(Color.green)
                }
                HStack(spacing: 10) {
                    Button {
                        feelTap()
                        model.send("signer_check")
                    } label: {
                        Label("Check", systemImage: "checkmark.shield.fill")
                            .font(.headline)
                            .foregroundStyle(Color.black)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 4)
                    }
                    .ferriteButton(prominent: true)
                    Button {
                        model.send("signer_sign_out")
                    } label: {
                        Label("Sign out", systemImage: "rectangle.portrait.and.arrow.right")
                    }
                    .ferriteButton(prominent: false)
                }
                Text("Check makes Ferrite's signing certificate (the first time) and a new 7-day profile, without installing anything. It never removes another app's certificate.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            default:
                if status == "failed", let message = signer?.message {
                    Label(message, systemImage: "exclamationmark.triangle.fill")
                        .font(.footnote)
                        .foregroundStyle(Color.red)
                }
                HStack(spacing: 10) {
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
                    Label("Sign in", systemImage: "person.crop.circle.badge.checkmark")
                        .font(.headline)
                        .foregroundStyle(Color.black)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 4)
                }
                .ferriteButton(prominent: true)
                Label("Your password goes only to Apple and isn't saved.", systemImage: "lock.fill")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(20)
        .glassCard(cornerRadius: 28)
    }
}

// MARK: Settings

struct SettingsView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            PageTitle(title: "Settings", symbol: "gearshape.fill")
            if let settings = model.state.settings {
                section(title: "Video", symbol: "display") {
                    Stepper(value: Binding(
                        get: { settings.render_distance },
                        set: { model.send("set:render_distance=\($0)") }
                    ), in: 2...64) {
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text("Render distance")
                            Text("\(settings.render_distance) chunks")
                                .foregroundStyle(Theme.accent)
                                .monospacedDigit()
                        }
                    }
                    Text("Ferrite starts a world at 8 and climbs to this while the iPhone has memory for it.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Divider().opacity(0.4)
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
                section(title: "Battery", symbol: "battery.100.bolt") {
                    Toggle(isOn: Binding(
                        get: { settings.battery },
                        set: { model.send("set:battery=\($0)") }
                    )) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Battery mode")
                            Text("60 fps in worlds, 30 in menus, fewer threads.")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .tint(Theme.accent)
                }
                section(title: "Extra Settings", symbol: "slider.horizontal.3") {
                    ForEach(Array(settings.extras.enumerated()), id: \.offset) { index, extra in
                        if index > 0 {
                            Divider().opacity(0.4)
                        }
                        HStack(alignment: .center, spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(extra.label)
                                Text(extra.help).font(.footnote).foregroundStyle(.secondary)
                            }
                            Spacer()
                            Button {
                                feelTap(.light)
                                model.send("set:extra=\(index)")
                            } label: {
                                Label("Change", systemImage: "arrow.triangle.2.circlepath")
                                    .font(.subheadline.weight(.semibold))
                            }
                            .ferriteButton(prominent: false)
                        }
                    }
                }
                Label("The rest is in the game: Options, then Video Settings.", systemImage: "info.circle")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                HStack {
                    Spacer()
                    ProgressView().tint(Theme.accent)
                    Spacer()
                }
                .padding(30)
                .glassCard(cornerRadius: 28)
            }
        }
    }

    private func section<Content: View>(
        title: String,
        symbol: String,
        @ViewBuilder _ content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(title, systemImage: symbol)
                .font(.headline)
                .foregroundStyle(Theme.accent)
            content()
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .glassCard(cornerRadius: 26)
    }
}

// MARK: About

struct AboutView: View {
    @ObservedObject var model: LauncherModel
    @State private var noLog = false

    var body: some View {
        tabPage {
            HStack(spacing: 22) {
                FerriteLogo(size: 104)
                    .shadow(color: Theme.accent.opacity(0.4), radius: 18, x: 0, y: 6)
                VStack(alignment: .leading, spacing: 6) {
                    Text("Ferrite")
                        .font(.system(size: 40, weight: .heavy, design: .rounded))
                    if let version = model.state.version {
                        Chip(text: "Version \(version)", symbol: "shippingbox.fill")
                    }
                    Text("Minecraft: Java Edition on iPhone. An unofficial, experimental port of Horizon, the Rust rewrite of Java Edition.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
            .padding(22)
            .glassCard(cornerRadius: 30)
            VStack(alignment: .leading, spacing: 10) {
                Label("Credits", systemImage: "heart.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                credit("Ferrite", "Made by razlua (@4rnz on Discord).")
                credit("Horizon", "By Ludovic Marie and the Horizon contributors, MIT.")
                credit("Microsoft sign-in", "Cazza Launcher's Azure application.")
                Text("Not affiliated with Horizon, Mojang or Microsoft. Minecraft is a trademark of Mojang Synergies AB.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
            .glassCard(cornerRadius: 26)
            VStack(alignment: .leading, spacing: 12) {
                Label("Found a bug?", systemImage: "ladybug.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                Text("Send the log to @4rnz on Discord, with what you did before it went wrong.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                HStack(spacing: 10) {
                    Button {
                        feelTap(.light)
                        noLog = !shareLog()
                    } label: {
                        Label("Send logs", systemImage: "square.and.arrow.up")
                            .font(.headline)
                            .foregroundStyle(Color.black)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 4)
                    }
                    .ferriteButton(prominent: true)
                    if let url = URL(string: "https://github.com/razlua/horizon-ios") {
                        Link(destination: url) {
                            Label("GitHub", systemImage: "chevron.left.forwardslash.chevron.right")
                                .font(.headline)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 4)
                        }
                        .ferriteButton(prominent: false)
                    }
                }
                if noLog {
                    Text("There's no log yet: play once first.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
            .glassCard(cornerRadius: 26)
        }
    }

    private func credit(_ name: String, _ text: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(name).font(.subheadline.weight(.bold))
            Text(text).font(.subheadline).foregroundStyle(.secondary)
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
