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
    var body: some View {
        tabPage {
            Text("Refresh").font(.system(size: 34, weight: .bold, design: .rounded))
            ExpiryCard()
            infoCard {
                Label("Refresh from inside Ferrite", systemImage: "arrow.clockwise.circle.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                Text("Coming in the next builds: Ferrite signs itself again with your Apple ID, so it keeps opening after 7 days, and reminds you a day before it runs out.")
                    .foregroundStyle(.secondary)
                Text("For now, refresh it with Sideloadly, AltStore or SideStore.")
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
