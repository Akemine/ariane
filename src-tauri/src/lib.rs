//! Ariane — point d'entrée de l'application Tauri.

mod actions;
mod apercu;
mod config;
mod fichiers;
mod gestionnaire;
mod git;
mod jeux;
mod maj;
mod menage;
mod nuages;
mod peripheriques;
mod presse_papiers;
mod recherche;
mod reseau;
mod surveillance;
mod systeme;

pub fn run() {
    // Linux : une Ariane tourne déjà ? Elle ouvre la demande dans sa fenêtre, celle-ci s'arrête là
    if gestionnaire::transmettre_a_l_instance_ouverte() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Aperçu rapide : images, vidéos, sons et PDF servis par morceaux (voir apercu.rs)
        .register_asynchronous_uri_scheme_protocol("apercu", |_contexte, requete, repondeur| {
            std::thread::spawn(move || repondeur.respond(apercu::repondre(&requete)));
        })
        // Linux : répond aux « Afficher dans le dossier » de VS Code, Firefox… (voir gestionnaire.rs)
        .setup(|app| {
            gestionnaire::demarrer(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fichiers::lister_dossier,
            fichiers::decrire_chemins,
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
            peripheriques::lister_peripheriques,
            peripheriques::monter_peripherique,
            peripheriques::ejecter_peripherique,
            reseau::connecter_partage,
            apercu::lire_debut,
            apercu::lire_fichier,
            menage::analyser_menage,
            menage::ressortir_de_la_corbeille,
            nuages::jauges_nuages,
            presse_papiers::ecrire_presse_papiers,
            presse_papiers::lire_presse_papiers,
            presse_papiers::vider_presse_papiers,
            surveillance::surveiller,
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
            gestionnaire::prendre_demandes,
            gestionnaire::etat_integration,
            gestionnaire::activer_integration,
            gestionnaire::desactiver_integration,
            gestionnaire::fermer_nautilus,
            config::lire_config,
            config::ecrire_config,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
