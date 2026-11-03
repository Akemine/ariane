//! Dossiers réseau (partages Windows, SMB).
//!
//! Windows : on lit \\serveur\partage\… directement ; quand le serveur demande des identifiants,
//! Windows affiche sa propre fenêtre de connexion.
//! Linux : comme Nautilus, le partage est monté par GNOME (gvfs), avec la fenêtre de connexion
//! de GNOME (et « Mémoriser »), puis lu par son dossier local /run/user/…/gvfs/smb-share:….
//! Dans les deux cas, le mot de passe ne passe jamais par Ariane.

use std::io;
use std::path::{Component, Path, Prefix};

/// Début des messages qui invitent l'interface à proposer la connexion au partage.
pub(crate) const CONNEXION_REQUISE: &str = "Connexion requise";

/// « \\serveur\partage » pour un chemin réseau Windows, sinon None.
pub(crate) fn partage(chemin: &Path) -> Option<String> {
    match chemin.components().next()? {
        Component::Prefix(p) => match p.kind() {
            Prefix::UNC(serveur, nom) | Prefix::VerbatimUNC(serveur, nom) => {
                Some(format!(r"\\{}\{}", serveur.to_string_lossy(), nom.to_string_lossy()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Message propre aux dossiers réseau, ou None pour les erreurs ordinaires.
pub(crate) fn message(e: &io::Error, chemin: &Path) -> Option<String> {
    let racine = partage(chemin)?;
    let serveur = racine.trim_start_matches('\\').split('\\').next().unwrap_or_default().to_string();
    Some(match e.raw_os_error()? {
        // ERROR_BAD_NETPATH, délai dépassé, réseau ou hôte injoignable
        53 | 121 | 1231 | 1232 => format!(
            "Le serveur {serveur} ne répond pas : vérifiez que vous êtes sur le réseau de ce serveur (ou connecté au VPN)."
        ),
        // ERROR_BAD_NET_NAME
        67 => format!("Le partage « {racine} » n'existe pas sur le serveur {serveur}."),
        // Mot de passe ou compte refusé, ou aucun identifiant pour ce serveur
        86 | 1244 | 1326 | 1327 | 1331 | 1909 => {
            format!("{CONNEXION_REQUISE} : le serveur {serveur} demande un identifiant et un mot de passe.")
        }
        1330 => format!("Le mot de passe de votre compte a expiré pour le serveur {serveur}."),
        // ERROR_SESSION_CREDENTIAL_CONFLICT
        1219 => format!(
            "Vous êtes déjà connecté au serveur {serveur} avec un autre compte : Windows n'en accepte qu'un à la fois par serveur."
        ),
        // ERROR_NETNAME_DELETED, ERROR_UNEXP_NET_ERR
        59 | 64 => format!("La connexion au serveur {serveur} a été coupée. Réessayez."),
        _ => return None,
    })
}

/// Encode un morceau de chemin pour une adresse smb:// (espaces, accents…).
#[cfg_attr(windows, allow(dead_code))]
fn encoder(morceau: &str, garder_pourcent: bool) -> String {
    let mut s = String::new();
    for o in morceau.bytes() {
        let libre = o.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&o) || (garder_pourcent && o == b'%');
        if libre {
            s.push(o as char);
        } else {
            s.push_str(&format!("%{o:02X}"));
        }
    }
    s
}

fn decoder(texte: &str) -> String {
    let o = texte.as_bytes();
    let mut sortie = Vec::with_capacity(o.len());
    let mut i = 0;
    while i < o.len() {
        let hex = (o[i] == b'%' && i + 2 < o.len())
            .then(|| std::str::from_utf8(&o[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()))
            .flatten();
        match hex {
            Some(v) => {
                sortie.push(v);
                i += 3;
            }
            None => {
                sortie.push(o[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&sortie).into_owned()
}

/// Les réglages d'un dossier de montage gvfs « smb-share:server=…,share=… ».
fn reglages_gvfs(nom: &str) -> Option<Vec<(&str, &str)>> {
    let liste: Vec<_> = nom.strip_prefix("smb-share:")?.split(',').filter_map(|r| r.split_once('=')).collect();
    let a = |cle| liste.iter().any(|(c, _)| *c == cle);
    (a("server") && a("share")).then_some(liste)
}

/// « data sur 192.0.0.119 » pour le dossier de montage gvfs d'un partage, dans le fil d'Ariane.
pub(crate) fn nom_partage_gvfs(nom: &str) -> Option<String> {
    let r = reglages_gvfs(nom)?;
    let val = |cle| r.iter().find(|(c, _)| *c == cle).map(|(_, v)| decoder(v)).unwrap_or_default();
    Some(format!("{} sur {}", val("share"), val("server")))
}

/// Adresse smb:// d'un dossier réseau tapé à la façon Windows (\\serveur\partage\…), déjà en smb://,
/// ou monté par gvfs (/run/user/…/gvfs/smb-share:server=…,share=…/…, utile après un redémarrage).
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn adresse_smb(chemin: &str) -> Option<String> {
    let chemin = chemin.trim();
    let (racine, reste, garder) = if let Some(r) = chemin.strip_prefix(r"\\").or_else(|| chemin.strip_prefix("//")) {
        let mut m = r.split(['\\', '/']).filter(|x| !x.is_empty());
        let (serveur, nom) = (m.next()?, m.next()?);
        (format!("{serveur}/{}", encoder(nom, false)), m.collect::<Vec<_>>(), false)
    } else if chemin.len() > 6 && chemin[..6].eq_ignore_ascii_case("smb://") {
        let mut m = chemin[6..].split('/').filter(|x| !x.is_empty());
        let (serveur, nom) = (m.next()?, m.next()?);
        (format!("{serveur}/{}", encoder(nom, true)), m.collect(), true)
    } else {
        let (_, apres) = chemin.split_once("/gvfs/")?;
        let mut m = apres.split('/').filter(|x| !x.is_empty());
        let r = reglages_gvfs(m.next()?)?;
        let val = |cle| r.iter().find(|(c, _)| *c == cle).map(|(_, v)| *v);
        let compte = match (val("domain"), val("user")) {
            (Some(d), Some(u)) => format!("{d};{u}@"),
            (None, Some(u)) => format!("{u}@"),
            _ => String::new(),
        };
        (format!("{compte}{}/{}", val("server")?, val("share")?), m.collect(), true)
    };
    let mut adresse = format!("smb://{racine}/");
    adresse.push_str(&reste.iter().map(|x| encoder(x, garder)).collect::<Vec<_>>().join("/"));
    Some(adresse)
}

/// Ouvre la connexion au partage et renvoie le chemin à lire : le même sous Windows,
/// le dossier local du montage sous Linux.
#[tauri::command]
pub async fn connecter_partage(window: tauri::WebviewWindow, chemin: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        let chemin = std::path::PathBuf::from(chemin.trim().replace('/', r"\"));
        let racine = partage(&chemin).ok_or("Ce n'est pas un dossier réseau.")?;
        let fenetre = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        crate::fichiers::en_tache(move || windows::connecter(fenetre, &racine, &chemin)).await
    }
    #[cfg(target_os = "linux")]
    {
        let adresse = adresse_smb(&chemin).ok_or("Ce n'est pas un dossier réseau.")?;
        let (envoi, reception) = std::sync::mpsc::channel();
        let w = window.clone();
        // GTK ne s'utilise que depuis le fil principal ; la fenêtre de GNOME s'ouvre au-dessus d'Ariane
        window
            .run_on_main_thread(move || {
                let fenetre = w.gtk_window().ok();
                gnome::monter(fenetre.as_ref(), &adresse, move |r| {
                    let _ = envoi.send(r);
                });
            })
            .map_err(|e| e.to_string())?;
        crate::fichiers::en_tache(move || reception.recv().unwrap_or_else(|_| Err("Connexion interrompue.".into()))).await
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (window, chemin);
        Err("Les dossiers réseau ne sont pas gérés sur ce système.".into())
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::path::Path;
    use windows_sys::Win32::NetworkManagement::WNet::{
        WNetAddConnection3W, CONNECT_INTERACTIVE, NETRESOURCEW, RESOURCETYPE_DISK,
    };

    /// Windows essaie le compte de la session, puis affiche sa fenêtre d'identifiants (avec « Mémoriser »).
    pub fn connecter(fenetre: isize, racine: &str, chemin: &Path) -> Result<String, String> {
        let mut nom: Vec<u16> = racine.encode_utf16().chain([0]).collect();
        let ressource = NETRESOURCEW {
            dwType: RESOURCETYPE_DISK,
            lpRemoteName: nom.as_mut_ptr(),
            ..Default::default()
        };
        let code = unsafe {
            WNetAddConnection3W(fenetre as _, &ressource, std::ptr::null(), std::ptr::null(), CONNECT_INTERACTIVE)
        };
        match code {
            0 => Ok(chemin.to_string_lossy().into_owned()),
            // ERROR_CANCELLED
            1223 => Err("Connexion annulée.".into()),
            c => {
                let e = io::Error::from_raw_os_error(c as i32);
                Err(super::message(&e, chemin).unwrap_or_else(|| format!("{e} ({racine})")))
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod gnome {
    use gtk::gio;
    use gtk::gio::prelude::*;

    /// Monte le partage avec la fenêtre d'identifiants de GNOME, puis donne son dossier local.
    pub fn monter(fenetre: Option<&gtk::ApplicationWindow>, adresse: &str, fin: impl FnOnce(Result<String, String>) + 'static) {
        let fichier = gio::File::for_uri(adresse);
        let operation = gtk::MountOperation::new(fenetre);
        let cible = fichier.clone();
        fichier.mount_enclosing_volume(gio::MountMountFlags::NONE, Some(&operation), gio::Cancellable::NONE, move |r| {
            let r = match r {
                Ok(()) => Ok(()),
                Err(e) if e.matches(gio::IOErrorEnum::AlreadyMounted) => Ok(()),
                Err(e) if e.matches(gio::IOErrorEnum::FailedHandled) => Err("Connexion annulée.".to_string()),
                Err(e) => Err(e.message().to_string()),
            };
            fin(r.and_then(|()| {
                cible.path().map(|p| p.to_string_lossy().into_owned()).ok_or_else(|| {
                    "Le partage est connecté mais n'a pas de dossier local : installez gvfs-fuse (sudo dnf install gvfs-fuse), puis fermez la session et rouvrez-la.".to_string()
                })
            }));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn reconnait_les_partages() {
        assert_eq!(partage(Path::new(r"\\192.0.0.119\data\Bureaux_etudes")).as_deref(), Some(r"\\192.0.0.119\data"));
        assert_eq!(partage(Path::new(r"\\srv\data")).as_deref(), Some(r"\\srv\data"));
        assert_eq!(partage(Path::new(r"C:\Users")), None);
        let e = io::Error::from_raw_os_error(1326);
        assert!(message(&e, Path::new(r"\\srv\data\x")).unwrap().starts_with(CONNEXION_REQUISE));
        assert_eq!(message(&e, Path::new(r"C:\x")), None);
    }

    #[test]
    fn adresses_smb() {
        assert_eq!(
            adresse_smb(r"\\192.0.0.119\data\Bureaux_etudes\Documentations produits").as_deref(),
            Some("smb://192.0.0.119/data/Bureaux_etudes/Documentations%20produits")
        );
        assert_eq!(adresse_smb(r"\\srv\data").as_deref(), Some("smb://srv/data/"));
        assert_eq!(adresse_smb("smb://srv/data/Études/a%20b").as_deref(), Some("smb://srv/data/%C3%89tudes/a%20b"));
        assert_eq!(
            adresse_smb("/run/user/1000/gvfs/smb-share:domain=TR,server=192.0.0.119,share=data,user=steph/Direction technique").as_deref(),
            Some("smb://TR;steph@192.0.0.119/data/Direction%20technique")
        );
        assert_eq!(adresse_smb("/home/steph"), None);
        assert_eq!(adresse_smb(r"\\srv"), None);
        assert_eq!(nom_partage_gvfs("smb-share:server=192.0.0.119,share=mes%20docs").as_deref(), Some("mes docs sur 192.0.0.119"));
        assert_eq!(nom_partage_gvfs("sftp:host=x"), None);
    }

    /// Lecture d'un vrai partage : le partage administratif C$ de ce PC.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn lit_un_partage_local() {
        let p = Path::new(r"\\127.0.0.1\C$\Windows");
        assert!(std::fs::read_dir(p).unwrap().count() > 0);
        let e = std::fs::read_dir(r"\\127.0.0.1\partage-qui-n-existe-pas").unwrap_err();
        println!("{e:?} → {:?}", message(&e, Path::new(r"\\127.0.0.1\partage-qui-n-existe-pas")));
    }
}
