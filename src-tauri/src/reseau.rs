//! Dossiers réseau Windows (\\serveur\partage\…) : messages d'erreur clairs, et connexion
//! avec la fenêtre d'identifiants de Windows quand le serveur en demande.

use std::io;
use std::path::{Component, Path, Prefix};

/// Début des messages qui invitent l'interface à proposer la connexion au partage.
pub(crate) const CONNEXION_REQUISE: &str = "Connexion requise";

/// « \\serveur\partage » pour un chemin réseau, sinon None.
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

/// Ouvre la connexion au partage : Windows essaie le compte de la session, puis affiche sa propre
/// fenêtre d'identifiants (avec « Mémoriser »). Le mot de passe ne passe jamais par Ariane.
#[tauri::command]
pub async fn connecter_partage(window: tauri::WebviewWindow, chemin: String) -> Result<(), String> {
    let chemin = std::path::PathBuf::from(chemin.trim().replace('/', r"\"));
    let racine = partage(&chemin).ok_or("Ce n'est pas un dossier réseau.")?;
    #[cfg(windows)]
    let fenetre = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    #[cfg(not(windows))]
    let _ = window;
    crate::fichiers::en_tache(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::NetworkManagement::WNet::{
                WNetAddConnection3W, CONNECT_INTERACTIVE, NETRESOURCEW, RESOURCETYPE_DISK,
            };
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
                0 => Ok(()),
                // ERROR_CANCELLED
                1223 => Err("Connexion annulée.".into()),
                c => {
                    let e = io::Error::from_raw_os_error(c as i32);
                    Err(message(&e, &chemin).unwrap_or_else(|| format!("{e} ({racine})")))
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (racine, chemin);
            Err("La connexion aux partages réseau n'est gérée que sous Windows.".into())
        }
    })
    .await
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
