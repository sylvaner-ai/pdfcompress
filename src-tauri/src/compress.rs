use crate::settings::Settings;
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Exécutable Ghostscript à utiliser, avec son répertoire de ressources éventuel.
#[derive(Debug, Clone)]
pub struct Engine {
    pub exe: PathBuf,
    /// Valeur de GS_LIB pour la version embarquée dans l'application.
    pub gs_lib: Option<String>,
}

impl Engine {
    /// Cherche d'abord la version embarquée, puis une installation Homebrew ou système.
    /// Les applications macOS lancées depuis le Finder n'héritent pas du PATH du shell,
    /// d'où la liste explicite d'emplacements.
    pub fn locate(resource_dir: Option<&Path>) -> Option<Engine> {
        if let Some(dir) = resource_dir {
            let root = dir.join("gs");
            let exe = root.join("bin").join("gs");
            if exe.is_file() {
                let share = root.join("share");
                let parts: Vec<String> = ["Resource/Init", "lib", "Resource/Font", "iccprofiles", "fonts"]
                    .iter()
                    .map(|p| share.join(p))
                    .filter(|p| p.is_dir())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                let gs_lib = (!parts.is_empty()).then(|| parts.join(":"));
                return Some(Engine { exe, gs_lib });
            }
        }
        ["/opt/homebrew/bin/gs", "/usr/local/bin/gs", "/usr/bin/gs"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
            .or_else(|| Command::new("gs").arg("--version").output().ok().filter(|o| o.status.success()).map(|_| PathBuf::from("gs")))
            .map(|exe| Engine { exe, gs_lib: None })
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.exe);
        if let Some(lib) = &self.gs_lib {
            cmd.env("GS_LIB", lib);
        }
        cmd.stdin(Stdio::null());
        cmd
    }

    pub fn version(&self) -> Option<String> {
        let out = self.command().arg("--version").output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub path: String,
    pub original_size: u64,
    pub new_size: u64,
    /// Faux lorsque le fichier était déjà optimisé : l'original est alors laissé intact.
    pub replaced: bool,
}

/// Facteur de quantification JPEG (QFactor) correspondant à une qualité de 1 à 100,
/// selon la même échelle que libjpeg : 50 → 1,0 ; 75 → 0,5 ; 90 → 0,2.
fn qfactor(quality: u8) -> f32 {
    let q = quality.clamp(1, 100) as f32;
    let scale = if q < 50.0 { 5000.0 / q } else { 200.0 - 2.0 * q };
    (scale / 100.0).max(0.05)
}

fn gs_args(settings: &Settings, input: &Path, output: &Path) -> Vec<String> {
    let (dpi, quality) = settings.effective();
    let mono_dpi = (dpi * 2).max(300);
    let qf = qfactor(quality);
    let dict = format!("<< /QFactor {qf:.2} /Blend 1 /HSamples [2 1 1 2] /VSamples [2 1 1 2] >>");
    let mut args: Vec<String> = [
        "-sDEVICE=pdfwrite",
        "-dCompatibilityLevel=1.6",
        "-dNOPAUSE",
        "-dBATCH",
        "-dQUIET",
        "-dSAFER",
        "-dPDFSETTINGS=/ebook",
        "-dDetectDuplicateImages=true",
        "-dCompressFonts=true",
        "-dSubsetFonts=true",
        "-dPassThroughJPEGImages=false",
        "-dDownsampleColorImages=true",
        "-dDownsampleGrayImages=true",
        "-dDownsampleMonoImages=true",
        "-dColorImageDownsampleType=/Bicubic",
        "-dGrayImageDownsampleType=/Bicubic",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(format!("-dColorImageResolution={dpi}"));
    args.push(format!("-dGrayImageResolution={dpi}"));
    args.push(format!("-dMonoImageResolution={mono_dpi}"));
    args.push(format!("-sOutputFile={}", output.display()));
    // Qualité JPEG appliquée aux images que Ghostscript choisit d'encoder en JPEG.
    args.push("-c".into());
    args.push(format!("<< /ColorACSImageDict {dict} /GrayACSImageDict {dict} >> setdistillerparams"));
    args.push("-f".into());
    args.push(input.display().to_string());
    args
}

fn is_pdf(path: &Path) -> bool {
    let mut header = [0u8; 1024];
    let Ok(mut f) = fs::File::open(path) else { return false };
    let n = f.read(&mut header).unwrap_or(0);
    header[..n].windows(5).any(|w| w == b"%PDF-")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path_for(path: &Path) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(".{stem}.pdfcompress-{}-{n}.tmp", std::process::id()))
}

/// Compresse le fichier et remplace l'original uniquement si le résultat est plus léger.
/// Le résultat est d'abord écrit dans un fichier temporaire du même dossier, puis
/// substitué par un renommage atomique : l'original n'est jamais laissé à moitié écrit.
pub fn compress_in_place(engine: &Engine, path: &Path, settings: &Settings) -> Result<Outcome, String> {
    let meta = fs::metadata(path).map_err(|e| format!("Fichier inaccessible : {e}"))?;
    if !meta.is_file() || !is_pdf(path) {
        return Err("Ce fichier n'est pas un PDF.".into());
    }
    let original_size = meta.len();
    let tmp = temp_path_for(path);

    let result = (|| {
        let out = engine
            .command()
            .args(gs_args(settings, path, &tmp))
            .output()
            .map_err(|e| format!("Impossible de lancer Ghostscript : {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return Err(if stderr.contains("password") || stdout.contains("password") {
                "PDF protégé par mot de passe.".to_string()
            } else {
                "Échec de la compression (PDF peut-être endommagé).".to_string()
            });
        }
        let new_size = fs::metadata(&tmp).map_err(|e| e.to_string())?.len();
        if new_size == 0 || !is_pdf(&tmp) {
            return Err("Ghostscript a produit un fichier invalide.".into());
        }
        if new_size >= original_size {
            return Ok(Outcome { path: path.display().to_string(), original_size, new_size: original_size, replaced: false });
        }
        fs::set_permissions(&tmp, meta.permissions()).map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| format!("Remplacement impossible : {e}"))?;
        Ok(Outcome { path: path.display().to_string(), original_size, new_size, replaced: true })
    })();

    let _ = fs::remove_file(&tmp);
    result
}

/// Développe les éléments déposés : fichiers PDF et dossiers (parcourus récursivement).
pub fn collect_pdfs(paths: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('.') && depth > 0 {
            return;
        }
        if path.is_dir() {
            if depth >= 8 {
                return;
            }
            if let Ok(entries) = fs::read_dir(path) {
                let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
                children.sort();
                for child in children {
                    walk(&child, out, depth + 1);
                }
            }
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) && !out.contains(&path.to_path_buf()) {
            out.push(path.to_path_buf());
        }
    }
    let mut out = Vec::new();
    for p in paths {
        walk(p, &mut out, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qfactor_follows_libjpeg_scale() {
        assert!((qfactor(50) - 1.0).abs() < 1e-6);
        assert!((qfactor(75) - 0.5).abs() < 1e-6);
        assert!(qfactor(95) < qfactor(70));
    }

    /// Nécessite Ghostscript sur la machine ; ignoré sinon.
    #[test]
    fn compresses_sample_in_place() {
        let Some(engine) = Engine::locate(None) else { return };
        let dir = std::env::temp_dir().join(format!("pdfcompress-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("échantillon.pdf");
        let ps = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/sample.ps");
        let status = engine
            .command()
            .args(["-sDEVICE=pdfwrite", "-dNOPAUSE", "-dBATCH", "-dQUIET", "-dAutoFilterColorImages=false", "-dColorImageFilter=/FlateEncode"])
            .arg(format!("-sOutputFile={}", pdf.display()))
            .arg(&ps)
            .status()
            .unwrap();
        assert!(status.success());

        let before = fs::metadata(&pdf).unwrap().len();
        let outcome = compress_in_place(&engine, &pdf, &Settings::default()).unwrap();
        assert!(outcome.replaced);
        assert_eq!(outcome.original_size, before);
        assert_eq!(fs::metadata(&pdf).unwrap().len(), outcome.new_size);
        assert!(outcome.new_size < before / 2, "{} -> {}", before, outcome.new_size);
        assert!(is_pdf(&pdf));

        // Un second passage ne doit pas dégrader ni grossir le fichier.
        let again = compress_in_place(&engine, &pdf, &Settings::default()).unwrap();
        assert!(again.new_size <= outcome.new_size);

        // Aucun fichier temporaire ne doit subsister.
        let leftovers: Vec<_> = fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).collect();
        assert!(leftovers.is_empty());
        assert_eq!(collect_pdfs(&[dir.clone()]), vec![pdf.clone()]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_non_pdf() {
        let Some(engine) = Engine::locate(None) else { return };
        let f = std::env::temp_dir().join(format!("pdfcompress-notpdf-{}.pdf", std::process::id()));
        fs::write(&f, b"bonjour").unwrap();
        assert!(compress_in_place(&engine, &f, &Settings::default()).is_err());
        assert_eq!(fs::read(&f).unwrap(), b"bonjour");
        fs::remove_file(&f).unwrap();
    }
}
