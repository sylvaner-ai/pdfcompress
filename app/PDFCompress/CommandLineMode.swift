import Foundation

/// Mode ligne de commande, utilisé par l'Action rapide du Finder (script shell dans Raccourcis),
/// indépendant de la communication entre Raccourcis et l'application :
///
///     "/Applications/PDF Compress.app/Contents/MacOS/PDF Compress" --compress fichier.pdf dossier…
///
/// Une ligne par fichier, puis une dernière ligne de résumé ; code de sortie 1 si un fichier a échoué.
enum CommandLineMode {
    static func run(_ paths: [String]) -> Int32 {
        let settings = CompressionSettings.load()
        let urls = Engine.collectPDFs(from: paths.map { URL(fileURLWithPath: $0) })
        var compressed = 0
        var unchanged = 0
        var failures = 0
        var saved: Int64 = 0

        for url in urls {
            do {
                let o = try Engine.compress(url, settings: settings)
                if o.replaced {
                    compressed += 1
                    saved += o.originalSize - o.newSize
                    print("\(url.lastPathComponent) : \(o.originalSize.formattedFileSize) → \(o.newSize.formattedFileSize)")
                } else {
                    unchanged += 1
                    print("\(url.lastPathComponent) : déjà optimisé, inchangé")
                }
            } catch {
                failures += 1
                print("\(url.lastPathComponent) : \(error.localizedDescription)")
            }
        }

        var summary: [String] = []
        if compressed > 0 { summary.append("\(compressed) PDF compressé(s), \(saved.formattedFileSize) gagnés") }
        if unchanged > 0 { summary.append("\(unchanged) déjà optimisé(s)") }
        if failures > 0 { summary.append("\(failures) échec(s)") }
        print(summary.isEmpty ? "Aucun PDF dans la sélection." : summary.joined(separator: " · "))
        return failures == 0 ? 0 : 1
    }
}
