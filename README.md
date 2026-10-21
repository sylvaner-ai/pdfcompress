# PDF Compress

Petite application macOS pour compresser des fichiers PDF par glisser-déposer.

- Déposez un ou plusieurs PDF (ou des dossiers) dans la fenêtre : ils sont compressés **sur place**, sans copie ni question.
- L'original n'est remplacé que si le résultat est plus léger ; le remplacement passe par un fichier temporaire et un renommage atomique, l'original n'est donc jamais laissé à moitié écrit.
- Niveau par défaut **Équilibré** (images à 150 dpi, qualité JPEG 70). Les préférences (⌘,) proposent Forte, Légère ou des réglages personnalisés, mémorisés entre les sessions.

## Compresser sans ouvrir la fenêtre

Lorsque des PDF sont transmis à l'application au lancement, elle les compresse sans rien afficher (ni fenêtre, ni icône dans le Dock), présente une notification avec le résultat, puis se ferme. Si la fenêtre est déjà ouverte, les fichiers rejoignent simplement sa liste.

- **Icône du Dock** : gardez PDF Compress dans le Dock et déposez-y vos PDF. L'application accepte les PDF déposés mais ne se déclare jamais comme lecteur PDF par défaut.
- **Action rapide du Finder** (clic droit sur des PDF → Actions rapides) : ouvrez Automator, choisissez **Nouveau document → Action rapide**, réglez « Le processus reçoit » sur **fichiers PDF** dans **Finder**, ajoutez l'action **Exécuter un script Shell** avec « Données en entrée » sur **comme arguments** et le script :
  `open -g -a "PDF Compress" "$@"`
  Enregistrez-la sous le nom « Compresser avec PDF Compress ».
- **Ligne de commande** : `open -g -a "PDF Compress" fichier.pdf`.

La notification est émise par AppleScript : macOS l'attribue à « Éditeur de script » et peut demander une fois l'autorisation de l'afficher.

Le moteur est [Ghostscript](https://www.ghostscript.com/), embarqué dans l'application : rien à installer.

## Installer

1. Ouvrez l'onglet **Actions** du dépôt, choisissez la dernière exécution réussie du workflow **Build** sur `main` et téléchargez l'artefact **PDF-Compress-macOS**.
2. Décompressez-le, ouvrez le `.dmg` et glissez **PDF Compress** dans **Applications**.
3. L'application n'est pas signée par Apple : au premier lancement, faites **clic droit → Ouvrir**, puis confirmez. Si macOS indique que l'application est « endommagée », exécutez une fois :
   `xattr -dr com.apple.quarantine "/Applications/PDF Compress.app"`

La version construite cible les Mac à puce Apple (M1 et suivants).

## Développer

Prérequis : Rust, Node.js 22 et `brew install ghostscript` (en développement, l'application utilise le Ghostscript de Homebrew).

```sh
npm install
npm run dev        # lance l'application
cd src-tauri && cargo test   # tests du moteur de compression
```

Construire l'application avec Ghostscript embarqué :

```sh
brew install dylibbundler
scripts/bundle-ghostscript.sh
npm run tauri build -- --config "$PWD/src-tauri/tauri.bundle-gs.conf.json"
```

## Organisation

- `src/` : interface (HTML, CSS, JavaScript sans framework).
- `src-tauri/src/compress.rs` : appel à Ghostscript, remplacement sûr du fichier, collecte des PDF déposés.
- `src-tauri/src/settings.rs` : niveaux de compression et préférences enregistrées.
- `src-tauri/src/background.rs` : compression sans fenêtre et notification de fin.
- `scripts/` : préparation et vérification du Ghostscript embarqué.
- `.github/workflows/build.yml` : tests, puis construction du `.dmg` sur un Mac.

## Licence de Ghostscript

Ghostscript est distribué sous licence AGPL. Un usage personnel ne pose pas de difficulté ; une diffusion publique de l'application imposerait de respecter cette licence ou d'acquérir une licence commerciale auprès d'Artifex.
