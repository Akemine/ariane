//! Réglages de l'utilisateur (espaces, tags, couleurs de dossiers…), stockés en JSON.
//!
//! Linux : ~/.config/fr.ariane.desktop/config.json
//! Windows : %APPDATA%\fr.ariane.desktop\config.json
//!
//! L'application s'appelait « Explorateur » : au premier lancement sous le nom Ariane,
//! les réglages de l'ancien dossier sont recopiés pour ne rien perdre.

use std::fs;
use std::io;
use std::path::PathBuf;
use tauri::Manager;

/// Identifiant de l'application avant son changement de nom.
const ANCIEN_IDENTIFIANT: &str = "fr.explorateur.desktop";

fn fichier(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dossier = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
    let config = dossier.join("config.json");
    if !config.exists() {
        if let Some(ancien) = dossier.parent().map(|p| p.join(ANCIEN_IDENTIFIANT).join("config.json")) {
            if ancien.is_file() {
                let _ = fs::copy(&ancien, &config);
            }
        }
    }
    Ok(config)
}

#[tauri::command]
pub fn lire_config(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let f = fichier(&app)?;
    match fs::read_to_string(&f) {
        Ok(contenu) => Ok(Some(contenu)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Écriture atomique : fichier temporaire puis renommage, pour ne jamais perdre les réglages.
#[tauri::command]
pub fn ecrire_config(app: tauri::AppHandle, contenu: String) -> Result<(), String> {
    let f = fichier(&app)?;
    let temporaire = f.with_extension("json.tmp");
    fs::write(&temporaire, contenu).map_err(|e| e.to_string())?;
    fs::rename(&temporaire, &f).map_err(|e| e.to_string())
}
