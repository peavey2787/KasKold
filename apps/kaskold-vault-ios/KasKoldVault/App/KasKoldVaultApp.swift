import SwiftUI
import UIKit

@main
struct KasKoldVaultApp: App {
    var body: some Scene {
        WindowGroup {
            VaultHomeView()
                .modifier(CapturePrivacy())
        }
    }
}

/// Covers the Vault whenever its contents could leave the device: while the
/// screen is recorded or mirrored, and whenever the app is not active so the
/// app-switcher snapshot never shows seed words or keys.
private struct CapturePrivacy: ViewModifier {
    @Environment(\.scenePhase) private var scenePhase
    @State private var captured = false

    func body(content: Content) -> some View {
        content
            .overlay {
                if captured || scenePhase != .active {
                    Color(uiColor: .systemBackground)
                        .ignoresSafeArea()
                        .overlay(Text("KasKold Vault is hidden").font(.headline))
                }
            }
            .onAppear { captured = Self.anyScreenCaptured() }
            .onReceive(NotificationCenter.default.publisher(for: UIScreen.capturedDidChangeNotification)) { _ in
                captured = Self.anyScreenCaptured()
            }
    }

    private static func anyScreenCaptured() -> Bool {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .contains { $0.traitCollection.sceneCaptureState == .active }
    }
}
