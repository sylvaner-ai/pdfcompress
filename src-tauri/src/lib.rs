mod background;
mod compress;
mod settings;

use compress::{Engine, Outcome};
use serde::Serialize;
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// Mode de fonctionnement, fixé au premier événement reçu.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Lancement en cours : on ne sait pas encore si des fichiers arrivent.
    Undecided,
    /// Fenêtre affichée : l'application reste ouverte.
    Window,
    /// Fichiers reçus au lancement (icône du Dock, Action rapide, `open -a`) :
    /// compression sans fenêtre, puis l'application se ferme.
    Background,
}

struct Launch {
    mode: Mode,
    /// Lots de fichiers en cours de compression sans fenêtre.
    jobs: usize,
}

struct AppState {
    engine: Option<Engine>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    launch: Mutex<Launch>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineInfo {
    available: bool,
    bundled: bool,
    version: Option<String>,
}

#[tauri::command]
fn engine_info(state: State<'_, AppState>) -> EngineInfo {
    match &state.engine {
        Some(e) => EngineInfo { available: true, bundled: e.exe.ends_with("gs/bin/gs"), version: e.version() },
        None => EngineInfo { available: false, bundled: false, version: None },
    }
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    settings.save(&state.settings_path)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn collect_pdfs(paths: Vec<String>) -> Vec<String> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    compress::collect_pdfs(&paths).into_iter().map(|p| p.display().to_string()).collect()
}

#[tauri::command]
async fn compress_file(path: String, state: State<'_, AppState>) -> Result<Outcome, String> {
    let engine = state.engine.clone().ok_or("Ghostscript est introuvable.")?;
    let settings = state.settings.lock().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || compress::compress_in_place(&engine, &PathBuf::from(path), &settings))
        .await
        .map_err(|e| e.to_string())?
}

fn show_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Fichiers ouverts avec l'application. Fenêtre affichée : ils rejoignent sa liste.
/// Sinon ils sont compressés sans rien afficher, avec une notification à la fin.
fn open_files(app: &AppHandle, paths: Vec<PathBuf>) {
    let state = app.state::<AppState>();
    {
        let mut launch = state.launch.lock().unwrap();
        if launch.mode == Mode::Window {
            let paths: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            let _ = app.emit("files-opened", paths);
            return;
        }
        launch.mode = Mode::Background;
        launch.jobs += 1;
    }
    // Retire l'icône du Dock et la barre de menus pendant le traitement.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap().clone();
        let results = background::compress_all(state.engine.as_ref(), &paths, &settings);
        background::notify("PDF Compress", &background::summary(&results));
        let mut launch = state.launch.lock().unwrap();
        launch.jobs -= 1;
        if launch.jobs == 0 && launch.mode == Mode::Background {
            app.exit(0);
        }
    });
}

/// Chemins passés en ligne de commande, par exemple
/// `"PDF Compress.app/Contents/MacOS/pdfcompress" rapport.pdf`.
fn cli_paths() -> Vec<PathBuf> {
    std::env::args().skip(1).filter(|a| !a.starts_with('-')).map(PathBuf::from).collect()
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir().ok();
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState {
                engine: Engine::locate(resource_dir.as_deref()),
                settings: Mutex::new(Settings::load(&settings_path)),
                settings_path,
                launch: Mutex::new(Launch { mode: Mode::Undecided, jobs: 0 }),
            });

            let handle = app.handle().clone();
            let paths = cli_paths();
            if !paths.is_empty() {
                open_files(&handle, paths);
                return Ok(());
            }
            // La fenêtre est masquée au départ : macOS transmet les fichiers déposés sur
            // l'icône juste après le lancement. Sans fichier reçu, on affiche la fenêtre.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(600));
                let state = handle.state::<AppState>();
                let mut launch = state.launch.lock().unwrap();
                if launch.mode == Mode::Undecided {
                    launch.mode = Mode::Window;
                    drop(launch);
                    let h = handle.clone();
                    let _ = handle.run_on_main_thread(move || show_window(&h));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine_info, get_settings, save_settings, collect_pdfs, compress_file])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de PDF Compress")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                let paths: Vec<PathBuf> = urls.iter().filter_map(|u| u.to_file_path().ok()).collect();
                if !paths.is_empty() {
                    open_files(app, paths);
                }
            }
            // Clic sur l'application alors qu'elle compresse en arrière-plan.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => {
                app.state::<AppState>().launch.lock().unwrap().mode = Mode::Window;
                show_window(app);
            }
            // Fenêtre fermée pendant une compression d'arrière-plan : on termine d'abord.
            RunEvent::ExitRequested { api, code: None, .. } => {
                let state = app.state::<AppState>();
                let mut launch = state.launch.lock().unwrap();
                if launch.jobs > 0 {
                    launch.mode = Mode::Background;
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
