import AppIntents
import SwiftUI

struct PDFCompressApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var model = CompressionModel.shared

    init() {
        PDFCompressShortcuts.updateAppShortcutParameters()
    }

    var body: some Scene {
        Window("PDF Compress", id: "main") {
            ContentView().environmentObject(model)
        }
        .defaultSize(width: 440, height: 560)

        Settings {
            PreferencesView()
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    /// Fichiers déposés sur l'icône de l'application (Dock, Finder) ou ouverts avec elle.
    func application(_ application: NSApplication, open urls: [URL]) {
        Task { @MainActor in CompressionModel.shared.add(urls) }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
