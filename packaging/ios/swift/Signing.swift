// When this install of Ferrite stops opening, and the reminders before it. A sideloaded app is
// signed with a provisioning profile (embedded.mobileprovision in the app) that ends: 7 days
// after signing with a free Apple ID. Its end date is in the profile's plist (`ExpirationDate`).
import Foundation
import UserNotifications

enum Signing {
    /// The profile's plist: between "<?xml" and "</plist>" in the signed (CMS) file. Nil without a
    /// profile (the Simulator, an ad hoc build).
    private static let profile: [String: Any]? = {
        guard let url = Bundle.main.url(forResource: "embedded", withExtension: "mobileprovision"),
              let data = try? Data(contentsOf: url),
              let start = data.range(of: Data("<?xml".utf8)),
              let end = data.range(of: Data("</plist>".utf8), in: start.lowerBound..<data.endIndex)
        else { return nil }
        let plist = data.subdata(in: start.lowerBound..<end.upperBound)
        return (try? PropertyListSerialization.propertyList(from: plist, format: nil)) as? [String: Any]
    }()

    /// When Ferrite stops opening until it's signed again.
    static var expiry: Date? { profile?["ExpirationDate"] as? Date }

    /// The Apple ID's team that signed it ("Jane Appleseed (Personal Team)").
    static var team: String? { profile?["TeamName"] as? String }

    /// Days left, rounded up: 0 once it has run out.
    static var daysLeft: Int? {
        guard let expiry else { return nil }
        return max(0, Int((expiry.timeIntervalSinceNow / 86_400).rounded(.up)))
    }

    /// "Tue 18 Nov, 18:40".
    static func format(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.setLocalizedDateFormatFromTemplate("EEE d MMM HH:mm")
        return formatter.string(from: date)
    }

    private static let soon = "ferrite.expiry.tomorrow"
    private static let today = "ferrite.expiry.today"

    /// A reminder a day before Ferrite runs out and another a few hours before, replacing the
    /// ones scheduled for an earlier signature. iOS asks the player once whether Ferrite may
    /// send notifications.
    static func scheduleReminders() {
        guard let expiry else { return }
        let center = UNUserNotificationCenter.current()
        center.requestAuthorization(options: [.alert, .sound]) { granted, _ in
            guard granted else { return }
            center.removePendingNotificationRequests(withIdentifiers: [Signing.soon, Signing.today])
            Signing.add(center, id: Signing.soon, at: expiry.addingTimeInterval(-24 * 3600),
                title: "Ferrite runs out tomorrow",
                body: "Refresh it today so it keeps opening.")
            Signing.add(center, id: Signing.today, at: expiry.addingTimeInterval(-3 * 3600),
                title: "Ferrite runs out today",
                body: "Refresh it now, or it won't open until it's signed again.")
        }
    }

    private static func add(_ center: UNUserNotificationCenter, id: String, at date: Date,
                            title: String, body: String) {
        let wait = date.timeIntervalSinceNow
        guard wait > 60 else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: wait, repeats: false)
        center.add(UNNotificationRequest(identifier: id, content: content, trigger: trigger))
    }
}
