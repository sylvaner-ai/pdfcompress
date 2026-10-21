import Foundation

// Avec --compress, l'exécutable compresse les fichiers indiqués puis quitte, sans interface.
// Sinon, il lance l'application.
let arguments = CommandLine.arguments
if let index = arguments.firstIndex(of: "--compress") {
    exit(CommandLineMode.run(Array(arguments[(index + 1)...])))
}
PDFCompressApp.main()
