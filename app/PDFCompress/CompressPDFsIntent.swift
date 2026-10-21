import AppIntents
import Foundation
import UniformTypeIdentifiers

/// Action « Compresser des PDF » proposée dans Raccourcis, utilisable comme Action rapide du Finder.
struct CompressPDFsIntent: AppIntent {
    static var title: LocalizedStringResource = "Compresser des PDF"
    static var description = IntentDescription(
        "Compresse des PDF sur place avec les réglages de PDF Compress. Un fichier n'est remplacé que si le résultat est plus léger."
    )
    static var openAppWhenRun = false

    @Parameter(title: "PDF", supportedTypeIdentifiers: ["com.adobe.pdf"], inputConnectionBehavior: .connectToPreviousIntentResult)
    var files: [IntentFile]

    static var parameterSummary: some ParameterSummary {
        Summary("Compresser \(\.$files)")
    }

    func perform() async throws -> some IntentResult & ReturnsValue<[IntentFile]> & ProvidesDialog {
        let settings = CompressionSettings.load()
        var compressed = 0
        var saved: Int64 = 0
        var failures: [String] = []
        var results: [IntentFile] = []

        for file in files {
            guard let url = file.fileURL else {
                failures.append("\(file.filename) : emplacement du fichier inconnu")
                continue
            }
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            do {
                let outcome = try await Engine.compressInBackground(url, settings: settings)
                if outcome.replaced {
                    compressed += 1
                    saved += outcome.originalSize - outcome.newSize
                }
            } catch {
                failures.append("\(url.lastPathComponent) : \(error.localizedDescription)")
            }
            results.append(IntentFile(fileURL: url, filename: url.lastPathComponent, type: .pdf))
        }

        var lines = [compressed == 0 ? "Aucun PDF n'a pu être allégé." : "\(compressed) PDF compressé(s), \(saved.formattedFileSize) gagnés."]
        lines += failures
        return .result(value: results, dialog: IntentDialog(stringLiteral: lines.joined(separator: "\n")))
    }
}

struct PDFCompressShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: CompressPDFsIntent(),
            phrases: ["Compresser des PDF avec \(.applicationName)"],
            shortTitle: "Compresser des PDF",
            systemImageName: "arrow.down.doc"
        )
    }
}
