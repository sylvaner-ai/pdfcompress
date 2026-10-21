use crate::gs;
use crate::settings::Settings;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub original_size: u64,
    pub new_size: u64,
    /// Faux lorsque le fichier était déjà optimisé : l'original est alors laissé intact.
    pub replaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotPdf = 1,
    Password = 2,
    Failed = 3,
    Io = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

fn err(kind: ErrorKind, message: impl Into<String>) -> Error {
    Error { kind, message: message.into() }
}

/// Facteur de quantification JPEG (QFactor) correspondant à une qualité de 1 à 100,
/// selon la même échelle que libjpeg : 50 → 1,0 ; 75 → 0,5 ; 90 → 0,2.
fn qfactor(quality: u32) -> f32 {
    let q = quality.clamp(1, 100) as f32;
    let scale = if q < 50.0 { 5000.0 / q } else { 200.0 - 2.0 * q };
    (scale / 100.0).max(0.05)
}

/// Dossiers de ressources de la copie de Ghostscript embarquée dans l'application
/// (`Contents/Resources/gs`), dans l'ordre attendu par Ghostscript.
fn search_path(resource_dir: &Path) -> Option<String> {
    let parts: Vec<String> = ["Resource/Init", "lib", "Resource/Font", "iccprofiles", "fonts"]
        .iter()
        .map(|p| resource_dir.join(p))
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join(":"))
}

fn gs_args(settings: &Settings, input: &Path, output: &Path, resource_dir: Option<&Path>) -> Vec<String> {
    let (dpi, quality) = settings.effective();
    let mono_dpi = (dpi * 2).max(300);
    let qf = qfactor(quality);
    let dict = format!("<< /QFactor {qf:.2} /Blend 1 /HSamples [2 1 1 2] /VSamples [2 1 1 2] >>");
    let mut args: Vec<String> = Vec::new();
    if let Some(path) = resource_dir.and_then(search_path) {
        args.push(format!("-I{path}"));
    }
    args.extend(
        [
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
        .map(|s| s.to_string()),
    );
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

pub(crate) fn is_pdf(path: &Path) -> bool {
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
pub fn compress_in_place(path: &Path, settings: &Settings, resource_dir: Option<&Path>) -> Result<Outcome, Error> {
    let meta = fs::metadata(path).map_err(|e| err(ErrorKind::Io, format!("Fichier inaccessible : {e}")))?;
    if !meta.is_file() || !is_pdf(path) {
        return Err(err(ErrorKind::NotPdf, "Ce fichier n'est pas un PDF."));
    }
    let original_size = meta.len();
    let tmp = temp_path_for(path);

    let result = (|| {
        // Sur un PDF chiffré ou illisible, Ghostscript peut réussir en produisant un PDF
        // sans page : ses messages sont donc vérifiés même en cas de succès.
        let (ok, log) = match gs::run(&gs_args(settings, path, &tmp, resource_dir)) {
            Ok(log) => (true, log.to_lowercase()),
            Err(log) => (false, log.to_lowercase()),
        };
        if log.contains("password") {
            return Err(err(ErrorKind::Password, "PDF protégé par mot de passe."));
        }
        if !ok || log.contains("no pages will be processed") {
            return Err(err(ErrorKind::Failed, "Échec de la compression (PDF peut-être endommagé)."));
        }
        let new_size = fs::metadata(&tmp).map_err(|e| err(ErrorKind::Io, e.to_string()))?.len();
        if new_size == 0 || !is_pdf(&tmp) {
            return Err(err(ErrorKind::Failed, "Ghostscript a produit un fichier invalide."));
        }
        if new_size >= original_size {
            return Ok(Outcome {
                original_size,
                new_size: original_size,
                replaced: false,
            });
        }
        fs::set_permissions(&tmp, meta.permissions()).map_err(|e| err(ErrorKind::Io, e.to_string()))?;
        fs::rename(&tmp, path).map_err(|e| err(ErrorKind::Io, format!("Remplacement impossible : {e}")))?;
        Ok(Outcome {
            original_size,
            new_size,
            replaced: true,
        })
    })();

    let _ = fs::remove_file(&tmp);
    result
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

    #[test]
    fn version_is_reported() {
        let v = gs::version().expect("version de libgs");
        assert!(v.starts_with(char::is_numeric), "{v}");
    }

    #[test]
    fn compresses_sample_in_place() {
        let dir = std::env::temp_dir().join(format!("pdfcompress-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("échantillon à compresser.pdf");
        let ps = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.ps");
        gs::run(&[
            "-sDEVICE=pdfwrite".into(),
            "-dNOPAUSE".into(),
            "-dBATCH".into(),
            "-dQUIET".into(),
            "-dAutoFilterColorImages=false".into(),
            "-dColorImageFilter=/FlateEncode".into(),
            format!("-sOutputFile={}", pdf.display()),
            ps.display().to_string(),
        ])
        .unwrap();

        let before = fs::metadata(&pdf).unwrap().len();
        let outcome = compress_in_place(&pdf, &Settings::default(), None).unwrap();
        assert!(outcome.replaced);
        assert_eq!(outcome.original_size, before);
        assert_eq!(fs::metadata(&pdf).unwrap().len(), outcome.new_size);
        assert!(outcome.new_size < before / 2, "{} -> {}", before, outcome.new_size);
        assert!(is_pdf(&pdf));

        // Un second passage ne doit pas dégrader ni grossir le fichier.
        let again = compress_in_place(&pdf, &Settings::default(), None).unwrap();
        assert!(again.new_size <= outcome.new_size);

        // Aucun fichier temporaire ne doit subsister.
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reports_password_protected_pdf() {
        let dir = std::env::temp_dir().join(format!("pdfcompress-pw-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("protégé.pdf");
        let ps = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.ps");
        gs::run(&[
            "-sDEVICE=pdfwrite".into(),
            "-dNOPAUSE".into(),
            "-dBATCH".into(),
            "-dQUIET".into(),
            "-sOwnerPassword=proprietaire".into(),
            "-sUserPassword=secret".into(),
            format!("-sOutputFile={}", pdf.display()),
            ps.display().to_string(),
        ])
        .unwrap();
        let content = fs::read(&pdf).unwrap();
        let e = compress_in_place(&pdf, &Settings::default(), None).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Password);
        assert_eq!(fs::read(&pdf).unwrap(), content);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_non_pdf() {
        let f = std::env::temp_dir().join(format!("pdfcompress-notpdf-{}.pdf", std::process::id()));
        fs::write(&f, b"bonjour").unwrap();
        let e = compress_in_place(&f, &Settings::default(), None).unwrap_err();
        assert_eq!(e.kind, ErrorKind::NotPdf);
        assert_eq!(fs::read(&f).unwrap(), b"bonjour");
        fs::remove_file(&f).unwrap();
    }

    #[test]
    fn reports_damaged_pdf_without_touching_it() {
        let f = std::env::temp_dir().join(format!("pdfcompress-damaged-{}.pdf", std::process::id()));
        let content = b"%PDF-1.4\n1 0 obj << /Type /Catalog >> garbage".to_vec();
        fs::write(&f, &content).unwrap();
        // Ghostscript peut échouer ou produire un fichier vide : dans tous les cas l'original reste intact.
        let _ = compress_in_place(&f, &Settings::default(), None);
        assert_eq!(fs::read(&f).unwrap(), content);
        fs::remove_file(&f).unwrap();
    }
}
