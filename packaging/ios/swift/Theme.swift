// Ferrite's look: the icon's yellow on charcoal, Liquid Glass on iOS 26 (with frosted material
// before it), the background the glass sits on, and the logos.
//
// Liquid Glass needs the iOS 26 SDK (Xcode 26, Swift 6.2): `#if compiler(>=6.2)` keeps the file
// building with an older Xcode, and `#available(iOS 26.0, *)` keeps iOS 16 to 25 on the material.
import SwiftUI
import UIKit

/// Ferrite's colors.
enum Theme {
    /// The icon's yellow, #FFC21A.
    static let accent = Color(red: 1.0, green: 0.76, blue: 0.10)
    /// The icon's amber, #F59E0B.
    static let amber = Color(red: 0.96, green: 0.62, blue: 0.04)
    /// The icon's charcoal, #1F2937.
    static let iron = Color(red: 0.12, green: 0.16, blue: 0.22)
    /// The icon's darker charcoal, #111827.
    static let ironDark = Color(red: 0.07, green: 0.09, blue: 0.15)
    /// The icon's pale yellow, #FEF3C7.
    static let cream = Color(red: 1.0, green: 0.95, blue: 0.78)
    static let background = Color(red: 0.05, green: 0.06, blue: 0.08)
    static let field = Color.white.opacity(0.07)
}

#if compiler(>=6.2)
/// The glass of a card or a capsule: tinted, and answering touches when `interactive`.
@available(iOS 26.0, *)
private func ferriteGlass(tint: Color?, interactive: Bool) -> Glass {
    var glass = Glass.regular
    if let tint {
        glass = glass.tint(tint)
    }
    if interactive {
        glass = glass.interactive()
    }
    return glass
}
#endif

extension View {
    /// A Liquid Glass card (iOS 26), or frosted material before it.
    @ViewBuilder
    func glassCard(cornerRadius: CGFloat = 24, tint: Color? = nil) -> some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *) {
            self.glassEffect(
                ferriteGlass(tint: tint, interactive: false),
                in: RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
            )
        } else {
            self.frosted(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous), tint: tint)
        }
        #else
        self.frosted(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous), tint: tint)
        #endif
    }

    /// A Liquid Glass capsule (iOS 26), or frosted material before it.
    @ViewBuilder
    func glassCapsule(tint: Color? = nil, interactive: Bool = false) -> some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *) {
            self.glassEffect(ferriteGlass(tint: tint, interactive: interactive), in: Capsule())
        } else {
            self.frosted(Capsule(), tint: tint)
        }
        #else
        self.frosted(Capsule(), tint: tint)
        #endif
    }

    /// A glass button (iOS 26): prominent ones in Ferrite's yellow. Bordered ones before it.
    @ViewBuilder
    func ferriteButton(prominent: Bool) -> some View {
        #if compiler(>=6.2)
        if #available(iOS 26.0, *) {
            if prominent {
                self.buttonStyle(.glassProminent).tint(Theme.accent)
            } else {
                self.buttonStyle(.glass)
            }
        } else {
            self.borderedButton(prominent: prominent)
        }
        #else
        self.borderedButton(prominent: prominent)
        #endif
    }

    /// iOS 16 to 25: frosted material, a faint tint and a hairline edge in `shape`.
    fileprivate func frosted<S: Shape>(_ shape: S, tint: Color?) -> some View {
        self.background {
            ZStack {
                shape.fill(.ultraThinMaterial)
                shape.fill((tint ?? Color.white).opacity(tint == nil ? 0.03 : 0.22))
                shape.stroke(Color.white.opacity(0.14), lineWidth: 1)
            }
        }
    }

    @ViewBuilder
    fileprivate func borderedButton(prominent: Bool) -> some View {
        if prominent {
            self.buttonStyle(.borderedProminent).tint(Theme.accent)
        } else {
            self.buttonStyle(.bordered).tint(Color.white)
        }
    }
}

/// What every tab sits on: charcoal, two glows in Ferrite's colors, and faint blocks, so the
/// glass has something to show through it. Still, for the battery.
struct LauncherBackground: View {
    var body: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height
            ZStack {
                LinearGradient(
                    colors: [Color(red: 0.09, green: 0.10, blue: 0.14), Color(red: 0.03, green: 0.035, blue: 0.05)],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                Circle()
                    .fill(Theme.accent.opacity(0.32))
                    .frame(width: width * 0.55, height: width * 0.55)
                    .blur(radius: 90)
                    .position(x: width * 0.10, y: height * 0.02)
                Circle()
                    .fill(Theme.amber.opacity(0.24))
                    .frame(width: width * 0.5, height: width * 0.5)
                    .blur(radius: 100)
                    .position(x: width * 0.96, y: height * 1.02)
                BlockField()
            }
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
    }
}

/// Faint outlines of blocks, seen from above at an angle, scattered over the background.
private struct BlockField: View {
    var body: some View {
        Canvas { context, size in
            let edge: CGFloat = 26
            let dx = edge * 0.8660254
            let dy = edge * 0.5
            var row = 0
            var y: CGFloat = -edge
            while y < size.height + edge {
                var column = 0
                var x: CGFloat = row % 2 == 0 ? dx : dx * 4
                while x < size.width + dx {
                    // A fixed scatter: the same blocks every time.
                    let seed = (row &* 7919) ^ (column &* 104_729)
                    if seed % 3 == 0 {
                        var outline = Path()
                        outline.move(to: CGPoint(x: x, y: y))
                        outline.addLine(to: CGPoint(x: x + dx, y: y + dy))
                        outline.addLine(to: CGPoint(x: x + dx, y: y + dy + edge))
                        outline.addLine(to: CGPoint(x: x, y: y + 2 * dy + edge))
                        outline.addLine(to: CGPoint(x: x - dx, y: y + dy + edge))
                        outline.addLine(to: CGPoint(x: x - dx, y: y + dy))
                        outline.closeSubpath()
                        outline.move(to: CGPoint(x: x - dx, y: y + dy))
                        outline.addLine(to: CGPoint(x: x, y: y + 2 * dy))
                        outline.addLine(to: CGPoint(x: x + dx, y: y + dy))
                        outline.move(to: CGPoint(x: x, y: y + 2 * dy))
                        outline.addLine(to: CGPoint(x: x, y: y + 2 * dy + edge))
                        context.stroke(outline, with: .color(Color.white.opacity(0.045)), lineWidth: 1)
                    }
                    x += dx * 6
                    column += 1
                }
                y += edge * 2.25
                row += 1
            }
        }
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
                with: .linearGradient(
                    Gradient(colors: [Theme.accent, Theme.amber]),
                    startPoint: CGPoint(x: 0, y: 0),
                    endPoint: CGPoint(x: 0, y: canvas.height)
                )
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

/// A small glass capsule with a symbol: the version, the days left, a status.
struct Chip: View {
    let text: String
    let symbol: String
    var color: Color = .white

    var body: some View {
        Label(text, systemImage: symbol)
            .font(.caption.weight(.semibold))
            .foregroundStyle(color)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .glassCapsule()
    }
}

/// A tab's page: the background, and its content scrolling over it.
func tabPage<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
    ZStack {
        LauncherBackground()
        ScrollView {
            VStack(alignment: .leading, spacing: 16, content: content)
                .padding(.horizontal, 28)
                .padding(.vertical, 18)
                .frame(maxWidth: 760, alignment: .leading)
                .frame(maxWidth: .infinity)
        }
    }
}

/// A tab's title, with its symbol in Ferrite's yellow.
struct PageTitle: View {
    let title: String
    let symbol: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: symbol)
                .font(.title2.weight(.bold))
                .foregroundStyle(Theme.accent)
            Text(title)
                .font(.system(size: 32, weight: .heavy, design: .rounded))
        }
        .padding(.bottom, 2)
    }
}

/// A text field's look: a faint rounded field.
extension View {
    func fieldStyle() -> some View {
        self.padding(12)
            .background(Theme.field, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}

/// A tap that is felt: the Play button and other big moments.
func feelTap(_ style: UIImpactFeedbackGenerator.FeedbackStyle = .medium) {
    UIImpactFeedbackGenerator(style: style).impactOccurred()
}
