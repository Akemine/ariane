//! Mises à jour de l'application.
//!
//! L'appli lit le fichier latest.json publié sur GitHub Releases (adresse dans tauri.conf.json),
//! compare la version, puis télécharge et installe l'installateur. Chaque version est signée :
//! une mise à jour qui ne porte pas la signature de la clé privée de l'auteur est refusée.

use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
pub struct InfoMiseAJour {
    version: String,
    notes: Option<String>,
}

#[tauri::command]
pub fn version_application(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Une version plus récente est-elle publiée ? `None` si l'appli est à jour.
#[tauri::command]
pub async fn verifier_mise_a_jour(app: tauri::AppHandle) -> Result<Option<InfoMiseAJour>, String> {
    let maj = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    Ok(maj.map(|m| InfoMiseAJour { version: m.version.clone(), notes: m.body.clone() }))
}

/// Télécharge, installe puis relance l'application.
#[tauri::command]
pub async fn installer_mise_a_jour(app: tauri::AppHandle) -> Result<(), String> {
    let Some(maj) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
        return Ok(());
    };
    maj.download_and_install(|_, _| {}, || {}).await.map_err(|e| format!("Mise à jour impossible : {e}"))?;
    // Sous Windows, l'installateur ferme lui-même l'application ; ailleurs, on relance
    app.restart()
}
