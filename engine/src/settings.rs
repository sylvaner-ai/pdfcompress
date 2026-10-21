/// Niveaux de compression proposés dans les préférences.
/// Les valeurs numériques sont celles de l'interface C (`PCPreset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Strong = 0,
    Balanced = 1,
    Light = 2,
    Custom = 3,
}

impl Preset {
    pub fn from_raw(v: u32) -> Preset {
        match v {
            0 => Preset::Strong,
            2 => Preset::Light,
            3 => Preset::Custom,
            _ => Preset::Balanced,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub preset: Preset,
    /// Résolution cible des images (dpi), utilisée en mode personnalisé.
    pub image_dpi: u32,
    /// Qualité JPEG des images (1 à 100), utilisée en mode personnalisé.
    pub jpeg_quality: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: Preset::Balanced,
            image_dpi: 150,
            jpeg_quality: 70,
        }
    }
}

impl Settings {
    /// Paramètres réellement appliqués (dpi, qualité JPEG) selon le niveau choisi.
    pub fn effective(&self) -> (u32, u32) {
        match self.preset {
            Preset::Strong => (100, 50),
            Preset::Balanced => (150, 70),
            Preset::Light => (220, 85),
            Preset::Custom => (self.image_dpi.clamp(50, 600), self.jpeg_quality.clamp(10, 100)),
        }
    }
}
