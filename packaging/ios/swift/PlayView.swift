// The Play tab: the player on the left, Play and the accounts on the right; the sign-in, the
// game files' download and the rest as one glass card. Every button sends the game an action
// (crates/horizon-client/src/ios_screen.rs, `parse_launcher_action`).
import SwiftUI
import UIKit

struct PlayView: View {
    @ObservedObject var model: LauncherModel
    @State private var selected = 0
    @State private var offlineName = ""
    @State private var changingSkin = false
    @State private var copied = false

    private var state: LauncherState { model.state }
    private var accounts: [AccountInfo] { state.accounts ?? [] }

    /// What the tab shows, for its animation between them.
    private var phase: String {
        "\(state.account?.kind ?? "")/\(state.setup?.kind ?? "")/\(state.account?.removing == true)"
    }

    var body: some View {
        ZStack {
            LauncherBackground()
            content
                .padding(.horizontal, 28)
                .padding(.top, 16)
                .padding(.bottom, 8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .transition(.opacity)
        }
        .animation(.easeOut(duration: 0.25), value: phase)
    }

    @ViewBuilder private var content: some View {
        let setup = state.setup?.kind ?? "not_needed"
        let kind = state.account?.kind ?? "asking"
        if (setup == "running" || setup == "waiting") && kind == "signed" {
            centered { downloading }
        } else if setup == "failed" {
            centered { setupFailed }
        } else if setup == "waiting" && kind == "offline" {
            centered { signInFirst }
        } else if kind == "choose" && state.account?.removing != true && !accounts.isEmpty {
            home
        } else {
            centered { accountCard }
        }
    }

    // MARK: Home: the player and Play

    private var home: some View {
        let index = max(0, min(selected, accounts.count - 1))
        let account = accounts[index]
        return HStack(alignment: .center, spacing: 24) {
            // The one rich thing on the screen: the player, in 3D.
            VStack(spacing: 8) {
                SkinFigure(uuid: account.uuid, height: 212)
                Text(account.name)
                    .font(.body.weight(.medium))
                    .foregroundStyle(Theme.text)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                Button {
                    changingSkin = true
                } label: {
                    Text("Change skin")
                        .font(.subheadline.weight(.medium))
                }
                .ferriteButton(prominent: false)
            }
            .frame(width: 196)
            .padding(.vertical, 16)
            .padding(.horizontal, 12)
            .card()

            VStack(alignment: .leading, spacing: 12) {
                header(subtitle: nil)
                Button {
                    feelTap()
                    model.send("play:\(index)")
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "play.fill")
                        Text("Play as \(account.name)")
                            .lineLimit(1)
                            .minimumScaleFactor(0.7)
                    }
                    .font(.title3.weight(.semibold))
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
                .ferriteButton(prominent: true)
                expiryNote
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(Array(accounts.enumerated()), id: \.offset) { i, other in
                            accountChip(other, at: i, selected: i == index)
                        }
                        Button {
                            model.send("add_account")
                        } label: {
                            Label("Add account", systemImage: "plus")
                                .font(.subheadline.weight(.medium))
                                .foregroundStyle(Theme.secondary)
                                .frame(height: 40)
                                .padding(.horizontal, 12)
                                .tile()
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(.vertical, 2)
                }
                HStack(spacing: 16) {
                    Menu {
                        Button {
                            model.send("offline")
                        } label: {
                            Label("Play offline", systemImage: "wifi.slash")
                        }
                        Button(role: .destructive) {
                            selected = 0
                            model.send("remove_mode")
                        } label: {
                            Label("Remove an account", systemImage: "person.crop.circle.badge.minus")
                        }
                    } label: {
                        Label("More", systemImage: "ellipsis")
                            .font(.footnote.weight(.medium))
                            .foregroundStyle(Theme.secondary)
                    }
                    updateBanner
                    Spacer(minLength: 0)
                }
                unpackingBar
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .sheet(isPresented: $changingSkin) {
            SkinChanger(model: model, index: index, account: account)
        }
    }

    private func accountChip(_ account: AccountInfo, at i: Int, selected isSelected: Bool) -> some View {
        Button {
            feelTap(.light)
            selected = i
        } label: {
            HStack(spacing: 8) {
                HeadAvatar(uuid: account.uuid, size: 24)
                Text(account.name)
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(isSelected ? Theme.text : Theme.secondary)
                    .lineLimit(1)
            }
            .frame(height: 40)
            .padding(.leading, 8)
            .padding(.trailing, 12)
            .tile(selected: isSelected)
        }
        .buttonStyle(.plain)
        .contextMenu {
            Button {
                feelTap(.light)
                selected = i
            } label: {
                Label("Choose \(account.name)", systemImage: "checkmark.circle")
            }
            Button(role: .destructive) {
                selected = 0
                model.send("remove:\(i)")
            } label: {
                Label("Remove \(account.name)", systemImage: "trash")
            }
        }
    }

    /// The logo and the name (with a line under it on the sign-in screens), then the days left
    /// as blocks and the version, quiet.
    private func header(subtitle: String?) -> some View {
        HStack(alignment: .center, spacing: 12) {
            FerriteLogo(size: 36)
            VStack(alignment: .leading, spacing: 2) {
                Text("Ferrite")
                    .font(.system(size: 22, weight: .semibold))
                    .foregroundStyle(Theme.text)
                if let subtitle {
                    Text(subtitle)
                        .font(.subheadline)
                        .foregroundStyle(Theme.secondary)
                        .lineLimit(1)
                }
            }
            Spacer(minLength: 8)
            if let days = Signing.daysLeft {
                HStack(spacing: 8) {
                    DaysBlocks(block: 8)
                    Text(daysLeftText(days))
                        .font(.footnote.weight(.medium))
                        .monospacedDigit()
                        .foregroundStyle(days <= 1 ? Theme.bad : Theme.secondary)
                }
            }
            if let version = state.version {
                Text(version)
                    .font(.footnote.monospaced())
                    .foregroundStyle(Theme.muted)
            }
        }
    }

    /// Two days or less before Ferrite stops opening: when, under Play.
    @ViewBuilder private var expiryNote: some View {
        if let expiry = Signing.expiry, let days = Signing.daysLeft, days <= 2 {
            Label(
                days == 0 ? "Ferrite has run out: sign it again to keep playing" : "Refresh before \(Signing.format(expiry)), or Ferrite won't open",
                systemImage: "clock"
            )
            .font(.footnote.weight(.medium))
            .foregroundStyle(days <= 1 ? Theme.bad : Theme.accent)
        }
    }

    @ViewBuilder private var updateBanner: some View {
        if let update = state.update {
            Button {
                feelTap(.light)
                model.send("get_update")
            } label: {
                Label(
                    update.installer.map { "Update to \(update.version) with \($0)" } ?? "Get version \(update.version)",
                    systemImage: "arrow.down.circle"
                )
                .font(.footnote.weight(.medium))
                .foregroundStyle(Theme.accent)
                .lineLimit(1)
            }
            .buttonStyle(.plain)
        }
    }

    @ViewBuilder private var unpackingBar: some View {
        if let unpacking = state.unpacking, unpacking.total > 0 {
            ProgressView(value: Double(unpacking.done), total: Double(unpacking.total)) {
                Text("Unpacking your game files. The first start takes a minute.")
                    .font(.footnote)
                    .foregroundStyle(Theme.secondary)
            }
            .tint(Theme.accent)
        }
    }

    // MARK: One card: the sign-in, the download and the rest

    private func centered<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header(subtitle: "Minecraft: Java Edition on iPhone")
                VStack(alignment: .leading, spacing: 12, content: content)
                    .padding(20)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .card(cornerRadius: 16)
                unpackingBar
            }
            .frame(maxWidth: 640)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
        }
    }

    /// A card's title. Only a problem gets a symbol (a warning in its color); the rest is text.
    private func cardTitle(_ title: String, symbol: String, color: Color = Theme.secondary) -> some View {
        HStack(spacing: 8) {
            if color == Theme.bad {
                Image(systemName: "exclamationmark.triangle").foregroundStyle(color)
            }
            Text(title).foregroundStyle(Theme.text)
        }
        .font(.title3.weight(.medium))
    }

    /// The symbols that say something on a button: Play's triangle and Back's chevron.
    private static let meaningful: Set<String> = ["play.fill", "chevron.left"]

    @ViewBuilder
    private func buttonLabel(_ title: String, symbol: String) -> some View {
        if PlayView.meaningful.contains(symbol) {
            Label(title, systemImage: symbol)
        } else {
            Text(title)
        }
    }

    private func primary(_ title: String, symbol: String, action: @escaping () -> Void) -> some View {
        Button {
            feelTap()
            action()
        } label: {
            buttonLabel(title, symbol: symbol)
                .font(.body.weight(.medium))
                .frame(maxWidth: .infinity)
                .padding(.vertical, 4)
        }
        .ferriteButton(prominent: true)
    }

    private func secondary(_ title: String, symbol: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            buttonLabel(title, symbol: symbol)
                .font(.subheadline.weight(.medium))
                .frame(maxWidth: .infinity)
        }
        .ferriteButton(prominent: false)
    }

    private func waiting(_ text: String) -> some View {
        HStack(spacing: 12) {
            ProgressView().tint(Theme.accent)
            Text(text).font(.body.weight(.medium))
        }
    }

    /// Back (when there are saved accounts) and Play offline.
    private var backOrOffline: some View {
        HStack(spacing: 8) {
            if !accounts.isEmpty {
                secondary("Back", symbol: "chevron.left") { model.send("back") }
            }
            secondary("Play offline", symbol: "wifi.slash") { model.send("offline") }
        }
    }

    @ViewBuilder private var downloading: some View {
        let done = state.setup?.done ?? 0
        let total = state.setup?.total ?? 0
        let stage = state.setup?.stage ?? ""
        cardTitle("Getting the game files", symbol: "arrow.down.circle.fill")
        Text("Ferrite downloads your Minecraft 26.3 from Mojang (about 520 MB) and its own tables, and sets them up. This happens once: keep Ferrite open, on Wi-Fi.")
            .foregroundStyle(Theme.secondary)
        if total > 0 {
            let fraction = min(1, Double(done) / Double(total))
            HStack(alignment: .firstTextBaseline) {
                Text("\(Int(fraction * 100))%")
                    .font(.system(size: 34, weight: .semibold))
                    .monospacedDigit()
                    .contentTransition(.numericText())
                Spacer()
                Text("\(megabytes(done)) of \(megabytes(total)) MB")
                    .font(.subheadline.weight(.medium))
                    .monospacedDigit()
                    .foregroundStyle(Theme.secondary)
            }
            ProgressView(value: fraction)
                .tint(Theme.accent)
            Text(stage).font(.footnote).foregroundStyle(Theme.secondary).lineLimit(2)
        } else {
            waiting(stage.isEmpty ? "Starting the download..." : "\(stage)...")
        }
    }

    private func megabytes(_ bytes: UInt64) -> String {
        String(format: "%.1f", Double(bytes) / 1_000_000)
    }

    @ViewBuilder private var setupFailed: some View {
        cardTitle("The download stopped", symbol: "exclamationmark.triangle.fill", color: Theme.bad)
        Text(state.setup?.error ?? "").foregroundStyle(Theme.secondary)
        Text("Try again goes on where it stopped").font(.footnote).foregroundStyle(Theme.secondary)
        primary("Try again", symbol: "arrow.clockwise") { model.send("retry_setup") }
    }

    @ViewBuilder private var signInFirst: some View {
        cardTitle("Sign in once first", symbol: "person.crop.circle.badge.exclamationmark")
        Text("Ferrite downloads the game files from Mojang with a Microsoft account that owns Minecraft: Java Edition. Sign in once; after that you can play offline too.")
            .foregroundStyle(Theme.secondary)
        primary("Sign in", symbol: "person.crop.circle") { model.send("back") }
    }

    @ViewBuilder private var accountCard: some View {
        let account = state.account
        switch account?.kind ?? "asking" {
        case "choose":
            cardTitle("Remove an account", symbol: "person.crop.circle.badge.minus", color: Theme.bad)
            Text("Tap an account to remove it from this iPhone. Its worlds stay.")
                .foregroundStyle(Theme.secondary)
            ForEach(Array(accounts.enumerated()), id: \.offset) { index, other in
                Button(role: .destructive) {
                    model.send("remove:\(index)")
                } label: {
                    HStack(spacing: 8) {
                        HeadAvatar(uuid: other.uuid, size: 24)
                        Text("Remove \(other.name)")
                        Spacer()
                        Image(systemName: "trash")
                    }
                    .frame(maxWidth: .infinity)
                }
                .ferriteButton(prominent: false)
            }
            secondary("Done", symbol: "checkmark") { model.send("remove_mode") }
        case "resuming":
            waiting("Signing in as \(account?.name ?? "")...")
            secondary("Back", symbol: "chevron.left") { model.send("back") }
        case "code":
            HStack(spacing: 12) {
                MicrosoftLogo(size: 24)
                Text("Sign in with Microsoft").font(.title3.weight(.medium))
            }
            Text("Tap Open sign-in page and sign in with the Microsoft account that owns Minecraft: Java Edition, then come back here.")
                .foregroundStyle(Theme.secondary)
            if let code = account?.code {
                HStack(spacing: 12) {
                    Text(code)
                        .font(.system(size: 42, weight: .semibold, design: .monospaced))
                        .textSelection(.enabled)
                        .lineLimit(1)
                        .minimumScaleFactor(0.6)
                    Button {
                        UIPasteboard.general.string = code
                        feelTap(.light)
                        copied = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { copied = false }
                    } label: {
                        Label(copied ? "Copied" : "Copy", systemImage: copied ? "checkmark" : "doc.on.doc")
                            .font(.subheadline.weight(.medium))
                    }
                    .ferriteButton(prominent: false)
                }
            }
            primary("Open sign-in page", symbol: "safari.fill") {
                if let link = account?.link, let url = URL(string: link) {
                    UIApplication.shared.open(url)
                }
            }
            if let uri = account?.uri {
                Text("Or open \(uri) on any device and enter the code")
                    .font(.footnote)
                    .foregroundStyle(Theme.secondary)
            }
            HStack(spacing: 8) {
                ProgressView().tint(Theme.accent)
                Text("Waiting for you to sign in...").font(.footnote).foregroundStyle(Theme.secondary)
            }
            backOrOffline
        case "signed":
            waiting("Signed in as \(account?.name ?? ""). Starting...")
        case "failed":
            cardTitle("Sign-in didn't work", symbol: "exclamationmark.triangle.fill", color: Theme.bad)
            Text(account?.error ?? "").foregroundStyle(Theme.secondary)
            primary("Try again", symbol: "arrow.clockwise") { model.send("retry") }
            backOrOffline
        case "offline_name":
            cardTitle("Play offline", symbol: "wifi.slash")
            Text("Your name in single player and on offline-mode servers: 3 to 16 letters, numbers or _.")
                .foregroundStyle(Theme.secondary)
            TextField("Name", text: $offlineName)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .font(.title3.monospaced())
                .fieldStyle()
                .onAppear { offlineName = account?.name ?? "" }
            if let problem = account?.problem {
                Label(problem, systemImage: "exclamationmark.triangle")
                    .font(.footnote)
                    .foregroundStyle(Theme.bad)
            }
            primary("Play", symbol: "play.fill") { model.send("offline_name:\(offlineName)") }
            secondary("Back", symbol: "chevron.left") { model.send("back") }
        case "offline":
            waiting("Playing offline. Starting...")
        default:
            waiting("Connecting to Microsoft...")
            backOrOffline
        }
    }
}
