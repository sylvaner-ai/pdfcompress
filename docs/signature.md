# Signature et notarisation

Le workflow **Build** signe l'application avec votre certificat **Developer ID** et la fait notariser par Apple dès que les six secrets ci-dessous existent dans le dépôt. Sans eux, il produit une application signée ad hoc (non notarisée), comme la version 1.

Ne collez jamais ces valeurs ailleurs que dans les secrets GitHub.

| Secret | Contenu |
| --- | --- |
| `MACOS_CERTIFICATE_P12` | Certificat « Developer ID Application » et sa clé privée, exportés en `.p12` puis encodés en base64 |
| `MACOS_CERTIFICATE_PASSWORD` | Mot de passe choisi lors de l'export du `.p12` |
| `APPLE_TEAM_ID` | Identifiant d'équipe Apple Developer (10 caractères) |
| `APPSTORE_API_KEY_ID` | Identifiant de la clé API App Store Connect |
| `APPSTORE_API_ISSUER_ID` | Identifiant de l'émetteur (Issuer ID) de cette clé |
| `APPSTORE_API_KEY_P8` | Fichier `AuthKey_XXXXXXXXXX.p8` de la clé, encodé en base64 |

## 1. Certificat Developer ID Application

Seul le titulaire du compte (Account Holder) peut créer ce type de certificat.

1. Sur votre Mac, ouvrez **Xcode → Réglages → Comptes**, sélectionnez votre identifiant Apple puis votre équipe, et cliquez sur **Gérer les certificats…**
2. Cliquez sur **+** puis **Developer ID Application**. Si le certificat existe déjà, passez à l'étape suivante.
3. Ouvrez **Trousseau d'accès**, catégorie **Mes certificats**, faites un clic droit sur **Developer ID Application : … (TEAMID)** puis **Exporter…**, au format `.p12`. Choisissez un mot de passe : ce sera `MACOS_CERTIFICATE_PASSWORD`.
4. Dans le Terminal, copiez le fichier encodé dans le presse-papiers :
   `base64 -i ~/Desktop/certificat.p12 | pbcopy`
   C'est la valeur de `MACOS_CERTIFICATE_P12`.

## 2. Identifiant d'équipe

Sur [developer.apple.com/account](https://developer.apple.com/account), section **Membership details**, relevez le **Team ID** : c'est `APPLE_TEAM_ID`. Il apparaît aussi entre parenthèses dans le nom du certificat.

## 3. Clé API pour la notarisation

1. Sur [App Store Connect](https://appstoreconnect.apple.com), ouvrez **Utilisateurs et accès → Intégrations → API App Store Connect → Clés d'équipe**.
2. Créez une clé (nom libre, par exemple « Notarisation PDF Compress », rôle **Developer**).
3. Relevez l'**ID de la clé** (`APPSTORE_API_KEY_ID`) et l'**ID de l'émetteur** affiché en haut de la page (`APPSTORE_API_ISSUER_ID`).
4. Téléchargez le fichier `AuthKey_XXXXXXXXXX.p8` (téléchargeable une seule fois), puis :
   `base64 -i ~/Downloads/AuthKey_XXXXXXXXXX.p8 | pbcopy`
   C'est la valeur de `APPSTORE_API_KEY_P8`.

## 4. Enregistrer les secrets dans GitHub

Dans le dépôt : **Settings → Secrets and variables → Actions → New repository secret**, une fois par secret, avec exactement les noms du tableau.

Variante en ligne de commande, sans passer par le presse-papiers (avec [GitHub CLI](https://cli.github.com)) :

```sh
cd chemin/vers/pdfcompress
base64 -i ~/Desktop/certificat.p12 | gh secret set MACOS_CERTIFICATE_P12
gh secret set MACOS_CERTIFICATE_PASSWORD   # demande la valeur
gh secret set APPLE_TEAM_ID
gh secret set APPSTORE_API_KEY_ID
gh secret set APPSTORE_API_ISSUER_ID
base64 -i ~/Downloads/AuthKey_XXXXXXXXXX.p8 | gh secret set APPSTORE_API_KEY_P8
```

Supprimez ensuite le `.p12` exporté et rangez le `.p8` en lieu sûr.

## 5. Vérifier

Relancez le workflow **Build** (onglet **Actions → Build → Run workflow**). L'étape « Signer, notariser et créer le .dmg » doit se terminer par « Application signée et notarisée ». Le `.dmg` téléchargé s'ouvre alors sans avertissement de macOS.
