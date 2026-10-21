# Script de l'Action rapide « Compresser avec PDF Compress » (app Raccourcis, action
# « Exécuter un script shell », entrée transmise « comme arguments »).
# Il appelle le mode ligne de commande de l'application, avec les réglages de ses Préférences,
# puis affiche le résumé dans une notification.
APP="/Applications/PDF Compress.app/Contents/MacOS/PDF Compress"
if [ ! -x "$APP" ]; then
  RESUME="PDF Compress est introuvable dans le dossier Applications."
else
  RESUME="$("$APP" --compress "$@" 2>/dev/null | tail -n 1)"
fi
osascript -e 'on run argv' -e 'display notification (item 1 of argv) with title "PDF Compress"' -e 'end run' "$RESUME"
