import SwiftUI

struct PreferencesView: View {
    @AppStorage(CompressionSettings.presetKey) private var preset = Preset.balanced.rawValue
    @AppStorage(CompressionSettings.imageDPIKey) private var imageDPI = 150
    @AppStorage(CompressionSettings.jpegQualityKey) private var jpegQuality = 70

    private var settings: CompressionSettings {
        CompressionSettings(preset: Preset(rawValue: preset) ?? .balanced, imageDPI: imageDPI, jpegQuality: jpegQuality)
    }

    var body: some View {
        Form {
            Picker("Compression :", selection: $preset) {
                ForEach(Preset.allCases) { Text($0.label).tag($0.rawValue) }
            }
            .pickerStyle(.radioGroup)

            if preset == Preset.custom.rawValue {
                LabeledContent("Résolution des images :") {
                    HStack {
                        Slider(value: double($imageDPI), in: 50...600, step: 10)
                        Text("\(imageDPI) dpi").monospacedDigit().frame(width: 64, alignment: .trailing)
                    }
                }
                LabeledContent("Qualité JPEG :") {
                    HStack {
                        Slider(value: double($jpegQuality), in: 10...100, step: 5)
                        Text("\(jpegQuality)").monospacedDigit().frame(width: 64, alignment: .trailing)
                    }
                }
            } else {
                let e = settings.effective
                LabeledContent("Réglages appliqués :") {
                    Text("images à \(e.dpi) dpi, qualité JPEG \(e.quality)").foregroundStyle(.secondary)
                }
            }

            Text("Un fichier n'est remplacé que si le résultat est plus léger.")
                .font(.callout)
                .foregroundStyle(.secondary)
            Text("Ghostscript \(Engine.version ?? "indisponible")")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .padding(20)
        .frame(width: 420)
    }

    private func double(_ binding: Binding<Int>) -> Binding<Double> {
        Binding(get: { Double(binding.wrappedValue) }, set: { binding.wrappedValue = Int($0.rounded()) })
    }
}
