// Compresse un PDF via l'interface C du moteur (utilisé par scripts/check-bundle.sh).
// Usage : selftest fichier.pdf dossier-ressources-gs
#include <stdio.h>
#include "pdfcompress.h"

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage : %s fichier.pdf dossier-ressources\n", argv[0]);
        return 2;
    }
    char *version = pc_engine_version();
    printf("Ghostscript %s\n", version ? version : "inconnu");
    pc_string_free(version);

    PCSettings settings = {PCPresetBalanced, 0, 0};
    PCOutcome out = pc_compress_in_place(argv[1], settings, argv[2]);
    if (out.status != PCStatusOK) {
        fprintf(stderr, "Échec : %s\n", out.message ? out.message : "inconnu");
        pc_string_free(out.message);
        return 1;
    }
    printf("%llu -> %llu octets, remplacé : %s\n", (unsigned long long)out.original_size,
           (unsigned long long)out.new_size, out.replaced ? "oui" : "non");
    return out.replaced && out.new_size < out.original_size ? 0 : 1;
}
