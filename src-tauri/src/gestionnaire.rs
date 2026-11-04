//! Linux : Ariane comme gestionnaire de fichiers du système.
//!
//! - « Afficher dans le dossier » de VS Code, Firefox, Chrome… passe par le service
//!   « org.freedesktop.FileManager1 » : Ariane le prend quand elle est le gestionnaire par défaut
//!   (sinon ces applis ouvrent toujours Nautilus).
//! - Fenêtre unique : une Ariane lancée alors qu'une autre tourne déjà (dossier ouvert depuis une
//!   autre application) lui passe sa demande et s'arrête ; un onglet s'ouvre dans la fenêtre
//!   existante, qui passe au premier plan.
//! - Le réglage « Ariane gestionnaire par défaut » (xdg-mime) se fait et se défait depuis Ariane ;
//!   tout reste dans le dossier de l'utilisateur, rien n'est demandé à l'administrateur.

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

/// Où en est l'intégration au système (affiché dans les réglages).
#[derive(Serialize, Default, Debug)]
pub struct Integration {
    /// false hors Linux : rien à régler
    disponible: bool,
    /// Ariane ouvre les dossiers des autres applications (xdg-mime, inode/directory)
    par_defaut: bool,
    /// Le programme qui ouvre les dossiers aujourd'hui (« org.gnome.Nautilus.desktop »…)
    defaut: Option<String>,
    /// Le programme qui répond à « Afficher dans le dossier » (« ariane », « nautilus »…)
    afficher_dans_dossier: Option<String>,
}

#[tauri::command]
pub async fn etat_integration() -> Integration {
    tauri::async_runtime::spawn_blocking(plateforme_etat).await.unwrap_or_default()
}

#[tauri::command]
pub async fn activer_integration(app: tauri::AppHandle) -> Result<Integration, String> {
    tauri::async_runtime::spawn_blocking(move || plateforme_activer(app)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn desactiver_integration(app: tauri::AppHandle) -> Result<Integration, String> {
    tauri::async_runtime::spawn_blocking(move || plateforme_desactiver(app)).await.map_err(|e| e.to_string())?
}

/// Nautilus garde « Afficher dans le dossier » tant qu'il tourne en arrière-plan : on le ferme
/// (ses fenêtres ouvertes se ferment aussi, l'interface prévient avant).
#[tauri::command]
pub async fn fermer_nautilus() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "linux")]
        {
            let s = std::process::Command::new("nautilus").arg("-q").output().map_err(|e| format!("Nautilus introuvable : {e}"))?;
            if !s.status.success() && !String::from_utf8_lossy(&s.stderr).trim().is_empty() {
                return Err(format!("Nautilus n'a pas voulu se fermer : {}", String::from_utf8_lossy(&s.stderr).trim()));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
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

/// Chemin pour la ligne Exec d'un fichier .desktop : entre guillemets, sans les caractères que
/// cette ligne interprète (None : chemin trop inhabituel pour être écrit sans risque).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn exec_desktop(exe: &str) -> Option<String> {
    (!exe.contains(['"', '`', '$', '\\', '%', '\n'])).then(|| format!("\"{exe}\" %U"))
}

/// mimeapps.list sans la ligne « inode/directory=…ariane… » (quand aucun autre gestionnaire n'est
/// connu pour la remplacer, on la retire : le système reprend alors son choix habituel).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn sans_ariane_pour_les_dossiers(mimeapps: &str) -> String {
    let mut sortie: Vec<&str> = mimeapps
        .lines()
        .filter(|l| !(l.trim_start().starts_with("inode/directory=") && l.to_lowercase().contains("ariane")))
        .collect();
    if mimeapps.ends_with('\n') {
        sortie.push("");
    }
    sortie.join("\n")
}

/// Les chemins donnés à Ariane au lancement (« ariane ~/Projets », « ariane file:///… »), en absolu.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn chemins_du_lancement(arguments: impl Iterator<Item = String>, dossier_courant: &std::path::Path) -> Vec<String> {
    arguments
        .filter(|a| !a.starts_with('-'))
        .map(|a| match a.strip_prefix("file://") {
            Some(reste) => crate::systeme::decoder_uri(reste),
            None => a,
        })
        .map(|a| {
            let p = std::path::Path::new(&a);
            // smb://… reste une adresse (l'interface montera le partage)
            if p.is_absolute() || a.contains("://") { a } else { dossier_courant.join(p).to_string_lossy().into_owned() }
        })
        .collect()
}

#[cfg(target_os = "linux")]
pub fn demarrer(app: tauri::AppHandle) {
    linux::demarrer(app)
}

#[cfg(not(target_os = "linux"))]
pub fn demarrer(_app: tauri::AppHandle) {}

/// Tout au début, avant d'ouvrir une fenêtre : true si une Ariane tournait déjà et a reçu la
/// demande (celle-ci n'a plus qu'à s'arrêter).
#[cfg(target_os = "linux")]
pub fn transmettre_a_l_instance_ouverte() -> bool {
    linux::transmettre()
}

#[cfg(not(target_os = "linux"))]
pub fn transmettre_a_l_instance_ouverte() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn plateforme_etat() -> Integration {
    linux::etat()
}
#[cfg(target_os = "linux")]
fn plateforme_activer(app: tauri::AppHandle) -> Result<Integration, String> {
    linux::activer(app)
}
#[cfg(target_os = "linux")]
fn plateforme_desactiver(app: tauri::AppHandle) -> Result<Integration, String> {
    linux::desactiver(app)
}

#[cfg(not(target_os = "linux"))]
fn plateforme_etat() -> Integration {
    Integration::default()
}
#[cfg(not(target_os = "linux"))]
fn plateforme_activer(_app: tauri::AppHandle) -> Result<Integration, String> {
    Err("Ce réglage n'existe que sous Linux.".into())
}
#[cfg(not(target_os = "linux"))]
fn plateforme_desactiver(_app: tauri::AppHandle) -> Result<Integration, String> {
    Err("Ce réglage n'existe que sous Linux.".into())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{chemins_du_lancement, exec_desktop, ligne_exec, sans_ariane_pour_les_dossiers, Demande, Integration, EN_ATTENTE};
    use gtk::gio;
    use gtk::glib::ToVariant;
    use gtk::prelude::GtkWindowExt;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Mutex;
    use tauri::{Emitter, Manager};

    const NOM: &str = "org.freedesktop.FileManager1";
    const INTERFACE: &str = r#"<node><interface name="org.freedesktop.FileManager1">
      <method name="ShowFolders"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
      <method name="ShowItems"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
      <method name="ShowItemProperties"><arg type="as" name="URIs" direction="in"/><arg type="s" name="StartupId" direction="in"/></method>
    </interface></node>"#;

    // Fenêtre unique : le nom que prend la première Ariane, et ce qu'on lui envoie
    const INSTANCE: &str = "fr.ariane.desktop";
    const INSTANCE_OBJET: &str = "/fr/ariane/desktop";
    const INSTANCE_XML: &str = r#"<node><interface name="fr.ariane.desktop">
      <method name="Ouvrir"><arg type="as" name="Chemins" direction="in"/><arg type="s" name="Jeton" direction="in"/></method>
    </interface></node>"#;

    /// Le service « Afficher dans le dossier », tant qu'on le tient (pour le rendre).
    static SERVICE: Mutex<Option<gio::OwnerId>> = Mutex::new(None);

    /// npm run dev : ni fenêtre unique ni service, pour ne pas gêner l'Ariane installée.
    fn developpement() -> bool {
        std::env::current_exe().map(|e| e.to_string_lossy().contains("/target/")).unwrap_or(false)
    }

    fn defaut_actuel() -> Option<String> {
        let s = Command::new("xdg-mime").args(["query", "default", "inode/directory"]).output().ok()?;
        let t = String::from_utf8_lossy(&s.stdout).trim().to_string();
        (!t.is_empty()).then_some(t)
    }

    /// Ariane est-elle le gestionnaire de fichiers par défaut ?
    fn est_par_defaut() -> bool {
        defaut_actuel().is_some_and(|d| d.to_lowercase().contains("ariane"))
    }

    fn fichier_service() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("dbus-1/services").join(format!("{NOM}.service")))
    }

    /// Le programme lui-même (pour une AppImage : le fichier .AppImage, pas son montage temporaire).
    fn executable() -> Option<String> {
        let exe = std::env::var("APPIMAGE").map(PathBuf::from).or_else(|_| std::env::current_exe()).ok()?;
        Some(exe.to_string_lossy().into_owned())
    }

    /// Fichier qui permet au système de lancer Ariane quand une appli demande le service
    /// et qu'Ariane n'est pas ouverte. Placé dans le dossier de l'utilisateur, il passe avant Nautilus.
    fn installer_service() {
        let Some(exe) = executable() else { return };
        if developpement() {
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
        if !developpement() {
            instance_unique(app.clone());
        }
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

    /// Une demande pour l'interface (elle ouvre les onglets quand elle la prend).
    fn ajouter(app: &tauri::AppHandle, mode: &str, chemins: Vec<String>) {
        if !chemins.is_empty() {
            EN_ATTENTE.lock().unwrap().push(Demande { mode: mode.into(), chemins });
            let _ = app.emit("demande-systeme", ());
        }
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

    /// Prend « Afficher dans le dossier » (une seule fois, même si on le redemande).
    fn posseder(app: tauri::AppHandle) {
        let mut service = SERVICE.lock().unwrap();
        if service.is_some() {
            return;
        }
        *service = Some(gio::bus_own_name(
            gio::BusType::Session,
            NOM,
            // Prend la place de Nautilus s'il l'accepte ; mais Nautilus, lui, ne peut pas nous la reprendre
            // tant qu'Ariane tourne (il démarre souvent en coulisse, par exemple pour la fenêtre « Ouvrir »)
            gio::BusNameOwnerFlags::REPLACE,
            move |connexion, _| {
                let Ok(noeud) = gio::DBusNodeInfo::for_xml(INTERFACE) else { return };
                let Some(interface) = noeud.lookup_interface(NOM) else { return };
                let app = app.clone();
                // Déjà enregistré si on a rendu puis repris le service : l'ancien enregistrement sert toujours
                let _ = connexion.register_object(
                    "/org/freedesktop/FileManager1",
                    &interface,
                    move |_, _, _, _, methode, parametres, invocation| {
                        let (uris, jeton): (Vec<String>, String) = parametres.get().unwrap_or_default();
                        let mode = if methode == "ShowFolders" { "dossiers" } else { "elements" };
                        let chemins: Vec<String> = uris.iter().filter_map(|u| uri_vers_chemin(u)).collect();
                        if !chemins.is_empty() {
                            ajouter(&app, mode, chemins);
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
        ));
    }

    /// Rend « Afficher dans le dossier » : Nautilus, s'il attendait, le reprend aussitôt.
    fn liberer() {
        if let Some(id) = SERVICE.lock().unwrap().take() {
            gio::bus_unown_name(id);
        }
    }

    // ———————————————————————————— Fenêtre unique ————————————————————————————

    /// La première Ariane prend le nom « fr.ariane.desktop » et reçoit les demandes des suivantes.
    fn instance_unique(app: tauri::AppHandle) {
        let id = gio::bus_own_name(
            gio::BusType::Session,
            INSTANCE,
            gio::BusNameOwnerFlags::NONE,
            move |connexion, _| {
                let Ok(noeud) = gio::DBusNodeInfo::for_xml(INSTANCE_XML) else { return };
                let Some(interface) = noeud.lookup_interface(INSTANCE) else { return };
                let app = app.clone();
                let _ = connexion.register_object(
                    INSTANCE_OBJET,
                    &interface,
                    move |_, _, _, _, _, parametres, invocation| {
                        let (chemins, jeton): (Vec<String>, String) = parametres.get().unwrap_or_default();
                        // Un dossier s'ouvre dans un onglet ; un fichier, dans son dossier, sélectionné
                        // (une adresse smb://… compte comme un dossier : l'interface la montera)
                        let (dossiers, fichiers): (Vec<String>, Vec<String>) = chemins.into_iter().partition(|c| c.contains("://") || Path::new(c).is_dir());
                        let fichiers: Vec<String> = fichiers.into_iter().filter(|c| Path::new(c).exists()).collect();
                        ajouter(&app, "dossiers", dossiers);
                        ajouter(&app, "elements", fichiers);
                        // Même relancée sans rien à ouvrir (raccourci…), la fenêtre passe devant
                        premier_plan(&app, &jeton);
                        invocation.return_value(None);
                    },
                    |_, _, _, _, _| ().to_variant(),
                    |_, _, _, _, _, _| false,
                );
            },
            |_, _| {},
            |_, _| {},
        );
        // Gardé jusqu'à la fermeture d'Ariane
        std::mem::forget(id);
    }

    /// Une Ariane tourne déjà ? Elle reçoit nos chemins et notre jeton d'activation ; true si c'est fait.
    pub fn transmettre() -> bool {
        if developpement() {
            return false;
        }
        let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else { return false };
        // Personne : c'est à nous. Notre propre parent (Ariane qui se relance après une mise à jour,
        // et qui s'arrête dans un instant) : on démarre aussi, sinon il n'y aurait plus d'Ariane du tout
        match pid_du_nom(&bus, INSTANCE) {
            None => return false,
            Some(pid) if pid == std::os::unix::process::parent_id() => return false,
            Some(_) => {}
        }
        let dossier = std::env::current_dir().unwrap_or_default();
        let chemins = chemins_du_lancement(std::env::args().skip(1), &dossier);
        // Donné par l'application qui nous lance (GNOME, xdg-open…), pour passer au premier plan
        let jeton = std::env::var("XDG_ACTIVATION_TOKEN").or_else(|_| std::env::var("DESKTOP_STARTUP_ID")).unwrap_or_default();
        bus.call_sync(
            Some(INSTANCE),
            INSTANCE_OBJET,
            INSTANCE,
            "Ouvrir",
            Some(&(chemins, jeton).to_variant()),
            None,
            // Personne n'a le nom : on ne lance rien, c'est à nous de démarrer
            gio::DBusCallFlags::NO_AUTO_START,
            3000,
            gio::Cancellable::NONE,
        )
        .is_ok()
    }

    // ———————————————————————————— Réglage « gestionnaire par défaut » ————————————————————————————

    /// Dossiers où le système cherche les applications (~/.local/share, puis /usr/local/share, /usr/share…).
    fn dossiers_applications() -> Vec<PathBuf> {
        let mut liste: Vec<PathBuf> = dirs::data_dir().into_iter().collect();
        let systeme = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        liste.extend(systeme.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
        liste.into_iter().map(|d| d.join("applications")).collect()
    }

    fn application_existe(id: &str) -> bool {
        dossiers_applications().iter().any(|d| d.join(id).is_file())
    }

    /// Le fichier .desktop d'Ariane (« Ariane.desktop » du paquet rpm / deb). Une AppImage n'en a pas :
    /// on en crée un dans ~/.local/share/applications, qui pointe vers le fichier .AppImage.
    fn application_ariane() -> Result<String, String> {
        // Toutes les Ariane installées (.rpm et AppImage peuvent coexister) : on préfère celle qui
        // lance le programme en cours, sinon la première trouvée
        let mut trouvees: Vec<(String, String)> = Vec::new();
        for d in dossiers_applications() {
            let Ok(lecture) = fs::read_dir(&d) else { continue };
            let mut noms: Vec<String> = lecture.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            noms.sort();
            for n in noms.into_iter().filter(|n| n.to_lowercase().starts_with("ariane") && n.ends_with(".desktop")) {
                let contenu = fs::read_to_string(d.join(&n)).unwrap_or_default();
                if !trouvees.iter().any(|(id, _)| *id == n) {
                    trouvees.push((n, contenu));
                }
            }
        }
        let exe = executable().unwrap_or_default();
        let nom_exe = Path::new(&exe).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let lance_ce_programme = |contenu: &str| {
            contenu.lines().filter_map(|l| l.strip_prefix("Exec=")).any(|e| (!exe.is_empty() && e.contains(&exe)) || e.split_whitespace().next().is_some_and(|c| c.trim_matches('"') == nom_exe))
        };
        if let Some((id, _)) = trouvees.iter().find(|(_, c)| lance_ce_programme(c)) {
            return Ok(id.clone());
        }
        // Une AppImage lancée alors qu'un .rpm est aussi installé : c'est elle qu'on veut, on lui crée le sien
        if std::env::var_os("APPIMAGE").is_none() {
            if let Some((id, _)) = trouvees.first() {
                return Ok(id.clone());
            }
        }
        let exe = std::env::var("APPIMAGE").map_err(|_| "Ariane n'est pas installée comme application (fichier .desktop introuvable).".to_string())?;
        let exec = exec_desktop(&exe).ok_or("Le chemin de l'AppImage contient des caractères inhabituels : déplacez-la dans un dossier au nom simple.")?;
        let dossier = dirs::data_dir().ok_or("Dossier ~/.local/share introuvable.")?.join("applications");
        fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
        let contenu = format!("[Desktop Entry]\nType=Application\nName=Ariane\nComment=Explorateur de fichiers\nExec={exec}\nTerminal=false\nMimeType=inode/directory;\nCategories=Utility;FileManager;\nStartupWMClass=ariane\n");
        fs::write(dossier.join("ariane-appimage.desktop"), contenu).map_err(|e| e.to_string())?;
        Ok("ariane-appimage.desktop".into())
    }

    /// Le gestionnaire d'avant, pour le remettre en rendant la main.
    fn fichier_precedent() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("fr.ariane.desktop").join("gestionnaire-precedent"))
    }

    /// Le processus qui tient un nom sur le bus de la session, ou None si personne.
    fn pid_du_nom(bus: &gio::DBusConnection, nom: &str) -> Option<u32> {
        let appeler = |methode: &str, arg: &str| {
            bus.call_sync(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                methode,
                Some(&(arg,).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                2000,
                gio::Cancellable::NONE,
            )
            .ok()
        };
        let (unique,): (String,) = appeler("GetNameOwner", nom)?.get()?;
        let (pid,): (u32,) = appeler("GetConnectionUnixProcessID", &unique)?.get()?;
        Some(pid)
    }

    /// Qui tient « Afficher dans le dossier » : le nom du programme (« ariane », « nautilus »…).
    fn proprietaire_du_service() -> Option<String> {
        let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
        let pid = pid_du_nom(&bus, NOM)?;
        if pid == std::process::id() {
            return Some("ariane".into());
        }
        fs::read_to_string(format!("/proc/{pid}/comm")).ok().map(|n| n.trim().to_string())
    }

    pub fn etat() -> Integration {
        let defaut = defaut_actuel();
        Integration {
            disponible: true,
            par_defaut: defaut.as_deref().is_some_and(|d| d.to_lowercase().contains("ariane")),
            defaut,
            afficher_dans_dossier: proprietaire_du_service(),
        }
    }

    pub fn activer(app: tauri::AppHandle) -> Result<Integration, String> {
        let id = application_ariane()?;
        // On retient celui d'avant (sauf si c'était déjà Ariane) pour pouvoir le remettre
        if let (Some(avant), Some(fichier)) = (defaut_actuel(), fichier_precedent()) {
            if !avant.to_lowercase().contains("ariane") {
                if let Some(d) = fichier.parent() {
                    let _ = fs::create_dir_all(d);
                }
                let _ = fs::write(&fichier, &avant);
            }
        }
        let s = Command::new("xdg-mime").args(["default", &id, "inode/directory"]).output().map_err(|e| format!("xdg-mime introuvable : {e}"))?;
        if !s.status.success() || !est_par_defaut() {
            return Err(format!("Le système n'a pas accepté Ariane comme gestionnaire de fichiers. {}", String::from_utf8_lossy(&s.stderr).trim()));
        }
        installer_service();
        let a = app.clone();
        let _ = app.run_on_main_thread(move || posseder(a));
        Ok(etat())
    }

    pub fn desactiver(app: tauri::AppHandle) -> Result<Integration, String> {
        let precedent = fichier_precedent().and_then(|f| fs::read_to_string(f).ok()).map(|t| t.trim().to_string());
        // Celui d'avant s'il existe encore, sinon Nautilus, sinon on retire simplement notre ligne
        let remplacant = precedent
            .filter(|p| !p.is_empty() && application_existe(p))
            .or_else(|| application_existe("org.gnome.Nautilus.desktop").then(|| "org.gnome.Nautilus.desktop".to_string()));
        match remplacant {
            Some(r) => {
                let s = Command::new("xdg-mime").args(["default", &r, "inode/directory"]).output().map_err(|e| format!("xdg-mime introuvable : {e}"))?;
                if !s.status.success() {
                    return Err(format!("Le système n'a pas accepté {r}. {}", String::from_utf8_lossy(&s.stderr).trim()));
                }
            }
            None => {
                if let Some(f) = dirs::config_dir().map(|d| d.join("mimeapps.list")) {
                    if let Ok(t) = fs::read_to_string(&f) {
                        let _ = fs::write(&f, sans_ariane_pour_les_dossiers(&t));
                    }
                }
            }
        }
        retirer_service();
        let _ = app.run_on_main_thread(liberer);
        Ok(etat())
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

    #[test]
    fn exec_d_une_appimage() {
        assert_eq!(exec_desktop("/home/s/Mes applis/Ariane.AppImage").as_deref(), Some("\"/home/s/Mes applis/Ariane.AppImage\" %U"));
        assert_eq!(exec_desktop("/home/s/$HOME/a"), None);
        assert_eq!(exec_desktop("/home/s/100%/a"), None);
    }

    #[test]
    fn retirer_ariane_de_mimeapps() {
        let avant = "[Default Applications]\ntext/plain=org.gnome.TextEditor.desktop\ninode/directory=Ariane.desktop\n\n[Added Associations]\ninode/directory=Ariane.desktop;org.gnome.Nautilus.desktop;\n";
        let apres = sans_ariane_pour_les_dossiers(avant);
        assert_eq!(apres, "[Default Applications]\ntext/plain=org.gnome.TextEditor.desktop\n\n[Added Associations]\n");
        // Un autre gestionnaire choisi n'est pas touché
        let autre = "[Default Applications]\ninode/directory=org.gnome.Nautilus.desktop\n";
        assert_eq!(sans_ariane_pour_les_dossiers(autre), autre);
    }

    // Chemins Linux (« /tmp » n'est pas absolu sous Windows)
    #[cfg(unix)]
    #[test]
    fn chemins_au_lancement() {
        let ici = std::path::Path::new("/home/moi");
        let args = ["--flag", "Projets", "/tmp/a b", "file:///home/moi/T%C3%A9l%C3%A9chargements", "smb://serveur/data/"].map(String::from);
        let c = chemins_du_lancement(args.into_iter(), ici);
        assert_eq!(c[0], std::path::Path::new("/home/moi").join("Projets").to_string_lossy());
        assert_eq!(c[2], "/home/moi/Téléchargements");
        assert_eq!(c[3], "smb://serveur/data/");
        assert_eq!(c.len(), 4);
    }
}
