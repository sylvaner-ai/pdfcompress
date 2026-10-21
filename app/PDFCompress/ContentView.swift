import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject private var model: CompressionModel
    @State private var isTargeted = false
    @State private var isImporting = false

    var body: some View {
        VStack(spacing: 0) {
            dropZone
            if !model.items.isEmpty {
                List(model.items) { FileRow(item: $0) }
                    .listStyle(.inset)
                footer
            }
        }
        .frame(minWidth: 360, minHeight: 420)
        .dropDestination(for: URL.self) { urls, _ in
            model.add(urls)
            return true
        } isTargeted: { isTargeted = $0 }
        .fileImporter(isPresented: $isImporting, allowedContentTypes: [.pdf, .folder], allowsMultipleSelection: true) { result in
            if case .success(let urls) = result { model.add(urls) }
        }
    }

    private var dropZone: some View {
        VStack(spacing: 8) {
            Image(systemName: "arrow.down.doc")
                .font(.system(size: 36, weight: .light))
                .foregroundStyle(isTargeted ? Color.accentColor : .secondary)
            Text("Déposez vos PDF ici")
                .font(.headline)
            Text("Ils sont compressés sur place, sans copie.")
                .font(.callout)
                .foregroundStyle(.secondary)
            Button("Choisir des fichiers…") { isImporting = true }
                .buttonStyle(.link)
        }
        .frame(maxWidth: .infinity, minHeight: model.items.isEmpty ? 300 : 150)
        .frame(maxHeight: model.items.isEmpty ? .infinity : 150)
        .background {
            RoundedRectangle(cornerRadius: 12)
                .strokeBorder(style: StrokeStyle(lineWidth: 2, dash: [6]))
                .foregroundStyle(isTargeted ? Color.accentColor : Color.secondary.opacity(0.4))
                .background(RoundedRectangle(cornerRadius: 12).fill(isTargeted ? Color.accentColor.opacity(0.08) : .clear))
        }
        .padding(16)
    }

    private var footer: some View {
        HStack {
            if model.totalBefore > 0 {
                let saved = model.totalBefore - model.totalAfter
                let pct = Int((Double(saved) / Double(model.totalBefore) * 100).rounded())
                Text("Gain total : \(saved.formattedFileSize) (−\(pct) %)")
            }
            Spacer()
            Button("Effacer la liste") { model.clear() }
                .buttonStyle(.link)
        }
        .font(.callout)
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

private struct FileRow: View {
    let item: CompressionModel.Item

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: 2) {
                Text(item.url.lastPathComponent)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            .help(item.url.path)
            Spacer()
            status
        }
        .padding(.vertical, 2)
    }

    @ViewBuilder private var status: some View {
        switch item.state {
        case .pending:
            Image(systemName: "clock").foregroundStyle(.secondary)
        case .running:
            ProgressView().controlSize(.small)
        case .done(let o) where o.replaced:
            let pct = Int(((1 - Double(o.newSize) / Double(o.originalSize)) * 100).rounded())
            Text(pct > 0 ? "−\(pct) %" : "−<1 %").monospacedDigit().foregroundStyle(.green)
        case .done:
            Image(systemName: "checkmark").foregroundStyle(.secondary)
        case .failed:
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
        }
    }

    private var detail: String {
        switch item.state {
        case .pending: "En attente…"
        case .running: "Compression…"
        case .done(let o) where o.replaced: "\(o.originalSize.formattedFileSize) → \(o.newSize.formattedFileSize)"
        case .done: "Déjà optimisé, fichier inchangé"
        case .failed(let message): message
        }
    }
}
