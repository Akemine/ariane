//! Ariane — point d'entrée de l'application Tauri.

mod actions;
mod config;
mod fichiers;
mod git;
mod jeux;
mod maj;
mod recherche;
mod systeme;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            fichiers::lister_dossier,
            fichiers::dossiers_standards,
            fichiers::creer_dossier,
            fichiers::creer_fichier,
            fichiers::renommer,
            fichiers::mettre_a_la_corbeille,
            fichiers::conflits_collage,
            fichiers::coller,
            fichiers::annuler_collage,
            git::etat_git,
            recherche::chercher,
            jeux::detecter_jeux,
            jeux::icones_applications,
            maj::version_application,
            actions::extraire_archive,
            actions::compresser,
            actions::taille_dossier,
            actions::lire_texte,
            actions::adresse_depot,
            actions::executer_admin,
            maj::verifier_mise_a_jour,
            maj::installer_mise_a_jour,
            systeme::ouvrir,
            systeme::ouvrir_terminal,
            systeme::ouvrir_vscode,
            systeme::chemin_initial,
            config::lire_config,
            config::ecrire_config,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
