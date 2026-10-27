# Script de l'Action rapide « Compresser avec PDF Compress » (Automator ou Raccourcis, action
# « Exécuter un script shell », entrée transmise « comme arguments »).
# Il appelle le mode ligne de commande de l'application, avec les réglages de ses Préférences,
# puis affiche un message de fin avec le résumé. Le script n'écrit rien en sortie : dans une
# Action rapide, le Finder enregistrerait ce texte dans un fichier à côté des PDF.
APP="/Applications/PDF Compress.app/Contents/MacOS/PDF Compress"
if [ ! -x "$APP" ]; then
  RESUME="PDF Compress est introuvable dans le dossier Applications."
else
  RESUME="$("$APP" --compress "$@" 2>/dev/null | tail -n 1)"
fi
osascript -e 'on run argv' -e 'display alert "Compression terminée" message (item 1 of argv)' -e 'end run' "$RESUME" > /dev/null
