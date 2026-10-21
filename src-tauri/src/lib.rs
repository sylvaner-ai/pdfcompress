mod compress;
mod settings;

use compress::{Engine, Outcome};
use serde::Serialize;
use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

struct AppState {
    engine: Option<Engine>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
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

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir().ok();
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState {
                engine: Engine::locate(resource_dir.as_deref()),
                settings: Mutex::new(Settings::load(&settings_path)),
                settings_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine_info, get_settings, save_settings, collect_pdfs, compress_file])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PDF Compress");
}
