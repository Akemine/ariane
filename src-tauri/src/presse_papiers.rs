//! Presse-papiers du système : un Ctrl+C dans Ariane se colle dans Teams, Outlook, l'Explorateur,
//! Nautilus… et un fichier copié ailleurs se colle dans Ariane.
//!
//! Windows : le format de l'Explorateur (CF_HDROP, avec « Preferred DropEffect » pour couper).
//! Linux : celui de Nautilus (x-special/gnome-copied-files, et text/uri-list pour les autres
//! applications), via le presse-papiers de GTK, qui marche sous Wayland comme sous X11.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Contenu {
    pub chemins: Vec<String>,
    /// « copier » ou « couper »
    pub mode: String,
}

#[tauri::command]
pub async fn ecrire_presse_papiers(window: tauri::WebviewWindow, chemins: Vec<String>, mode: String) -> Result<(), String> {
    plateforme::ecrire(window, Contenu { chemins, mode }).await
}

/// Les fichiers du presse-papiers, ou None s'il contient autre chose (du texte, une image…).
#[tauri::command]
pub async fn lire_presse_papiers(window: tauri::WebviewWindow) -> Result<Option<Contenu>, String> {
    plateforme::lire(window).await
}

/// Après « couper puis coller » : les fichiers ont changé de place, on ne les propose plus.
#[tauri::command]
pub async fn vider_presse_papiers(window: tauri::WebviewWindow) -> Result<(), String> {
    plateforme::vider(window).await
}

/// Linux : « copy\nfile:///…\nfile:///… » (le format de Nautilus).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn lire_gnome(texte: &str, vers_chemin: impl Fn(&str) -> Option<String>) -> Option<Contenu> {
    let mut lignes = texte.lines().map(str::trim).filter(|l| !l.is_empty());
    let mode = match lignes.next()? {
        "cut" => "couper",
        "copy" => "copier",
        _ => return None,
    };
    let chemins: Vec<String> = lignes.filter_map(vers_chemin).collect();
    (!chemins.is_empty()).then(|| Contenu { chemins, mode: mode.into() })
}

#[cfg(windows)]
mod plateforme {
    use super::Contenu;
    use std::{ptr, thread, time::Duration};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
        SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows_sys::Win32::System::Ole::CF_HDROP;
    use windows_sys::Win32::UI::Shell::{DragQueryFileW, DROPFILES};

    const DROPEFFECT_COPY: u32 = 1;
    const DROPEFFECT_MOVE: u32 = 2;

    fn fenetre(window: &tauri::WebviewWindow) -> isize {
        window.hwnd().map(|h| h.0 as isize).unwrap_or(0)
    }

    fn format_effet() -> u32 {
        let nom: Vec<u16> = "Preferred DropEffect".encode_utf16().chain([0]).collect();
        unsafe { RegisterClipboardFormatW(nom.as_ptr()) }
    }

    /// Ouvre le presse-papiers ; une autre application peut le tenir un instant, on réessaie.
    struct Ouvert;
    impl Ouvert {
        fn ouvrir(hwnd: isize) -> Result<Self, String> {
            for _ in 0..10 {
                if unsafe { OpenClipboard(hwnd as HWND) } != 0 {
                    return Ok(Ouvert);
                }
                thread::sleep(Duration::from_millis(30));
            }
            Err("Le presse-papiers est occupé par une autre application.".into())
        }
    }
    impl Drop for Ouvert {
        fn drop(&mut self) {
            unsafe { CloseClipboard() };
        }
    }

    /// Bloc mémoire « déplaçable », comme le veut le presse-papiers, rempli avec `octets`.
    fn bloc(octets: &[u8]) -> Result<*mut core::ffi::c_void, String> {
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, octets.len());
            let p = GlobalLock(h) as *mut u8;
            if h.is_null() || p.is_null() {
                return Err("Mémoire insuffisante pour le presse-papiers.".into());
            }
            ptr::copy_nonoverlapping(octets.as_ptr(), p, octets.len());
            GlobalUnlock(h);
            Ok(h)
        }
    }

    /// DROPFILES suivi des chemins en UTF-16, chacun terminé par un zéro, puis un zéro final.
    pub(super) fn octets_hdrop(chemins: &[String]) -> Vec<u8> {
        let entete = DROPFILES { pFiles: std::mem::size_of::<DROPFILES>() as u32, pt: Default::default(), fNC: 0, fWide: 1 };
        let mut octets = unsafe { std::slice::from_raw_parts(&entete as *const _ as *const u8, std::mem::size_of::<DROPFILES>()) }.to_vec();
        for c in chemins {
            octets.extend(c.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
        }
        octets.extend([0, 0]);
        octets
    }

    pub async fn ecrire(window: tauri::WebviewWindow, contenu: Contenu) -> Result<(), String> {
        let hwnd = fenetre(&window);
        crate::fichiers::en_tache(move || ecrire_sync(hwnd, &contenu)).await
    }

    pub(super) fn ecrire_sync(hwnd: isize, contenu: &Contenu) -> Result<(), String> {
        let _ouvert = Ouvert::ouvrir(hwnd)?;
        unsafe {
            EmptyClipboard();
            let fichiers = bloc(&octets_hdrop(&contenu.chemins))?;
            if SetClipboardData(CF_HDROP as u32, fichiers).is_null() {
                return Err("Impossible de remplir le presse-papiers.".into());
            }
            let effet = if contenu.mode == "couper" { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };
            SetClipboardData(format_effet(), bloc(&effet.to_le_bytes())?);
        }
        Ok(())
    }

    pub async fn lire(window: tauri::WebviewWindow) -> Result<Option<Contenu>, String> {
        let hwnd = fenetre(&window);
        crate::fichiers::en_tache(move || lire_sync(hwnd)).await
    }

    pub(super) fn lire_sync(hwnd: isize) -> Result<Option<Contenu>, String> {
        unsafe {
            if IsClipboardFormatAvailable(CF_HDROP as u32) == 0 {
                return Ok(None);
            }
            let _ouvert = Ouvert::ouvrir(hwnd)?;
            let hdrop = GetClipboardData(CF_HDROP as u32);
            if hdrop.is_null() {
                return Ok(None);
            }
            let nombre = DragQueryFileW(hdrop, u32::MAX, ptr::null_mut(), 0);
            let mut chemins = Vec::new();
            for k in 0..nombre {
                let longueur = DragQueryFileW(hdrop, k, ptr::null_mut(), 0);
                let mut tampon = vec![0u16; longueur as usize + 1];
                DragQueryFileW(hdrop, k, tampon.as_mut_ptr(), tampon.len() as u32);
                chemins.push(String::from_utf16_lossy(&tampon[..longueur as usize]));
            }
            // Coupé dans l'Explorateur : on déplace ; sinon on copie
            let mut mode = "copier";
            let h = GetClipboardData(format_effet());
            if !h.is_null() {
                let p = GlobalLock(h) as *const u32;
                if !p.is_null() {
                    if p.read_unaligned() & DROPEFFECT_MOVE != 0 {
                        mode = "couper";
                    }
                    GlobalUnlock(h);
                }
            }
            Ok((!chemins.is_empty()).then(|| Contenu { chemins, mode: mode.into() }))
        }
    }

    pub async fn vider(window: tauri::WebviewWindow) -> Result<(), String> {
        let hwnd = fenetre(&window);
        crate::fichiers::en_tache(move || {
            let _ouvert = Ouvert::ouvrir(hwnd)?;
            unsafe { EmptyClipboard() };
            Ok(())
        })
        .await
    }
}

#[cfg(target_os = "linux")]
mod plateforme {
    use super::{lire_gnome, Contenu};
    use gtk::prelude::*;
    use gtk::{gdk, gio};
    use std::sync::mpsc;
    use std::time::Duration;

    const GNOME: &str = "x-special/gnome-copied-files";

    fn presse_papiers() -> gtk::Clipboard {
        gtk::Clipboard::get(&gdk::Atom::intern("CLIPBOARD"))
    }

    /// GTK ne s'utilise que depuis le fil principal : on y envoie le travail et on attend la réponse.
    async fn sur_le_fil_principal<T: Send + 'static>(
        window: &tauri::WebviewWindow,
        travail: impl FnOnce(mpsc::Sender<T>) + Send + 'static,
    ) -> Result<T, String> {
        let (envoi, reception) = mpsc::channel();
        window.run_on_main_thread(move || travail(envoi)).map_err(|e| e.to_string())?;
        crate::fichiers::en_tache(move || reception.recv_timeout(Duration::from_secs(3)).map_err(|_| "Le presse-papiers ne répond pas.".to_string())).await
    }

    pub async fn ecrire(window: tauri::WebviewWindow, contenu: Contenu) -> Result<(), String> {
        sur_le_fil_principal(&window, move |fin| {
            let uris: Vec<String> = contenu.chemins.iter().map(|c| gio::File::for_path(c).uri().to_string()).collect();
            let gnome = format!("{}\n{}", if contenu.mode == "couper" { "cut" } else { "copy" }, uris.join("\n"));
            let texte = contenu.chemins.join("\n");
            let cibles = [
                gtk::TargetEntry::new(GNOME, gtk::TargetFlags::empty(), 0),
                gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::empty(), 1),
                gtk::TargetEntry::new("UTF8_STRING", gtk::TargetFlags::empty(), 2),
                gtk::TargetEntry::new("text/plain;charset=utf-8", gtk::TargetFlags::empty(), 2),
            ];
            // Les données restent ici tant qu'Ariane tient le presse-papiers ; GTK les donne à qui les demande
            let ok = presse_papiers().set_with_data(&cibles, move |_, donnees, genre| match genre {
                0 => donnees.set(&donnees.target(), 8, gnome.as_bytes()),
                1 => {
                    let liste: Vec<&str> = uris.iter().map(String::as_str).collect();
                    donnees.set_uris(&liste);
                }
                _ => {
                    donnees.set_text(&texte);
                }
            });
            let _ = fin.send(if ok { Ok(()) } else { Err("Impossible de remplir le presse-papiers.".to_string()) });
        })
        .await?
    }

    pub async fn lire(window: tauri::WebviewWindow) -> Result<Option<Contenu>, String> {
        sur_le_fil_principal(&window, move |fin| {
            let vers_chemin = |u: &str| gio::File::for_uri(u).path().map(|p| p.to_string_lossy().into_owned());
            presse_papiers().request_contents(&gdk::Atom::intern(GNOME), move |pp, donnees| {
                let texte = String::from_utf8_lossy(&donnees.data()).into_owned();
                if let Some(c) = lire_gnome(&texte, vers_chemin) {
                    let _ = fin.send(Some(c));
                    return;
                }
                // Copié depuis une autre application (Dolphin, un navigateur…) : une simple liste d'adresses
                pp.request_uris(move |_, uris| {
                    let chemins: Vec<String> = uris.iter().filter_map(|u| vers_chemin(u)).collect();
                    let _ = fin.send((!chemins.is_empty()).then(|| Contenu { chemins, mode: "copier".into() }));
                });
            });
        })
        .await
    }

    pub async fn vider(window: tauri::WebviewWindow) -> Result<(), String> {
        sur_le_fil_principal(&window, |fin| {
            presse_papiers().clear();
            let _ = fin.send(());
        })
        .await
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod plateforme {
    use super::Contenu;
    pub async fn ecrire(_: tauri::WebviewWindow, _: Contenu) -> Result<(), String> {
        Ok(())
    }
    pub async fn lire(_: tauri::WebviewWindow) -> Result<Option<Contenu>, String> {
        Ok(None)
    }
    pub async fn vider(_: tauri::WebviewWindow) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_de_nautilus() {
        let chemin = |u: &str| u.strip_prefix("file://").map(str::to_string);
        assert_eq!(
            lire_gnome("cut\nfile:///home/moi/a.zip\nfile:///home/moi/b.txt\n", chemin),
            Some(Contenu { chemins: vec!["/home/moi/a.zip".into(), "/home/moi/b.txt".into()], mode: "couper".into() })
        );
        assert_eq!(lire_gnome("copy\nfile:///x", chemin).unwrap().mode, "copier");
        assert_eq!(lire_gnome("bonjour", chemin), None);
        assert_eq!(lire_gnome("copy\n", chemin), None);
    }

    #[cfg(windows)]
    #[test]
    fn bloc_de_l_explorateur() {
        let o = plateforme::octets_hdrop(&["C:\\a".into(), "D:\\é".into()]);
        // En-tête de 20 octets, « C:\a\0 » et « D:\é\0 » en UTF-16, puis le zéro final
        assert_eq!(o.len(), 20 + 5 * 2 + 5 * 2 + 2);
        assert_eq!(&o[..4], &20u32.to_le_bytes());
        assert_eq!(&o[16..20], &1u32.to_le_bytes(), "fWide");
    }

    /// Le vrai presse-papiers de Windows (à lancer à la main) : PowerShell y met un fichier comme
    /// l'Explorateur, Ariane le relit ; Ariane en met deux, PowerShell les relit.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn presse_papiers_windows() {
        let ps = |script: &str| {
            let s = std::process::Command::new("powershell").args(["-NoProfile", "-STA", "-Command", script]).output().unwrap();
            String::from_utf8_lossy(&s.stdout).trim().to_string()
        };
        let dossier = std::env::temp_dir().join(format!("ariane-pp-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let a = dossier.join("plans ete.zip");
        let b = dossier.join("notes.txt");
        std::fs::write(&a, "x").unwrap();
        std::fs::write(&b, "y").unwrap();
        // Le texte déjà dans le presse-papiers est remis à la fin
        let avant = std::env::temp_dir().join("ariane-pp-avant.txt");
        ps(&format!("Get-Clipboard -Raw | Set-Content -NoNewline -Encoding utf8 '{}'", avant.display()));

        ps(&format!("Set-Clipboard -Path '{}'", a.display()));
        let lu = plateforme::lire_sync(0).unwrap().expect("rien lu");
        assert_eq!(lu, Contenu { chemins: vec![a.to_string_lossy().into()], mode: "copier".into() });

        let c = Contenu { chemins: vec![a.to_string_lossy().into(), b.to_string_lossy().into()], mode: "couper".into() };
        match plateforme::ecrire_sync(0, &c) {
            Ok(()) => {
                assert_eq!(plateforme::lire_sync(0).unwrap(), Some(c));
                let vu = ps("(Get-Clipboard -Format FileDropList).FullName -join '|'");
                assert_eq!(vu, format!("{}|{}", a.display(), b.display()));
                println!("écrit par Ariane, relu par PowerShell : {vu}");
            }
            Err(e) => println!("écriture sans fenêtre : {e}"),
        }
        ps(&format!("$t = Get-Content -Raw -Encoding utf8 '{0}'; if ($t) {{ Set-Clipboard -Value $t }} else {{ Set-Clipboard -Value $null }}; Remove-Item '{0}'", avant.display()));
        std::fs::remove_dir_all(&dossier).ok();
    }
}
