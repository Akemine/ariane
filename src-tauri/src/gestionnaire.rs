//! Linux : Ariane répond au service « org.freedesktop.FileManager1 », celui qu'utilisent
//! VS Code, Firefox, Chrome… pour « Afficher dans le dossier ». Sans lui, ces applis
//! ouvrent toujours Nautilus, même quand Ariane est le gestionnaire de fichiers par défaut.
//!
//! Le service n'est pris que si Ariane est le gestionnaire par défaut
//! (xdg-mime default Ariane.desktop inode/directory) : rien ne change pour les autres.

use serde::Serialize;
use std::sync::Mutex;

/// Une demande reçue : ouvrir des dossiers, ou montrer des éléments dans leur dossier.
#[derive(Serialize, Clone)]
pub struct Demande {
    mode: String,
    chemins: Vec<String>,
}

/// Demandes reçues avant que l'interface soit prête (Ariane lancée par la demande elle-même).
static EN_ATTENTE: Mutex<Vec<Demande>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn prendre_demandes() -> Vec<Demande> {
    std::mem::take(&mut *EN_ATTENTE.lock().unwrap())
}

#[cfg(target_os = "linux")]
pub fn demarrer(app: tauri::AppHandle) {
    linux::demarrer(app)
}

#[cfg(not(target_os = "linux"))]
pub fn demarrer(_app: tauri::AppHandle) {}

#[cfg(target_os = "linux")]
mod linux {
    use super::{Demande, EN_ATTENTE};
    use gtk::gio;
    use gtk::glib::ToVariant;
    use std::fs;
    use std::path::PathBuf;
    use tauri::{Emitter, Manager};

    const NOM: &str = "org.freedesktop.FileManager1";
    const INTERFACE: &str = r#"<node><interface name="org.freedesktop.FileManager1">
      <method name="ShowFolders"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
      <method name="ShowItems"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
      <method name="ShowItemProperties"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
    </interface></node>"#;

    /// Ariane est-elle le gestionnaire de fichiers par défaut ?
    fn est_par_defaut() -> bool {
        std::process::Command::new("xdg-mime")
            .args(["query", "default", "inode/directory"])
            .output()
            .map(|s| String::from_utf8_lossy(&s.stdout).to_lowercase().contains("ariane"))
            .unwrap_or(false)
    }

    fn fichier_service() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("dbus-1/services").join(format!("{NOM}.service")))
    }

    /// Fichier qui permet au système de lancer Ariane quand une appli demande le service
    /// et qu'Ariane n'est pas ouverte. Placé dans le dossier de l'utilisateur, il passe avant Nautilus.
    fn installer_service() {
        // AppImage : chemin du fichier .AppImage, pas celui du montage temporaire
        let exe = std::env::var("APPIMAGE").map(PathBuf::from).or_else(|_| std::env::current_exe());
        let Ok(exe) = exe else { return };
        let exe = exe.to_string_lossy().into_owned();
        if exe.contains("/target/") {
            return; // version de développement : on n'enregistre pas un exécutable temporaire
        }
        let Some(fichier) = fichier_service() else { return };
        let contenu = format!("[D-BUS Service]\nName={NOM}\nExec={exe}\n");
        if fs::read_to_string(&fichier).ok().as_deref() != Some(contenu.as_str()) {
            if let Some(dossier) = fichier.parent() {
                let _ = fs::create_dir_all(dossier);
            }
            let _ = fs::write(&fichier, contenu);
        }
    }

    fn retirer_service() {
        if let Some(fichier) = fichier_service() {
            let ce_service_est_le_notre = fs::read_to_string(&fichier).map(|t| t.contains("ariane")).unwrap_or(false);
            if ce_service_est_le_notre {
                let _ = fs::remove_file(fichier);
            }
        }
    }

    fn uri_vers_chemin(uri: &str) -> Option<String> {
        let reste = uri.strip_prefix("file://")?;
        // « file://hote/chemin » : on ne garde que le chemin
        let reste = if reste.starts_with('/') { reste } else { &reste[reste.find('/')?..] };
        Some(crate::systeme::decoder_uri(reste))
    }

    pub fn demarrer(app: tauri::AppHandle) {
        if !est_par_defaut() {
            retirer_service();
            return;
        }
        installer_service();
        gio::bus_own_name(
            gio::BusType::Session,
            NOM,
            // Prend la place de Nautilus s'il tient déjà le service et l'accepte
            gio::BusNameOwnerFlags::ALLOW_REPLACEMENT | gio::BusNameOwnerFlags::REPLACE,
            move |connexion, _| {
                let Ok(noeud) = gio::DBusNodeInfo::for_xml(INTERFACE) else { return };
                let Some(interface) = noeud.lookup_interface(NOM) else { return };
                let app = app.clone();
                let _ = connexion.register_object(
                    "/org/freedesktop/FileManager1",
                    &interface,
                    move |_, _, _, _, methode, parametres, invocation| {
                        let (uris, _): (Vec<String>, String) = parametres.get().unwrap_or_default();
                        let mode = if methode == "ShowFolders" { "dossiers" } else { "elements" };
                        let chemins: Vec<String> = uris.iter().filter_map(|u| uri_vers_chemin(u)).collect();
                        if !chemins.is_empty() {
                            EN_ATTENTE.lock().unwrap().push(Demande { mode: mode.into(), chemins });
                            let _ = app.emit("demande-systeme", ());
                            // Ariane passe au premier plan
                            if let Some(fenetre) = app.get_webview_window("main") {
                                let _ = fenetre.unminimize();
                                let _ = fenetre.show();
                                let _ = fenetre.set_focus();
                            }
                        }
                        invocation.return_value(None);
                    },
                    |_, _, _, _, _| ().to_variant(),
                    |_, _, _, _, _, _| false,
                );
            },
            |_, _| {},
            |_, _| {},
        );
    }
}
