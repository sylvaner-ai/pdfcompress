//! Moteur de compression de PDF Compress.
//!
//! Le cœur (`compress`) pilote Ghostscript en tant que bibliothèque ; ce fichier expose
//! l'interface C appelée par l'application Swift, décrite dans `include/pdfcompress.h`.

mod compress;
mod gs;
mod settings;

pub use compress::{compress_in_place, Error, ErrorKind, Outcome};
pub use settings::{Preset, Settings};

use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};

/// Résultat d'une compression, tel que vu par Swift (`PCOutcome`).
#[repr(C)]
pub struct PCOutcome {
    /// 0 : succès ; sinon un `PCStatus` d'erreur.
    pub status: i32,
    pub original_size: u64,
    pub new_size: u64,
    pub replaced: bool,
    /// Message d'erreur en UTF-8 (nul en cas de succès), à libérer avec `pc_string_free`.
    pub message: *mut c_char,
}

/// Réglages transmis par Swift (`PCSettings`).
#[repr(C)]
pub struct PCSettings {
    pub preset: u32,
    pub image_dpi: u32,
    pub jpeg_quality: u32,
}

impl From<&PCSettings> for Settings {
    fn from(s: &PCSettings) -> Self {
        Settings {
            preset: Preset::from_raw(s.preset),
            image_dpi: s.image_dpi,
            jpeg_quality: s.jpeg_quality,
        }
    }
}

unsafe fn str_from_c(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| CStr::from_ptr(p).to_string_lossy().into_owned())
}

fn into_c_string(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "")).map(CString::into_raw).unwrap_or(std::ptr::null_mut())
}

/// Compresse `path` sur place. `resource_dir` (facultatif) désigne les ressources de la
/// copie de Ghostscript embarquée dans l'application.
///
/// # Safety
/// `path` doit être une chaîne C valide ; `resource_dir` est nul ou une chaîne C valide.
#[no_mangle]
pub unsafe extern "C" fn pc_compress_in_place(path: *const c_char, settings: PCSettings, resource_dir: *const c_char) -> PCOutcome {
    let fail = |kind: ErrorKind, message: String| PCOutcome {
        status: kind as i32,
        original_size: 0,
        new_size: 0,
        replaced: false,
        message: into_c_string(message),
    };
    let Some(path) = str_from_c(path) else {
        return fail(ErrorKind::Io, "Chemin manquant.".into());
    };
    let resource_dir = str_from_c(resource_dir).map(PathBuf::from);
    match compress_in_place(Path::new(&path), &Settings::from(&settings), resource_dir.as_deref()) {
        Ok(o) => PCOutcome {
            status: 0,
            original_size: o.original_size,
            new_size: o.new_size,
            replaced: o.replaced,
            message: std::ptr::null_mut(),
        },
        Err(e) => fail(e.kind, e.message),
    }
}

/// Résolution (dpi) et qualité JPEG réellement appliquées pour ces réglages.
///
/// # Safety
/// `image_dpi` et `jpeg_quality` sont nuls ou pointent vers des entiers modifiables.
#[no_mangle]
pub unsafe extern "C" fn pc_effective_settings(settings: PCSettings, image_dpi: *mut u32, jpeg_quality: *mut u32) {
    let (dpi, quality) = Settings::from(&settings).effective();
    if !image_dpi.is_null() {
        *image_dpi = dpi;
    }
    if !jpeg_quality.is_null() {
        *jpeg_quality = quality;
    }
}

/// Version de Ghostscript, à libérer avec `pc_string_free` ; nul si indisponible.
#[no_mangle]
pub extern "C" fn pc_engine_version() -> *mut c_char {
    gs::version().map(into_c_string).unwrap_or(std::ptr::null_mut())
}

/// Libère une chaîne renvoyée par le moteur.
///
/// # Safety
/// `s` doit provenir du moteur (ou être nul) et n'être libérée qu'une fois.
#[no_mangle]
pub unsafe extern "C" fn pc_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_interface_reports_errors_and_settings() {
        let f = std::env::temp_dir().join(format!("pdfcompress-ffi-{}.pdf", std::process::id()));
        std::fs::write(&f, b"pas un pdf").unwrap();
        let path = CString::new(f.to_str().unwrap()).unwrap();
        let settings = PCSettings {
            preset: 1,
            image_dpi: 0,
            jpeg_quality: 0,
        };
        let out = unsafe { pc_compress_in_place(path.as_ptr(), settings, std::ptr::null()) };
        assert_eq!(out.status, ErrorKind::NotPdf as i32);
        assert!(!out.message.is_null());
        let msg = unsafe { CStr::from_ptr(out.message) }.to_str().unwrap().to_string();
        assert!(msg.contains("PDF"));
        unsafe { pc_string_free(out.message) };
        std::fs::remove_file(&f).unwrap();

        let (mut dpi, mut q) = (0, 0);
        unsafe {
            pc_effective_settings(
                PCSettings {
                    preset: 3,
                    image_dpi: 1000,
                    jpeg_quality: 5,
                },
                &mut dpi,
                &mut q,
            )
        };
        assert_eq!((dpi, q), (600, 10));
        unsafe {
            pc_effective_settings(
                PCSettings {
                    preset: 1,
                    image_dpi: 0,
                    jpeg_quality: 0,
                },
                &mut dpi,
                &mut q,
            )
        };
        assert_eq!((dpi, q), (150, 70));
    }
}
