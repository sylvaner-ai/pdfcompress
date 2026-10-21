use crate::compress::{self, Engine, Outcome};
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

const MAX_PARALLEL: usize = 3;

/// Compresse les PDF désignés (fichiers ou dossiers), jusqu'à trois à la fois.
/// Renvoie, dans l'ordre, le résultat de chaque fichier avec son chemin.
pub fn compress_all(engine: Option<&Engine>, paths: &[PathBuf], settings: &Settings) -> Vec<(PathBuf, Result<Outcome, String>)> {
    let pdfs = compress::collect_pdfs(paths);
    let Some(engine) = engine else {
        return pdfs.into_iter().map(|p| (p, Err("Ghostscript est introuvable.".to_string()))).collect();
    };
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(pdfs.len()));
    thread::scope(|s| {
        for _ in 0..MAX_PARALLEL.min(pdfs.len()) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = pdfs.get(i) else { break };
                let r = compress::compress_in_place(engine, path, settings);
                results.lock().unwrap().push((i, path.clone(), r));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(i, ..)| *i);
    results.into_iter().map(|(_, p, r)| (p, r)).collect()
}

pub fn format_size(bytes: u64) -> String {
    let units = ["o", "Ko", "Mo", "Go"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    let n = if i == 0 { format!("{v:.0}") } else { format!("{v:.1}").trim_end_matches(".0").replace('.', ",") };
    format!("{n} {}", units[i])
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n > 1 { "s" } else { "" })
}

/// Texte de la notification affichée à la fin d'une compression sans fenêtre.
pub fn summary(results: &[(PathBuf, Result<Outcome, String>)]) -> String {
    if results.is_empty() {
        return "Aucun fichier PDF dans la sélection.".into();
    }
    let (mut before, mut after, mut replaced, mut same) = (0u64, 0u64, 0usize, 0usize);
    let mut failures = Vec::new();
    for (path, r) in results {
        match r {
            Ok(o) => {
                before += o.original_size;
                after += o.new_size;
                if o.replaced { replaced += 1 } else { same += 1 }
            }
            Err(e) => failures.push((path, e)),
        }
    }
    let mut parts = Vec::new();
    if replaced > 0 {
        let saved = before - after;
        let pct = (saved as f64 / before as f64 * 100.0).round() as u64;
        parts.push(format!("{} · {} gagnés (−{pct} %)", plural(replaced, "PDF compressé"), format_size(saved)));
    }
    if same > 0 {
        parts.push(format!("{} déjà {}", plural(same, "PDF"), if same > 1 { "optimisés" } else { "optimisé" }));
    }
    match failures.as_slice() {
        [] => {}
        [(path, err)] => parts.push(format!("Échec pour {} : {err}", file_name(path))),
        _ => parts.push(plural(failures.len(), "échec")),
    }
    parts.join("\n")
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Notification macOS via AppleScript : aucune autorisation ni dépendance supplémentaire.
/// Les textes sont passés en arguments, donc sans échappement à gérer.
pub fn notify(title: &str, body: &str) {
    if cfg!(target_os = "macos") {
        let _ = Command::new("osascript")
            .args(["-e", "on run argv", "-e", "display notification (item 2 of argv) with title (item 1 of argv)", "-e", "end run", title, body])
            .status();
    } else {
        println!("{title} — {body}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(original: u64, new: u64) -> (PathBuf, Result<Outcome, String>) {
        let outcome = Outcome { path: "a.pdf".into(), original_size: original, new_size: new, replaced: new < original };
        (PathBuf::from("a.pdf"), Ok(outcome))
    }

    #[test]
    fn formats_sizes_in_french() {
        assert_eq!(format_size(512), "512 o");
        assert_eq!(format_size(2048), "2 Ko");
        assert_eq!(format_size(1536 * 1024), "1,5 Mo");
    }

    #[test]
    fn summarises_results() {
        assert_eq!(summary(&[]), "Aucun fichier PDF dans la sélection.");
        assert_eq!(summary(&[ok(4096, 1024)]), "1 PDF compressé · 3 Ko gagnés (−75 %)");
        let failed = (PathBuf::from("/x/secret.pdf"), Err("PDF protégé par mot de passe.".to_string()));
        assert_eq!(
            summary(&[ok(4096, 1024), ok(2048, 1024), ok(100, 100), failed]),
            "2 PDF compressés · 4 Ko gagnés (−66 %)\n1 PDF déjà optimisé\nÉchec pour secret.pdf : PDF protégé par mot de passe."
        );
    }
}
