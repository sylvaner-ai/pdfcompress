// Interface C du moteur Rust de PDF Compress (engine/src/lib.rs).
#ifndef PDFCOMPRESS_H
#define PDFCOMPRESS_H

#include <stdbool.h>
#include <stdint.h>

typedef enum {
    PCPresetStrong = 0,
    PCPresetBalanced = 1,
    PCPresetLight = 2,
    PCPresetCustom = 3,
} PCPreset;

typedef enum {
    PCStatusOK = 0,
    PCStatusNotPDF = 1,
    PCStatusPassword = 2,
    PCStatusFailed = 3,
    PCStatusIO = 4,
} PCStatus;

typedef struct {
    uint32_t preset;        // PCPreset
    uint32_t image_dpi;     // utilisé si preset == PCPresetCustom
    uint32_t jpeg_quality;  // utilisé si preset == PCPresetCustom
} PCSettings;

typedef struct {
    int32_t status;         // PCStatus
    uint64_t original_size;
    uint64_t new_size;
    bool replaced;          // faux si le résultat n'était pas plus léger
    char *message;          // erreur en UTF-8, à libérer avec pc_string_free
} PCOutcome;

// Compresse le PDF sur place ; l'original n'est remplacé que si le résultat est plus léger.
// resource_dir : ressources du Ghostscript embarqué (Contents/Resources/gs), ou NULL.
// Les appels sont sérialisés par le moteur : appeler hors du fil principal.
PCOutcome pc_compress_in_place(const char *path, PCSettings settings, const char *resource_dir);

// Résolution et qualité JPEG réellement appliquées pour ces réglages.
void pc_effective_settings(PCSettings settings, uint32_t *image_dpi, uint32_t *jpeg_quality);

// Version de Ghostscript (à libérer avec pc_string_free), ou NULL.
char *pc_engine_version(void);

void pc_string_free(char *s);

#endif
