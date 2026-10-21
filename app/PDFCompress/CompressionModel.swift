import Foundation

/// File de compression de la fenêtre principale.
@MainActor
final class CompressionModel: ObservableObject {
    static let shared = CompressionModel()

    enum State {
        case pending
        case running
        case done(CompressionOutcome)
        case failed(String)
    }

    struct Item: Identifiable {
        let id = UUID()
        let url: URL
        var state: State = .pending

        var isFinished: Bool {
            switch state {
            case .pending, .running: false
            case .done, .failed: true
            }
        }
    }

    @Published private(set) var items: [Item] = []
    @Published private(set) var totalBefore: Int64 = 0
    @Published private(set) var totalAfter: Int64 = 0

    private var queue: [UUID] = []
    private var isRunning = false

    /// Ajoute des fichiers ou dossiers déposés ; la compression démarre aussitôt.
    func add(_ urls: [URL]) {
        for url in Engine.collectPDFs(from: urls) {
            let item = Item(url: url)
            items.insert(item, at: 0)
            queue.append(item.id)
        }
        guard !isRunning else { return }
        isRunning = true
        Task { await drain() }
    }

    func clear() {
        items.removeAll(where: \.isFinished)
        if items.isEmpty {
            totalBefore = 0
            totalAfter = 0
        }
    }

    // Le moteur traite un fichier à la fois : la file est vidée dans l'ordre.
    private func drain() async {
        while !queue.isEmpty {
            let id = queue.removeFirst()
            guard let url = items.first(where: { $0.id == id })?.url else { continue }
            setState(.running, for: id)
            do {
                let outcome = try await Engine.compressInBackground(url, settings: .load())
                totalBefore += outcome.originalSize
                totalAfter += outcome.newSize
                setState(.done(outcome), for: id)
            } catch {
                setState(.failed(error.localizedDescription), for: id)
            }
        }
        isRunning = false
    }

    private func setState(_ state: State, for id: UUID) {
        if let i = items.firstIndex(where: { $0.id == id }) { items[i].state = state }
    }
}
