// The Refresh, Settings and About tabs. Refresh and Settings fill in over the next builds
// (refreshing Ferrite with your Apple ID, the settings the game keeps); About has the credits,
// the links and Send logs.
import SwiftUI
import UIKit

private func tabPage<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
    ZStack {
        Theme.background.ignoresSafeArea()
        ScrollView {
            VStack(alignment: .leading, spacing: 16, content: content)
                .padding(24)
                .frame(maxWidth: 680, alignment: .leading)
                .frame(maxWidth: .infinity)
        }
    }
}

func infoCard<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
    VStack(alignment: .leading, spacing: 10, content: content)
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.card, in: RoundedRectangle(cornerRadius: 16))
}

/// How long Ferrite keeps opening, colored by how soon it runs out.
struct ExpiryCard: View {
    var body: some View {
        infoCard {
            if let expiry = Signing.expiry, let days = Signing.daysLeft {
                let color: Color = days <= 1 ? .red : (days <= 2 ? Theme.accent : .green)
                Label(days == 0 ? "Ferrite has run out" : "Signed until \(Signing.format(expiry))",
                      systemImage: days <= 2 ? "exclamationmark.triangle.fill" : "checkmark.seal.fill")
                    .font(.headline)
                    .foregroundStyle(color)
                Text(days == 1 ? "1 day left." : "\(days) days left.")
                    .foregroundStyle(.secondary)
                if let team = Signing.team {
                    Text("Signed by \(team).").font(.footnote).foregroundStyle(.secondary)
                }
                Text("Ferrite reminds you a day before, and again on the day, if you allow its notifications.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                Label("Ferrite can't tell when it runs out", systemImage: "questionmark.circle")
                    .font(.headline)
                Text("This copy wasn't signed by a sideloading tool.").foregroundStyle(.secondary)
            }
        }
    }
}

struct RefreshView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            Text("Refresh").font(.system(size: 34, weight: .bold, design: .rounded))
            ExpiryCard()
            AppleIDCard(model: model)
            infoCard {
                Label("Refreshing from inside Ferrite", systemImage: "arrow.clockwise.circle.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                Text("Sign in and Check work now. Refresh now (signing and installing Ferrite again over itself) comes in the next builds: it also needs a pairing file and LocalDevVPN. Until then, refresh with Sideloadly, AltStore or SideStore.")
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct SettingsView: View {
    @ObservedObject var model: LauncherModel

    var body: some View {
        tabPage {
            Text("Settings").font(.system(size: 34, weight: .bold, design: .rounded))
            if let settings = model.state.settings {
                infoCard {
                    Text("Video").font(.headline)
                    Stepper(value: Binding(
                        get: { settings.render_distance },
                        set: { model.send("set:render_distance=\($0)") }
                    ), in: 2...64) {
                        Text("Render distance: \(settings.render_distance) chunks")
                    }
                    Text("Ferrite starts a world at 8 and climbs to this while the iPhone has memory for it.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                    Picker("Max frame rate", selection: Binding(
                        get: { settings.max_fps },
                        set: { model.send("set:max_fps=\($0)") }
                    )) {
                        Text("30").tag(30)
                        Text("60").tag(60)
                        Text("120").tag(120)
                    }
                    .pickerStyle(.segmented)
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
                }
                infoCard {
                    Text("Extra Settings").font(.headline)
                    ForEach(Array(settings.extras.enumerated()), id: \.offset) { index, extra in
                        HStack(alignment: .center, spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(extra.label)
                                Text(extra.help).font(.footnote).foregroundStyle(.secondary)
                            }
                            Spacer()
                            Button("Change") { model.send("set:extra=\(index)") }
                                .buttonStyle(.bordered)
                        }
                    }
                }
                Text("The rest is in the game: Options, then Video Settings.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                infoCard {
                    ProgressView()
                }
            }
        }
    }
}

struct AboutView: View {
    @ObservedObject var model: LauncherModel
    @State private var noLog = false

    var body: some View {
        tabPage {
            Text("About").font(.system(size: 34, weight: .bold, design: .rounded))
            infoCard {
                Text("Ferrite").font(.title2.bold())
                if let version = model.state.version {
                    Text("Version \(version)").foregroundStyle(.secondary)
                }
                Text("An unofficial, experimental iPhone port of Horizon, the Rust rewrite of Minecraft: Java Edition.")
                    .foregroundStyle(.secondary)
                Text("Made by razlua (@4rnz on Discord). Horizon is by Ludovic Marie and the Horizon contributors (MIT).")
                    .foregroundStyle(.secondary)
                Text("Not affiliated with Horizon, Mojang or Microsoft.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            infoCard {
                Text("Found a bug?").font(.headline)
                Text("Send the log to @4rnz on Discord, with what you did before it went wrong.")
                    .foregroundStyle(.secondary)
                HStack {
                    Button {
                        noLog = !shareLog()
                    } label: {
                        Label("Send logs", systemImage: "square.and.arrow.up")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    if let url = URL(string: "https://github.com/razlua/horizon-ios") {
                        Link(destination: url) {
                            Label("GitHub", systemImage: "link").frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                    }
                }
                if noLog {
                    Text("There's no log yet: play once first.").font(.footnote).foregroundStyle(.secondary)
                }
            }
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
        infoCard {
            Text("Apple ID").font(.headline)
            switch status {
            case "signing_in", "working":
                HStack(spacing: 12) {
                    ProgressView()
                    Text(signer?.message ?? "Signing in to Apple...")
                }
            case "needs_code":
                Text("Apple sent a code to your devices. Type it here.").foregroundStyle(.secondary)
                if let message = signer?.message {
                    Text(message).font(.footnote).foregroundStyle(.red)
                }
                TextField("6-digit code", text: $code)
                    .keyboardType(.numberPad)
                    .font(.title2.monospaced())
                    .padding(10)
                    .background(Color.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
                Button {
                    code.withCString { hzSignerCode($0) }
                    code = ""
                } label: {
                    Text("Submit code").font(.headline).foregroundStyle(.black).frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                HStack {
                    Button("Send again") { "#resend".withCString { hzSignerCode($0) } }
                        .buttonStyle(.bordered)
                    if let to = signer?.sms_to {
                        Button("Text \(to) instead") { "#sms".withCString { hzSignerCode($0) } }
                            .buttonStyle(.bordered)
                    }
                }
            case "signed_in":
                Label("Signed in as \(signer?.email ?? "")", systemImage: "person.crop.circle.badge.checkmark")
                if let team = signer?.team {
                    Text("Team: \(team)").font(.footnote).foregroundStyle(.secondary)
                }
                if let expires = signer?.checked_expires {
                    Text("Check: Apple would sign Ferrite again until \(Signing.format(Date(timeIntervalSince1970: expires))).")
                        .foregroundStyle(.green)
                }
                HStack {
                    Button {
                        model.send("signer_check")
                    } label: {
                        Text("Check").font(.headline).foregroundStyle(.black).frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    Button("Sign out") { model.send("signer_sign_out") }
                        .buttonStyle(.bordered)
                }
                Text("Check makes Ferrite's signing certificate (the first time) and a new 7-day profile, without installing anything. It never removes another app's certificate.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            default:
                if status == "failed", let message = signer?.message {
                    Label(message, systemImage: "exclamationmark.triangle.fill").foregroundStyle(.red)
                }
                Text("Your password goes only to Apple and isn't saved.").font(.footnote).foregroundStyle(.secondary)
                TextField("Apple ID (email)", text: $email)
                    .textContentType(.username)
                    .keyboardType(.emailAddress)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .padding(10)
                    .background(Color.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
                SecureField("Password", text: $password)
                    .textContentType(.password)
                    .padding(10)
                    .background(Color.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
                Button {
                    email.withCString { e in password.withCString { p in hzSignerSignIn(e, p) } }
                    password = ""
                } label: {
                    Text("Sign in").font(.headline).foregroundStyle(.black).frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
            }
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
