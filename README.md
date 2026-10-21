# PDF Compress

Petite application macOS pour compresser des fichiers PDF par glisser-déposer, et depuis Raccourcis ou le Finder.

- Déposez un ou plusieurs PDF (ou des dossiers) dans la fenêtre ou sur l'icône de l'application : ils sont compressés **sur place**, sans copie ni question.
- L'original n'est remplacé que si le résultat est plus léger ; le remplacement passe par un fichier temporaire et un renommage atomique, l'original n'est donc jamais laissé à moitié écrit. Un PDF protégé par mot de passe est laissé intact.
- Niveau par défaut **Équilibré** (images à 150 dpi, qualité JPEG 70). Les préférences (⌘,) proposent Forte, Légère ou des réglages personnalisés.
- Une Action rapide du Finder compresse les PDF sélectionnés sans ouvrir la fenêtre, avec les mêmes réglages.

L'interface est en SwiftUI ; le moteur, en Rust, pilote [Ghostscript](https://www.ghostscript.com/) en tant que bibliothèque, embarquée dans l'application : rien à installer.

## Installer

1. Ouvrez l'onglet **Actions** du dépôt, choisissez la dernière exécution réussie du workflow **Build** sur `main` et téléchargez l'artefact **PDF-Compress-macOS**.
2. Décompressez-le, ouvrez le `.dmg` et glissez **PDF Compress** dans **Applications**.

L'application cible les Mac à puce Apple sous macOS 14 ou plus récent. Elle est signée et notarisée lorsque les secrets décrits dans [docs/signature.md](docs/signature.md) sont configurés ; sinon, au premier lancement, faites **clic droit → Ouvrir**.

## Action rapide du Finder

Méthode recommandée, qui fonctionne sans signature Apple :

1. Ouvrez l'app **Raccourcis** et créez un raccourci nommé par exemple « Compresser avec PDF Compress ».
2. Dans son panneau d'informations (ⓘ), cochez **Utiliser comme action rapide** et **Finder**, et réglez l'entrée reçue sur **Fichiers PDF** et **Dossiers**.
3. Ajoutez l'action **Exécuter un script shell**, choisissez **Transmettre l'entrée : comme arguments**, et collez le contenu de [scripts/action-rapide.sh](scripts/action-rapide.sh).
4. Dans le Finder, sélectionnez des PDF ou des dossiers, puis clic droit → **Actions rapides** → votre raccourci. Une notification résume le gain.

Le script appelle le mode ligne de commande de l'application, utilisable aussi dans le Terminal :
`"/Applications/PDF Compress.app/Contents/MacOS/PDF Compress" --compress fichier.pdf dossier…`

L'action **Compresser des PDF** est aussi proposée directement dans Raccourcis (App Intents). Si Raccourcis indique ne pas pouvoir communiquer avec l'app, utilisez la méthode ci-dessus.

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
