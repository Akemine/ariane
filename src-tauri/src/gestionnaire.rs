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

/// Argument entre apostrophes, pour la ligne Exec (chemins avec espaces).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn citer(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Ligne Exec du service. Elle revérifie tout au moment du lancement, car ce fichier passe
/// avant celui de Nautilus : si Ariane a été désinstallée ou n'est plus le gestionnaire par
/// défaut, le fichier s'efface et Nautilus prend le relais, sans attendre qu'Ariane soit relancée.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn ligne_exec(exe: &str, fichier: &str) -> String {
    let script = r#"if [ -x "$0" ] && xdg-mime query default inode/directory | grep -qi ariane; then exec "$0"; fi; rm -f "$1"; exec nautilus --gapplication-service"#;
    format!("/bin/sh -c {} {} {}", citer(script), citer(exe), citer(fichier))
}

#[cfg(target_os = "linux")]
pub fn demarrer(app: tauri::AppHandle) {
    linux::demarrer(app)
}

#[cfg(not(target_os = "linux"))]
pub fn demarrer(_app: tauri::AppHandle) {}

#[cfg(target_os = "linux")]
mod linux {
    use super::{ligne_exec, Demande, EN_ATTENTE};
    use gtk::gio;
    use gtk::glib::ToVariant;
    use gtk::prelude::GtkWindowExt;
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
        let contenu = format!("[D-BUS Service]\nName={NOM}\nExec={}\n", ligne_exec(&exe, &fichier.to_string_lossy()));
        if fs::read_to_string(&fichier).ok().as_deref() != Some(contenu.as_str()) {
            if let Some(dossier) = fichier.parent() {
                let _ = fs::create_dir_all(dossier);
            }
            let _ = fs::write(&fichier, contenu);
        }
    }

    fn retirer_service() {
        if let Some(fichier) = fichier_service() {
            // « Ariane_0.8.9_amd64.AppImage » compte aussi : on compare sans les majuscules
            let ce_service_est_le_notre =
                fs::read_to_string(&fichier).map(|t| t.to_lowercase().contains("ariane")).unwrap_or(false);
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
        // xdg-mime est un script : on le lance à côté, pour ne pas retarder l'ouverture de la fenêtre
        std::thread::spawn(move || {
            if !est_par_defaut() {
                retirer_service();
                return;
            }
            installer_service();
            // Le service D-Bus répond sur la boucle de GTK, donc depuis le fil principal
            let a = app.clone();
            let _ = app.run_on_main_thread(move || posseder(a));
        });
    }

    /// Passe Ariane au premier plan. Sous Wayland, il faut le jeton d'activation transmis par
    /// l'appli qui demande, sinon GNOME affiche « Ariane est prêt » au lieu d'amener la fenêtre.
    fn premier_plan(app: &tauri::AppHandle, jeton: &str) {
        let Some(fenetre) = app.get_webview_window("main") else { return };
        let _ = fenetre.unminimize();
        let _ = fenetre.show();
        if let Ok(fenetre_gtk) = fenetre.gtk_window() {
            if !jeton.is_empty() {
                donner_jeton(&fenetre_gtk, jeton);
            }
            fenetre_gtk.present();
        }
        // Sans jeton, on demande quand même (X11, ou GNOME qui l'accepte parfois) ; avec un jeton,
        // une seconde demande sans lui pourrait passer après et être refusée
        if jeton.is_empty() {
            let _ = fenetre.set_focus();
        }
    }

    /// Wayland : le jeton d'activation se donne à l'écran juste avant present(), qui l'utilise
    /// (xdg-activation) ; set_startup_id ne le transmet pas. X11 : set_startup_id suffit.
    fn donner_jeton(fenetre: &gtk::ApplicationWindow, jeton: &str) {
        use gtk::glib::prelude::*;
        use gtk::glib::translate::ToGlibPtr;
        use gtk::prelude::WidgetExt;
        let ecran = fenetre.display();
        if ecran.type_().name() == "GdkWaylandDisplay" {
            let Ok(texte) = std::ffi::CString::new(jeton) else { return };
            let brut: *mut gtk::gdk::ffi::GdkDisplay = ecran.to_glib_none().0;
            unsafe { gdk_wayland_sys::gdk_wayland_display_set_startup_notification_id(brut as *mut _, texte.as_ptr()) };
        } else {
            fenetre.set_startup_id(jeton);
        }
    }

    fn posseder(app: tauri::AppHandle) {
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
                        let (uris, jeton): (Vec<String>, String) = parametres.get().unwrap_or_default();
                        let mode = if methode == "ShowFolders" { "dossiers" } else { "elements" };
                        let chemins: Vec<String> = uris.iter().filter_map(|u| uri_vers_chemin(u)).collect();
                        if !chemins.is_empty() {
                            EN_ATTENTE.lock().unwrap().push(Demande { mode: mode.into(), chemins });
                            let _ = app.emit("demande-systeme", ());
                            premier_plan(&app, &jeton);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_avec_espaces_et_apostrophes() {
        let l = ligne_exec("/home/s/Mes applis/l'Ariane.AppImage", "/home/s/.local/share/x.service");
        assert!(l.starts_with("/bin/sh -c 'if [ -x \"$0\" ]"), "{l}");
        assert!(l.ends_with(r" '/home/s/Mes applis/l'\''Ariane.AppImage' '/home/s/.local/share/x.service'"), "{l}");
        println!("{l}");
    }
}
