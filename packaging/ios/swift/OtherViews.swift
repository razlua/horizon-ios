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

private func infoCard<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
    VStack(alignment: .leading, spacing: 10, content: content)
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.card, in: RoundedRectangle(cornerRadius: 16))
}

struct RefreshView: View {
    var body: some View {
        tabPage {
            Text("Refresh").font(.system(size: 34, weight: .bold, design: .rounded))
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
    var body: some View {
        tabPage {
            Text("Settings").font(.system(size: 34, weight: .bold, design: .rounded))
            infoCard {
                Label("Coming here soon", systemImage: "gearshape.fill")
                    .font(.headline)
                    .foregroundStyle(Theme.accent)
                Text("Render distance, Safe Area, the HUD and the battery options will be here. For now they're in the game: Options, then Extra Settings (Ferrite) and Video Settings.")
                    .foregroundStyle(.secondary)
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
