use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Niveaux de compression proposés dans les préférences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Strong,
    Balanced,
    Light,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub preset: Preset,
    /// Résolution cible des images (dpi), utilisée en mode personnalisé.
    pub image_dpi: u32,
    /// Qualité JPEG des images (1 à 100), utilisée en mode personnalisé.
    pub jpeg_quality: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self { preset: Preset::Balanced, image_dpi: 150, jpeg_quality: 70 }
    }
}

impl Settings {
    /// Paramètres réellement appliqués (dpi, qualité JPEG) selon le niveau choisi.
    pub fn effective(&self) -> (u32, u8) {
        match self.preset {
            Preset::Strong => (100, 50),
            Preset::Balanced => (150, 70),
            Preset::Light => (220, 85),
            Preset::Custom => (self.image_dpi.clamp(50, 600), self.jpeg_quality.clamp(10, 100)),
        }
    }

    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }
}
