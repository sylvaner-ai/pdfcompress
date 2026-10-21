# PDF Compress

Petite application macOS pour compresser des fichiers PDF par glisser-déposer, et depuis Raccourcis ou le Finder.

- Déposez un ou plusieurs PDF (ou des dossiers) dans la fenêtre ou sur l'icône de l'application : ils sont compressés **sur place**, sans copie ni question.
- L'original n'est remplacé que si le résultat est plus léger ; le remplacement passe par un fichier temporaire et un renommage atomique, l'original n'est donc jamais laissé à moitié écrit. Un PDF protégé par mot de passe est laissé intact.
- Niveau par défaut **Équilibré** (images à 150 dpi, qualité JPEG 70). Les préférences (⌘,) proposent Forte, Légère ou des réglages personnalisés.
- L'action **Compresser des PDF** est disponible dans l'app Raccourcis, avec les mêmes réglages.

L'interface est en SwiftUI ; le moteur, en Rust, pilote [Ghostscript](https://www.ghostscript.com/) en tant que bibliothèque, embarquée dans l'application : rien à installer.

## Installer

1. Ouvrez l'onglet **Actions** du dépôt, choisissez la dernière exécution réussie du workflow **Build** sur `main` et téléchargez l'artefact **PDF-Compress-macOS**.
2. Décompressez-le, ouvrez le `.dmg` et glissez **PDF Compress** dans **Applications**.

L'application cible les Mac à puce Apple sous macOS 14 ou plus récent. Elle est signée et notarisée lorsque les secrets décrits dans [docs/signature.md](docs/signature.md) sont configurés ; sinon, au premier lancement, faites **clic droit → Ouvrir**.

## Action rapide du Finder

1. Ouvrez l'app **Raccourcis** et créez un raccourci.
2. Ajoutez l'action **Compresser des PDF** (catégorie Apps → PDF Compress).
3. Dans le panneau d'informations du raccourci (ⓘ), cochez **Utiliser comme action rapide** et **Finder**, et réglez l'entrée reçue sur **Fichiers PDF**.
4. Dans le Finder, sélectionnez des PDF puis clic droit → **Actions rapides** → votre raccourci.

## Développer

Prérequis : Xcode 16 ou plus récent, Rust, `brew install ghostscript xcodegen dylibbundler`.

```sh
cd engine && cargo test          # tests du moteur (Ghostscript requis)
cd app && xcodegen && open PDFCompress.xcodeproj
```

Xcode compile le moteur Rust avant l'application (`scripts/build-engine.sh`). En développement, l'application utilise la libgs de Homebrew.

Construire une version autonome :

```sh
xcodebuild -project app/PDFCompress.xcodeproj -scheme PDFCompress -configuration Release -derivedDataPath app/build build
APP="app/build/Build/Products/Release/PDF Compress.app"
scripts/bundle-ghostscript.sh "$APP"   # embarque libgs et ses ressources
scripts/check-bundle.sh "$APP"         # vérifie la compression sans Homebrew
scripts/package.sh "$APP" PDF-Compress.dmg
```

## Organisation

- `engine/` : moteur Rust (bibliothèque statique) et son interface C `include/pdfcompress.h`.
- `app/` : application SwiftUI, action App Intents et projet XcodeGen (`project.yml`).
- `scripts/` : compilation du moteur, intégration de Ghostscript, signature et notarisation.
- `docs/signature.md` : configuration de la signature Developer ID.
