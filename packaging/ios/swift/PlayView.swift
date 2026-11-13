// The Play tab: the Microsoft accounts, the sign-in, the game files' download and the update
// notice, as the game's own start screen has them. Every button sends the game an action.
import SwiftUI
import UIKit

struct PlayView: View {
    @ObservedObject var model: LauncherModel
    @State private var offlineName = ""

    private var state: LauncherState { model.state }

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    HStack(alignment: .firstTextBaseline) {
                        Text("Ferrite")
                            .font(.system(size: 34, weight: .bold, design: .rounded))
                        Spacer()
                        if let version = state.version {
                            Text("Version \(version)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    expiryBanner
                    content
                    if let unpacking = state.unpacking, unpacking.total > 0 {
                        ProgressView(value: Double(unpacking.done), total: Double(unpacking.total)) {
                            Text("Unpacking your game files. This first start takes a minute.")
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .padding(24)
                .frame(maxWidth: 680, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
        }
    }

    /// The download takes the screen while it runs, or when it can't start yet.
    @ViewBuilder private var content: some View {
        let setup = state.setup?.kind ?? "not_needed"
        let account = state.account?.kind ?? "asking"
        if (setup == "running" || setup == "waiting") && account == "signed" {
            downloading
        } else if setup == "failed" {
            card {
                Text("The download stopped").font(.title3.bold())
                Text(state.setup?.error ?? "").foregroundStyle(.secondary)
                Text("Try again goes on where it stopped.").foregroundStyle(.secondary)
                primary("Try again") { model.send("retry_setup") }
            }
        } else if setup == "waiting" && account == "offline" {
            card {
                Text("Sign in once first").font(.title3.bold())
                Text("Ferrite downloads the game files from Mojang with a Microsoft account that owns Minecraft: Java Edition. Sign in once; after that you can play offline too.")
                    .foregroundStyle(.secondary)
                primary("Sign in") { model.send("back") }
            }
        } else {
            accountView
        }
    }

    private var downloading: some View {
        card {
            Text("Getting the game files").font(.title3.bold())
            Text("Ferrite downloads your Minecraft 26.3 from Mojang (about 520 MB) and its own tables, and sets them up. This happens once. Keep Ferrite open, on Wi-Fi.")
                .foregroundStyle(.secondary)
            let done = state.setup?.done ?? 0
            let total = state.setup?.total ?? 0
            let stage = state.setup?.stage ?? ""
            if total > 0 {
                ProgressView(value: Double(done), total: Double(total)) {
                    Text("\(stage): \(done / 1_000_000) of \(total / 1_000_000) MB")
                        .font(.footnote)
                }
            } else {
                ProgressView { Text(stage.isEmpty ? "Starting..." : "\(stage)...").font(.footnote) }
            }
        }
    }

    @ViewBuilder private var accountView: some View {
        let account = state.account
        let accounts = state.accounts ?? []
        switch account?.kind ?? "asking" {
        case "choose":
            if account?.removing == true {
                card {
                    Text("Remove an account").font(.title3.bold())
                    Text("Tap an account to remove it from this iPhone.").foregroundStyle(.secondary)
                    ForEach(Array(accounts.enumerated()), id: \.offset) { index, name in
                        Button(role: .destructive) { model.send("remove:\(index)") } label: {
                            Label("Remove \(name)", systemImage: "trash").frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                    }
                    secondary("Done") { model.send("remove_mode") }
                }
            } else {
                card {
                    Text("Choose an account").font(.title3.bold())
                    Text("Play as one of your Microsoft accounts, or add another.")
                        .foregroundStyle(.secondary)
                    ForEach(Array(accounts.enumerated()), id: \.offset) { index, name in
                        if index == 0 {
                            primary("Play as \(name)") { model.send("play:\(index)") }
                        } else {
                            secondary("Play as \(name)") { model.send("play:\(index)") }
                        }
                    }
                    HStack {
                        secondary("Add account") { model.send("add_account") }
                        secondary("Remove...") { model.send("remove_mode") }
                        secondary("Play offline") { model.send("offline") }
                    }
                }
                updateBanner
            }
        case "resuming":
            card {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Signing in as \(account?.name ?? "")...")
                }
                secondary("Back") { model.send("back") }
            }
        case "code":
            card {
                Text("Sign in with Microsoft").font(.title3.bold())
                Text("Tap Open sign-in page and sign in with the Microsoft account that owns Minecraft: Java Edition, then come back here.")
                    .foregroundStyle(.secondary)
                if let code = account?.code {
                    HStack(spacing: 16) {
                        Text(code)
                            .font(.system(size: 38, weight: .bold, design: .monospaced))
                            .textSelection(.enabled)
                        Button {
                            UIPasteboard.general.string = code
                        } label: {
                            Label("Copy", systemImage: "doc.on.doc")
                        }
                        .buttonStyle(.bordered)
                    }
                }
                primary("Open sign-in page") {
                    if let link = account?.link, let url = URL(string: link) {
                        UIApplication.shared.open(url)
                    }
                }
                if let uri = account?.uri {
                    Text("Or open \(uri) on any device and enter the code.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Waiting for you to sign in...").font(.footnote).foregroundStyle(.secondary)
                }
                HStack {
                    if !accounts.isEmpty { secondary("Back") { model.send("back") } }
                    secondary("Play offline") { model.send("offline") }
                }
            }
        case "signed":
            card {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Signed in as \(account?.name ?? ""). Starting...")
                }
            }
        case "failed":
            card {
                Text("Sign-in did not work").font(.title3.bold())
                Text(account?.error ?? "").foregroundStyle(.secondary)
                primary("Try again") { model.send("retry") }
                HStack {
                    if !accounts.isEmpty { secondary("Back") { model.send("back") } }
                    secondary("Play offline") { model.send("offline") }
                }
            }
        case "offline_name":
            card {
                Text("Play offline").font(.title3.bold())
                Text("Your name in single player and on offline-mode servers: 3 to 16 letters, numbers or _.")
                    .foregroundStyle(.secondary)
                TextField("Name", text: $offlineName)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.title3.monospaced())
                    .padding(10)
                    .background(Color.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
                    .onAppear { offlineName = account?.name ?? "" }
                if let problem = account?.problem {
                    Text(problem).font(.footnote).foregroundStyle(.red)
                }
                primary("Play") { model.send("offline_name:\(offlineName)") }
                secondary("Back") { model.send("back") }
            }
        case "offline":
            card {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Playing offline. Starting...")
                }
            }
        default:
            card {
                HStack(spacing: 12) {
                    ProgressView()
                    Text("Connecting to Microsoft...")
                }
                HStack {
                    if !accounts.isEmpty { secondary("Back") { model.send("back") } }
                    secondary("Play offline") { model.send("offline") }
                }
            }
        }
    }

    /// Two days or less before Ferrite stops opening: a warning above everything.
    @ViewBuilder private var expiryBanner: some View {
        if let expiry = Signing.expiry, let days = Signing.daysLeft, days <= 2 {
            card {
                Label(days == 0 ? "Ferrite has run out" : "Refresh before \(Signing.format(expiry))",
                      systemImage: "exclamationmark.triangle.fill")
                    .font(.headline)
                    .foregroundStyle(days <= 1 ? Color.red : Theme.accent)
                Text("Sign it again with Sideloadly, AltStore or SideStore, or it won't open after that. Your worlds stay.")
                    .foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder private var updateBanner: some View {
        if let update = state.update {
            card {
                Label("Version \(update.version) is out", systemImage: "arrow.down.circle.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                if let installer = update.installer {
                    Text("\(installer) installs it over this app; your worlds and sign-ins stay.")
                        .foregroundStyle(.secondary)
                    primary("Update with \(installer)") { model.send("get_update") }
                } else {
                    Text("Download it and sideload it over this app; your worlds and sign-ins stay.")
                        .foregroundStyle(.secondary)
                    primary("Get the update") { model.send("get_update") }
                }
            }
        }
    }

    private func card<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 12, content: content)
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.card, in: RoundedRectangle(cornerRadius: 16))
    }

    private func primary(_ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(.headline).foregroundStyle(.black).frame(maxWidth: .infinity)
                .padding(.vertical, 4)
        }
        .buttonStyle(.borderedProminent)
    }

    private func secondary(_ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).frame(maxWidth: .infinity)
        }
        .buttonStyle(.bordered)
    }
}
