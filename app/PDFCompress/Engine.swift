import Foundation

/// Niveaux de compression (mêmes valeurs que `PCPreset` côté Rust).
enum Preset: Int, CaseIterable, Identifiable, Sendable {
    case strong = 0
    case balanced = 1
    case light = 2
    case custom = 3

    var id: Int { rawValue }

    var label: String {
        switch self {
        case .strong: "Forte"
        case .balanced: "Équilibrée"
        case .light: "Légère"
        case .custom: "Personnalisée"
        }
    }
}

/// Réglages de compression, mémorisés dans les préférences de l'application.
/// L'application et les actions Raccourcis lisent les mêmes valeurs.
struct CompressionSettings: Sendable {
    static let presetKey = "preset"
    static let imageDPIKey = "imageDpi"
    static let jpegQualityKey = "jpegQuality"

    var preset: Preset = .balanced
    var imageDPI = 150
    var jpegQuality = 70

    static func load(from defaults: UserDefaults = .standard) -> CompressionSettings {
        var s = CompressionSettings()
        if let raw = defaults.object(forKey: presetKey) as? Int, let p = Preset(rawValue: raw) { s.preset = p }
        if let dpi = defaults.object(forKey: imageDPIKey) as? Int { s.imageDPI = dpi }
        if let q = defaults.object(forKey: jpegQualityKey) as? Int { s.jpegQuality = q }
        return s
    }

    fileprivate var c: PCSettings {
        PCSettings(preset: UInt32(preset.rawValue), image_dpi: UInt32(clamping: imageDPI), jpeg_quality: UInt32(clamping: jpegQuality))
    }

    /// Résolution et qualité JPEG réellement appliquées.
    var effective: (dpi: Int, quality: Int) {
        var dpi: UInt32 = 0
        var quality: UInt32 = 0
        pc_effective_settings(c, &dpi, &quality)
        return (Int(dpi), Int(quality))
    }
}

struct CompressionOutcome: Sendable {
    let url: URL
    let originalSize: Int64
    let newSize: Int64
    /// Faux lorsque le fichier était déjà optimisé : l'original est alors laissé intact.
    let replaced: Bool
}

struct CompressionError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

/// Accès au moteur Rust.
enum Engine {
    /// Ressources du Ghostscript embarqué (absentes en développement : libgs utilise alors les siennes).
    private static let resourceDirectory: String? = {
        guard let dir = Bundle.main.resourceURL?.appendingPathComponent("gs", isDirectory: true),
              FileManager.default.fileExists(atPath: dir.path) else { return nil }
        return dir.path
    }()

    static let version: String? = {
        guard let s = pc_engine_version() else { return nil }
        defer { pc_string_free(s) }
        return String(cString: s)
    }()

    /// Compresse le PDF sur place. Bloquant : à appeler hors du fil principal.
    static func compress(_ url: URL, settings: CompressionSettings) throws -> CompressionOutcome {
        let out = url.withUnsafeFileSystemRepresentation { path in
            if let dir = resourceDirectory {
                return dir.withCString { pc_compress_in_place(path, settings.c, $0) }
            }
            return pc_compress_in_place(path, settings.c, nil)
        }
        if let message = out.message {
            defer { pc_string_free(message) }
            throw CompressionError(message: String(cString: message))
        }
        if out.status != 0 {
            throw CompressionError(message: "Échec de la compression.")
        }
        return CompressionOutcome(url: url, originalSize: Int64(out.original_size), newSize: Int64(out.new_size), replaced: out.replaced)
    }

    /// Développe les éléments déposés : fichiers PDF et dossiers (parcourus récursivement,
    /// sans les éléments cachés), sans doublon.
    static func collectPDFs(from urls: [URL]) -> [URL] {
        var result: [URL] = []
        var seen = Set<String>()
        func add(_ url: URL) {
            guard url.pathExtension.lowercased() == "pdf", seen.insert(url.standardizedFileURL.path).inserted else { return }
            result.append(url)
        }
        let fm = FileManager.default
        for url in urls {
            var isDir: ObjCBool = false
            guard fm.fileExists(atPath: url.path, isDirectory: &isDir) else { continue }
            guard isDir.boolValue else { add(url); continue }
            let files = fm.enumerator(at: url, includingPropertiesForKeys: [.isRegularFileKey], options: [.skipsHiddenFiles, .skipsPackageDescendants])?
                .compactMap { $0 as? URL }
                .filter { (try? $0.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true }
                .sorted { $0.path.localizedStandardCompare($1.path) == .orderedAscending } ?? []
            files.forEach(add)
        }
        return result
    }

    /// Compresse hors du fil principal.
    static func compressInBackground(_ url: URL, settings: CompressionSettings) async throws -> CompressionOutcome {
        try await Task.detached(priority: .userInitiated) { try compress(url, settings: settings) }.value
    }
}

extension Int64 {
    var formattedFileSize: String { ByteCountFormatter.string(fromByteCount: self, countStyle: .file) }
}
