// Ferrite's look: graphite greys like the game's own screens, the icon's yellow for the one thing
// that matters on a screen (Play, the selection, the days left), and the logos. Surfaces are flat
// steps of one grey with a hairline edge: no glows, no colored washes.
import SwiftUI
import UIKit

/// Ferrite's colors: one neutral ramp, three text roles, one accent.
enum Theme {
    /// The icon's yellow, #FFC21A: Play, the selection, the days left.
    static let accent = Color(red: 1.0, green: 0.76, blue: 0.10)
    /// Text and symbols on the yellow.
    static let accentInk = Color(red: 0.09, green: 0.08, blue: 0.05)
    /// The icon's charcoal, #1F2937, for the logo's blocks.
    static let iron = Color(red: 0.12, green: 0.16, blue: 0.22)
    /// The icon's darker charcoal, #111827.
    static let ironDark = Color(red: 0.07, green: 0.09, blue: 0.15)
    /// The icon's pale yellow, #FEF3C7.
    static let cream = Color(red: 1.0, green: 0.95, blue: 0.78)

    /// The graphite ramp, darkest first.
    static let background = Color(red: 0.055, green: 0.058, blue: 0.063)
    static let surface = Color(red: 0.098, green: 0.102, blue: 0.110)
    static let raised = Color(red: 0.145, green: 0.149, blue: 0.157)
    static let hairline = Color.white.opacity(0.08)
    static let field = Color.white.opacity(0.06)

    static let text = Color.white.opacity(0.94)
    static let secondary = Color.white.opacity(0.62)
    static let muted = Color.white.opacity(0.40)

    /// Meaning only: all good, and a problem. Toned down to sit with the greys.
    static let good = Color(red: 0.47, green: 0.80, blue: 0.53)
    static let bad = Color(red: 0.95, green: 0.43, blue: 0.39)
}

extension View {
    /// A card: one step up from the background, with a hairline edge.
    func card(cornerRadius: CGFloat = 16) -> some View {
        self
            .background(Theme.surface, in: RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .stroke(Theme.hairline, lineWidth: 1)
            }
    }

    /// A small tile (an account, More): raised when selected, with the accent's edge.
    func tile(selected: Bool = false) -> some View {
        self
            .background(
                selected ? Theme.raised : Theme.surface,
                in: RoundedRectangle(cornerRadius: 10, style: .continuous)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(selected ? Theme.accent : Theme.hairline, lineWidth: selected ? 1.5 : 1)
            }
    }

    /// Ferrite's buttons: the prominent one in the yellow, the rest a raised grey.
    func ferriteButton(prominent: Bool) -> some View {
        self.buttonStyle(FerriteButtonStyle(prominent: prominent))
    }
}

struct FerriteButtonStyle: ButtonStyle {
    let prominent: Bool

    func makeBody(configuration: Configuration) -> some View {
        FerriteButtonBody(configuration: configuration, prominent: prominent)
    }
}

private struct FerriteButtonBody: View {
    let configuration: ButtonStyleConfiguration
    let prominent: Bool
    @Environment(\.isEnabled) private var enabled

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: 12, style: .continuous)
        configuration.label
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .foregroundStyle(prominent ? Theme.accentInk : Theme.text)
            .background(prominent ? Theme.accent : Theme.raised, in: shape)
            .overlay {
                shape.stroke(prominent ? Color.clear : Theme.hairline, lineWidth: 1)
            }
            .opacity(enabled ? (configuration.isPressed ? 0.82 : 1) : 0.4)
            .scaleEffect(configuration.isPressed ? 0.98 : 1)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

/// What every tab sits on: the darkest grey.
struct LauncherBackground: View {
    var body: some View {
        Theme.background
            .ignoresSafeArea()
            .allowsHitTesting(false)
    }
}

/// Ferrite's logo, drawn: an F of charcoal blocks on yellow, its last block being placed.
struct FerriteLogo: View {
    var size: CGFloat = 44

    var body: some View {
        Canvas { context, canvas in
            let k = canvas.width / 1024
            let square = CGRect(x: 0, y: 0, width: canvas.width, height: canvas.height)
            context.fill(
                Path(roundedRect: square, cornerRadius: 230 * k, style: .continuous),
                with: .color(Theme.accent)
            )
            let blocks: [(CGFloat, CGFloat)] = [
                (280, 220), (400, 220), (520, 220), (280, 340), (280, 460), (400, 460), (520, 460),
                (280, 580), (280, 700),
            ]
            for (x, y) in blocks {
                let block = CGRect(x: x * k, y: y * k, width: 104 * k, height: 104 * k)
                context.fill(Path(roundedRect: block, cornerRadius: 12 * k), with: .color(Theme.iron))
            }
            let slot = CGRect(x: 640 * k, y: 220 * k, width: 104 * k, height: 104 * k)
            context.stroke(
                Path(roundedRect: slot, cornerRadius: 12 * k),
                with: .color(Theme.iron.opacity(0.35)),
                style: StrokeStyle(lineWidth: 6 * k, dash: [18 * k, 14 * k])
            )
            let placed = CGRect(x: 666 * k, y: 194 * k, width: 104 * k, height: 104 * k)
            context.fill(Path(roundedRect: placed, cornerRadius: 12 * k), with: .color(Theme.ironDark))
            let shine = CGRect(x: 682 * k, y: 210 * k, width: 72 * k, height: 14 * k)
            context.fill(Path(roundedRect: shine, cornerRadius: 7 * k), with: .color(Theme.cream))
        }
        .frame(width: size, height: size)
        .accessibilityLabel("Ferrite")
    }
}

/// Microsoft's four squares, for the "Sign in with Microsoft" card, as Microsoft's sign-in
/// branding has them.
struct MicrosoftLogo: View {
    var size: CGFloat = 20

    var body: some View {
        let gap = size * 0.08
        let cell = (size - gap) / 2
        VStack(spacing: gap) {
            HStack(spacing: gap) {
                Rectangle().fill(Color(red: 0.95, green: 0.31, blue: 0.13)).frame(width: cell, height: cell)
                Rectangle().fill(Color(red: 0.50, green: 0.73, blue: 0.0)).frame(width: cell, height: cell)
            }
            HStack(spacing: gap) {
                Rectangle().fill(Color(red: 0.0, green: 0.64, blue: 0.94)).frame(width: cell, height: cell)
                Rectangle().fill(Color(red: 1.0, green: 0.73, blue: 0.0)).frame(width: cell, height: cell)
            }
        }
        .accessibilityHidden(true)
    }
}

/// A quiet status: a small symbol and a short text, in one color, with no capsule.
struct Chip: View {
    let text: String
    let symbol: String
    var color: Color = Theme.secondary

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: symbol)
                .font(.caption)
            Text(text)
                .font(.footnote.weight(.medium))
                .monospacedDigit()
        }
        .foregroundStyle(color)
        .lineLimit(1)
    }
}

/// Ferrite's signature: the 7 days a free signature lasts, as 7 blocks, the ones left filled.
struct DaysBlocks: View {
    var block: CGFloat = 10

    var body: some View {
        let days = Signing.daysLeft
        let filled = days.map { max(0, min(7, $0)) } ?? 0
        let color: Color = days.map { $0 <= 1 ? Theme.bad : Theme.accent } ?? Theme.muted
        HStack(spacing: max(2, (block * 0.3).rounded())) {
            ForEach(0..<7, id: \.self) { i in
                RoundedRectangle(cornerRadius: max(1.5, block * 0.18), style: .continuous)
                    .fill(i < filled ? color : Color.white.opacity(0.09))
                    .frame(width: block, height: block)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(days.map { "\($0) of 7 days left" } ?? "Days left unknown")
    }
}

/// "5 days left", "1 day left", "Run out".
func daysLeftText(_ days: Int) -> String {
    days == 0 ? "Run out" : (days == 1 ? "1 day left" : "\(days) days left")
}

/// A tab's page: the background, and its content scrolling over it. The last card scrolls up
/// clear of the floating tab bar.
func tabPage<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
    ZStack {
        LauncherBackground()
        ScrollView {
            VStack(alignment: .leading, spacing: 16, content: content)
                .padding(.horizontal, 28)
                .padding(.top, 20)
                .padding(.bottom, 32)
                .frame(maxWidth: 760, alignment: .leading)
                .frame(maxWidth: .infinity)
        }
    }
}

/// A tab's title.
struct PageTitle: View {
    let title: String

    var body: some View {
        Text(title)
            .font(.system(size: 28, weight: .semibold))
            .foregroundStyle(Theme.text)
            .padding(.bottom, 4)
    }
}

/// A card's small heading: sentence case, secondary, no symbol.
struct CardHeading: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .font(.subheadline.weight(.medium))
            .foregroundStyle(Theme.secondary)
    }
}

/// A text field's look: a faint rounded field.
extension View {
    func fieldStyle() -> some View {
        self.padding(12)
            .background(Theme.field, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(Theme.hairline, lineWidth: 1)
            }
    }
}

/// A tap that is felt: the Play button and other big moments.
func feelTap(_ style: UIImpactFeedbackGenerator.FeedbackStyle = .medium) {
    UIImpactFeedbackGenerator(style: style).impactOccurred()
}
