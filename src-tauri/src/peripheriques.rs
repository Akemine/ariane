//! Périphériques branchés : clés USB, disques externes, cartes SD, CD et DVD.
//!
//! Linux : la liste vient de `lsblk`, montage et éjection passent par `udisksctl`
//! (comme Nautilus, sans mot de passe pour un périphérique amovible).
//! Windows : lettres de lecteur, avec le type de bus pour reconnaître un disque USB
//! que Windows présente comme « fixe », et une éjection propre comme « Retirer le périphérique ».

use crate::fichiers::en_tache;
use serde::Serialize;

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Peripherique {
    /// « D: » sous Windows, « /dev/sdb1 » sous Linux
    id: String,
    nom: String,
    /// Point de montage ; absent tant qu'une partition (Linux) n'est pas montée
    chemin: Option<String>,
    taille: Option<u64>,
    libre: Option<u64>,
    /// « usb » (clé), « disque » (disque externe), « carte » (SD) ou « optique » (CD, DVD)
    genre: String,
}

fn nom_par_defaut(genre: &str) -> String {
    match genre {
        "disque" => "Disque externe",
        "carte" => "Carte mémoire",
        "optique" => "CD / DVD",
        _ => "Clé USB",
    }
    .into()
}

#[tauri::command]
pub async fn lister_peripheriques() -> Vec<Peripherique> {
    tauri::async_runtime::spawn_blocking(plateforme::liste).await.unwrap_or_default()
}

/// Monte une partition (Linux) et renvoie son point de montage.
#[tauri::command]
pub async fn monter_peripherique(id: String) -> Result<String, String> {
    en_tache(move || plateforme::monter(&id)).await
}

/// Démonte et éjecte le périphérique qui porte `id` : on peut ensuite le débrancher.
#[tauri::command]
pub async fn ejecter_peripherique(id: String) -> Result<(), String> {
    en_tache(move || plateforme::ejecter(&id)).await
}

// ———————————————————————————— Linux ————————————————————————————

#[cfg(not(windows))]
mod plateforme {
    use super::{nom_par_defaut, Peripherique};
    use serde_json::Value;
    use std::process::Command;

    /// Points de montage qui signalent le disque du système : jamais proposé à l'éjection.
    const SYSTEME: &[&str] = &["/", "/boot", "/boot/efi", "/efi", "/home", "/usr", "/var", "[SWAP]"];

    /// Lance un outil en anglais (messages d'erreur reconnaissables) ; renvoie sa sortie ou son erreur.
    fn executer(programme: &str, args: &[&str]) -> Result<String, String> {
        let sortie = Command::new(programme)
            .args(args)
            .env("LC_ALL", "C")
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => format!("« {programme} » est introuvable sur ce système."),
                _ => format!("Impossible de lancer {programme} : {e}"),
            })?;
        if sortie.status.success() {
            Ok(String::from_utf8_lossy(&sortie.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&sortie.stderr).trim().to_string())
        }
    }

    /// Disques et partitions, en arbre (les partitions dans « children »).
    fn appareils() -> Vec<Value> {
        let Ok(json) = executer(
            "lsblk",
            &["-J", "-b", "-o", "PATH,TYPE,FSTYPE,LABEL,MODEL,SIZE,FSAVAIL,MOUNTPOINT,RM,HOTPLUG,TRAN"],
        ) else {
            return Vec::new();
        };
        serde_json::from_str::<Value>(&json)
            .ok()
            .and_then(|v| v["blockdevices"].as_array().cloned())
            .unwrap_or_default()
    }

    // Selon sa version, lsblk écrit les booléens et les nombres en texte ou en JSON
    fn vrai(v: &Value) -> bool {
        v.as_bool().unwrap_or(false) || v.as_str() == Some("1")
    }
    fn nombre(v: &Value) -> Option<u64> {
        v.as_u64().or_else(|| v.as_str()?.parse().ok())
    }
    fn chaine(v: &Value) -> Option<String> {
        v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
    }
    fn enfants(v: &Value) -> Vec<&Value> {
        v["children"].as_array().map(|c| c.iter().collect()).unwrap_or_default()
    }

    /// Le nœud et tout ce qu'il contient (partitions, volumes LVM…).
    fn descendants<'a>(v: &'a Value, liste: &mut Vec<&'a Value>) {
        liste.push(v);
        for e in enfants(v) {
            descendants(e, liste);
        }
    }

    pub(super) fn amovibles(disques: &[Value]) -> Vec<Peripherique> {
        let mut liste = Vec::new();
        for disque in disques {
            let tran = disque["tran"].as_str().unwrap_or("");
            let chemin_disque = chaine(&disque["path"]).unwrap_or_default();
            let optique = disque["type"].as_str() == Some("rom");
            let carte = tran == "mmc" || chemin_disque.starts_with("/dev/mmcblk");
            let amovible = vrai(&disque["rm"]) || vrai(&disque["hotplug"]) || tran == "usb";
            let mut tout = Vec::new();
            descendants(disque, &mut tout);
            let systeme = tout
                .iter()
                .filter_map(|v| v["mountpoint"].as_str())
                .any(|m| SYSTEME.contains(&m));
            if !(amovible || optique) || systeme {
                continue;
            }
            let genre = if optique {
                "optique"
            } else if carte {
                "carte"
            } else if vrai(&disque["rm"]) {
                "usb"
            } else {
                "disque" // disque externe : non « amovible » mais branché à chaud
            };
            let modele = chaine(&disque["model"]);
            // Une clé formatée sans table de partitions : le disque est lui-même le volume
            let volumes = if enfants(disque).is_empty() { vec![disque] } else { enfants(disque) };
            for v in volumes {
                // Sans système de fichiers (lecteur vide, partition réservée) ou chiffré : rien à ouvrir
                let Some(fs) = chaine(&v["fstype"]) else { continue };
                if fs == "swap" || fs.starts_with("crypto") || fs.ends_with("_member") {
                    continue;
                }
                liste.push(Peripherique {
                    id: chaine(&v["path"]).unwrap_or_default(),
                    nom: chaine(&v["label"]).or_else(|| modele.clone()).unwrap_or_else(|| nom_par_defaut(genre)),
                    chemin: chaine(&v["mountpoint"]),
                    taille: nombre(&v["size"]),
                    libre: nombre(&v["fsavail"]),
                    genre: genre.into(),
                });
            }
        }
        liste
    }

    pub fn liste() -> Vec<Peripherique> {
        amovibles(&appareils())
    }

    fn expliquer(action: &str, erreur: &str) -> String {
        if erreur.contains("busy") {
            "Le périphérique est encore utilisé : fermez les fichiers, fenêtres ou terminaux ouverts dessus, puis réessayez.".into()
        } else if erreur.contains("NotAuthorized") || erreur.contains("not authorized") {
            format!("Autorisation refusée pour {action} ce périphérique.")
        } else {
            format!("Impossible de {action} : {erreur}")
        }
    }

    pub fn monter(id: &str) -> Result<String, String> {
        executer("udisksctl", &["mount", "-b", id]).map_err(|e| expliquer("monter", &e))?;
        // Le point de montage choisi par udisks (/run/media/moi/CLE)
        executer("lsblk", &["-n", "-o", "MOUNTPOINT", id])
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Le périphérique est monté, mais son dossier est introuvable.".into())
    }

    pub fn ejecter(id: &str) -> Result<(), String> {
        let disques = appareils();
        let disque = disques
            .iter()
            .find(|d| {
                let mut tout = Vec::new();
                descendants(d, &mut tout);
                tout.iter().any(|v| v["path"].as_str() == Some(id))
            })
            .ok_or("Ce périphérique n'est plus branché.")?;
        // Toutes les partitions montées du disque, pas seulement celle cliquée
        let mut tout = Vec::new();
        descendants(disque, &mut tout);
        for v in tout.iter().filter(|v| chaine(&v["mountpoint"]).is_some()) {
            let p = chaine(&v["path"]).unwrap_or_default();
            executer("udisksctl", &["unmount", "-b", &p]).map_err(|e| expliquer("démonter", &e))?;
        }
        // Couper l'alimentation : la clé s'éteint. Certains lecteurs ne savent pas le faire,
        // ce n'est pas grave : tout est démonté, on peut débrancher.
        let chemin = chaine(&disque["path"]).unwrap_or_default();
        if disque["type"].as_str() == Some("rom") {
            let _ = executer("eject", &[&chemin]);
        } else {
            let _ = executer("udisksctl", &["power-off", "-b", &chemin]);
        }
        Ok(())
    }
}

// ———————————————————————————— Windows ————————————————————————————

#[cfg(windows)]
mod plateforme {
    use super::{nom_par_defaut, Peripherique};
    use std::mem::size_of;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Parent, CM_Request_Device_EjectW, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
        SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW, CR_SUCCESS, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT,
        SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        BusTypeMmc, BusTypeSd, BusTypeUsb, CreateFileW, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
        GetVolumeInformationW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{SetThreadErrorMode, SEM_FAILCRITICALERRORS};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        PropertyStandardQuery, StorageDeviceProperty, FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME,
        GUID_DEVINTERFACE_CDROM, GUID_DEVINTERFACE_DISK, IOCTL_STORAGE_EJECT_MEDIA, IOCTL_STORAGE_GET_DEVICE_NUMBER,
        IOCTL_STORAGE_MEDIA_REMOVAL, IOCTL_STORAGE_QUERY_PROPERTY, PREVENT_MEDIA_REMOVAL, STORAGE_DEVICE_DESCRIPTOR,
        STORAGE_DEVICE_NUMBER, STORAGE_PROPERTY_QUERY,
    };

    const LECTEUR_AMOVIBLE: u32 = 2;
    const LECTEUR_FIXE: u32 = 3;
    const LECTEUR_OPTIQUE: u32 = 5;

    fn large(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn depuis_large(t: &[u16]) -> String {
        let n = t.iter().position(|&c| c == 0).unwrap_or(t.len());
        String::from_utf16_lossy(&t[..n])
    }

    /// Poignée Windows fermée automatiquement.
    struct Poignee(HANDLE);
    impl Drop for Poignee {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Ouvre un périphérique sans le lire (aucun droit d'accès : juste pour l'interroger).
    fn ouvrir(chemin: &str) -> Option<Poignee> {
        let h = unsafe {
            CreateFileW(large(chemin).as_ptr(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, null(), OPEN_EXISTING, 0, null_mut())
        };
        (h != INVALID_HANDLE_VALUE).then_some(Poignee(h))
    }

    /// Les boîtes « Insérez un disque dans le lecteur » ne doivent pas apparaître pendant la recherche.
    struct SansAlerte(u32);
    impl SansAlerte {
        fn activer() -> Self {
            let mut ancien = 0;
            unsafe { SetThreadErrorMode(SEM_FAILCRITICALERRORS, &mut ancien) };
            SansAlerte(ancien)
        }
    }
    impl Drop for SansAlerte {
        fn drop(&mut self) {
            unsafe { SetThreadErrorMode(self.0, null_mut()) };
        }
    }

    /// Type de bus (USB, SD…) et modèle (« Samsung Portable SSD T5 ») du disque qui porte la lettre.
    fn description(lettre: char) -> Option<(i32, String)> {
        let h = ouvrir(&format!(r"\\.\{lettre}:"))?;
        let requete = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut tampon = [0u8; 1024];
        let mut lus = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                h.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &requete as *const _ as *const _,
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                tampon.as_mut_ptr() as *mut _,
                tampon.len() as u32,
                &mut lus,
                null_mut(),
            )
        };
        let lus = lus as usize;
        if ok == 0 || lus < size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
            return None;
        }
        let d: STORAGE_DEVICE_DESCRIPTOR = unsafe { std::ptr::read_unaligned(tampon.as_ptr() as *const _) };
        let texte = |decalage: u32| {
            let debut = decalage as usize;
            if debut == 0 || debut >= lus {
                return String::new();
            }
            let s = &tampon[debut..lus];
            let n = s.iter().position(|&c| c == 0).unwrap_or(s.len());
            String::from_utf8_lossy(&s[..n]).trim().to_string()
        };
        let modele = [texte(d.VendorIdOffset), texte(d.ProductIdOffset)]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Some((d.BusType, modele))
    }

    pub fn liste() -> Vec<Peripherique> {
        let _sans_alerte = SansAlerte::activer();
        let systeme = std::env::var("SystemDrive")
            .ok()
            .and_then(|s| s.chars().next())
            .unwrap_or('C')
            .to_ascii_uppercase();
        let masque = unsafe { GetLogicalDrives() };
        let mut liste = Vec::new();
        // À partir de C: : A: et B: sont les anciens lecteurs de disquettes, très lents à interroger
        for i in 2..26u8 {
            let lettre = (b'A' + i) as char;
            if masque & (1 << i) == 0 || lettre == systeme {
                continue;
            }
            let racine = format!("{lettre}:\\");
            let racine_l = large(&racine);
            let sorte = unsafe { GetDriveTypeW(racine_l.as_ptr()) };
            let (bus, modele) = match sorte {
                LECTEUR_AMOVIBLE | LECTEUR_FIXE => description(lettre).unwrap_or((0, String::new())),
                _ => (0, String::new()),
            };
            let genre = match sorte {
                LECTEUR_OPTIQUE => "optique",
                LECTEUR_AMOVIBLE | LECTEUR_FIXE if bus == BusTypeSd || bus == BusTypeMmc => "carte",
                LECTEUR_AMOVIBLE => "usb",
                // Les disques USB externes se déclarent « fixes » : c'est le bus qui les trahit
                LECTEUR_FIXE if bus == BusTypeUsb => "disque",
                _ => continue, // disque interne, lecteur réseau
            };
            // Échec : lecteur vide (lecteur de cartes sans carte, CD sans disque)
            let mut etiquette = [0u16; 261];
            let present = unsafe {
                GetVolumeInformationW(
                    racine_l.as_ptr(),
                    etiquette.as_mut_ptr(),
                    etiquette.len() as u32,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    0,
                )
            };
            if present == 0 {
                continue;
            }
            let (mut libre, mut total) = (0u64, 0u64);
            let espace = unsafe { GetDiskFreeSpaceExW(racine_l.as_ptr(), &mut libre, &mut total, null_mut()) } != 0;
            let etiquette = depuis_large(&etiquette);
            let base = [etiquette.trim(), modele.as_str()]
                .into_iter()
                .find(|s| !s.is_empty())
                .map(String::from)
                .unwrap_or_else(|| nom_par_defaut(genre));
            liste.push(Peripherique {
                id: format!("{lettre}:"),
                nom: format!("{base} ({lettre}:)"),
                chemin: Some(racine),
                taille: espace.then_some(total),
                libre: espace.then_some(libre),
                genre: genre.into(),
            });
        }
        liste
    }

    pub fn monter(_id: &str) -> Result<String, String> {
        Err("Windows monte lui-même les périphériques.".into())
    }

    /// Numéro du disque (ou du lecteur optique) qui porte un volume ou une interface.
    fn numero(chemin: &str) -> Option<STORAGE_DEVICE_NUMBER> {
        let h = ouvrir(chemin)?;
        let mut n = STORAGE_DEVICE_NUMBER { DeviceType: 0, DeviceNumber: 0, PartitionNumber: 0 };
        let mut lus = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                h.0,
                IOCTL_STORAGE_GET_DEVICE_NUMBER,
                null(),
                0,
                &mut n as *mut _ as *mut _,
                size_of::<STORAGE_DEVICE_NUMBER>() as u32,
                &mut lus,
                null_mut(),
            )
        };
        (ok != 0).then_some(n)
    }

    /// Le disque dans le gestionnaire de périphériques, retrouvé par son numéro.
    fn instance(cherche: &STORAGE_DEVICE_NUMBER, optique: bool) -> Option<u32> {
        let guid = if optique { GUID_DEVINTERFACE_CDROM } else { GUID_DEVINTERFACE_DISK };
        let liste = unsafe { SetupDiGetClassDevsW(&guid, null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE) };
        if liste == INVALID_HANDLE_VALUE as isize {
            return None;
        }
        let mut trouve = None;
        for i in 0.. {
            let mut interface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
            interface.cbSize = size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if unsafe { SetupDiEnumDeviceInterfaces(liste, null(), &guid, i, &mut interface) } == 0 {
                break;
            }
            let mut taille = 0u32;
            unsafe { SetupDiGetDeviceInterfaceDetailW(liste, &interface, null_mut(), 0, &mut taille, null_mut()) };
            if taille == 0 {
                continue;
            }
            // Tampon aligné sur 8 octets, comme la structure qu'il contient
            let mut tampon = vec![0u64; (taille as usize).div_ceil(8)];
            let detail = tampon.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            let mut info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
            info.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
            let ok = unsafe {
                (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
                SetupDiGetDeviceInterfaceDetailW(liste, &interface, detail, taille, null_mut(), &mut info)
            };
            if ok == 0 {
                continue;
            }
            let chemin = unsafe {
                let debut = std::ptr::addr_of!((*detail).DevicePath) as *const u16;
                let max = (taille as usize - 4) / 2;
                depuis_large(std::slice::from_raw_parts(debut, max))
            };
            if let Some(n) = numero(&chemin) {
                if n.DeviceType == cherche.DeviceType && n.DeviceNumber == cherche.DeviceNumber {
                    trouve = Some(info.DevInst);
                    break;
                }
            }
        }
        unsafe { SetupDiDestroyDeviceInfoList(liste) };
        trouve
    }

    fn occupe(lettre: char) -> String {
        format!("« {lettre}: » est encore utilisé : fermez les fichiers et fenêtres ouverts dessus, puis réessayez.")
    }

    /// Envoie un ordre au volume, sans paramètre ni réponse.
    fn ordre(h: &Poignee, code: u32, entree: *const core::ffi::c_void, taille: u32) -> bool {
        let mut lus = 0u32;
        unsafe { DeviceIoControl(h.0, code, entree, taille, null_mut(), 0, &mut lus, null_mut()) != 0 }
    }

    /// Lecteur interne (CD, lecteur de cartes) : on démonte le volume et on éjecte le support,
    /// sans toucher à l'appareil lui-même.
    fn ejecter_support(lettre: char, optique: bool) -> Result<(), String> {
        let acces = if optique { GENERIC_READ } else { GENERIC_READ | GENERIC_WRITE };
        let chemin = large(&format!(r"\\.\{lettre}:"));
        let h = unsafe {
            CreateFileW(chemin.as_ptr(), acces, FILE_SHARE_READ | FILE_SHARE_WRITE, null(), OPEN_EXISTING, 0, null_mut())
        };
        if h == INVALID_HANDLE_VALUE {
            return Err(format!("« {lettre}: » est introuvable."));
        }
        let h = Poignee(h);
        let mut verrouille = false;
        for _ in 0..3 {
            if ordre(&h, FSCTL_LOCK_VOLUME, null(), 0) {
                verrouille = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        if !verrouille {
            return Err(occupe(lettre));
        }
        if !ordre(&h, FSCTL_DISMOUNT_VOLUME, null(), 0) {
            return Err(occupe(lettre));
        }
        let liberer = PREVENT_MEDIA_REMOVAL { PreventMediaRemoval: false };
        ordre(&h, IOCTL_STORAGE_MEDIA_REMOVAL, &liberer as *const _ as *const _, size_of::<PREVENT_MEDIA_REMOVAL>() as u32);
        // Une carte SD ne sort pas toute seule : démontée, on peut la retirer
        ordre(&h, IOCTL_STORAGE_EJECT_MEDIA, null(), 0);
        Ok(())
    }

    pub fn ejecter(id: &str) -> Result<(), String> {
        let lettre = id
            .chars()
            .next()
            .filter(char::is_ascii_alphabetic)
            .ok_or("Lecteur inconnu.")?
            .to_ascii_uppercase();
        let _sans_alerte = SansAlerte::activer();
        let optique = unsafe { GetDriveTypeW(large(&format!("{lettre}:\\")).as_ptr()) } == LECTEUR_OPTIQUE;
        // Seul un appareil USB se retire en entier ; ailleurs, le « parent » serait un contrôleur interne
        if description(lettre).map(|d| d.0) != Some(BusTypeUsb) {
            return ejecter_support(lettre, optique);
        }
        let introuvable = || format!("« {lettre}: » est introuvable : il a peut-être déjà été débranché.");
        let n = numero(&format!(r"\\.\{lettre}:")).ok_or_else(introuvable)?;
        let disque = instance(&n, optique).ok_or_else(introuvable)?;
        // On éjecte l'appareil USB qui porte le disque, comme « Retirer le périphérique en toute sécurité »
        let mut appareil = 0u32;
        if unsafe { CM_Get_Parent(&mut appareil, disque, 0) } != CR_SUCCESS {
            return Err(introuvable());
        }
        for essai in 0..3 {
            let mut veto = 0;
            let mut nom_veto = [0u16; 260];
            let r = unsafe { CM_Request_Device_EjectW(appareil, &mut veto, nom_veto.as_mut_ptr(), nom_veto.len() as u32, 0) };
            if r == CR_SUCCESS && veto == 0 {
                return Ok(());
            }
            if essai < 2 {
                // Un programme vient peut-être de lâcher un fichier : on laisse un instant
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        }
        Err(occupe(lettre))
    }
}

/// Affiche les vrais périphériques branchés : `cargo test peripheriques -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore]
fn afficher_les_peripheriques_branches() {
    for p in plateforme::liste() {
        println!("{p:?}");
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::plateforme::amovibles;
    use serde_json::json;

    #[test]
    fn reconnait_les_cles_et_ignore_le_systeme() {
        let disques = vec![
            // Disque du système : jamais proposé
            json!({"path": "/dev/nvme0n1", "type": "disk", "rm": false, "hotplug": false, "tran": "nvme", "children": [
                {"path": "/dev/nvme0n1p1", "type": "part", "fstype": "vfat", "mountpoint": "/boot/efi"},
                {"path": "/dev/nvme0n1p2", "type": "part", "fstype": "btrfs", "mountpoint": "/"}
            ]}),
            // Clé USB montée (vieux lsblk : booléens et nombres en texte)
            json!({"path": "/dev/sdb", "type": "disk", "rm": "1", "hotplug": "1", "tran": "usb", "model": "Cruzer Blade", "children": [
                {"path": "/dev/sdb1", "type": "part", "fstype": "vfat", "label": "PHOTOS", "size": "16000000000", "fsavail": "4000000000", "mountpoint": "/run/media/moi/PHOTOS"}
            ]}),
            // Disque externe pas encore monté, sans étiquette
            json!({"path": "/dev/sdc", "type": "disk", "rm": false, "hotplug": true, "tran": "usb", "model": "Portable SSD T5", "children": [
                {"path": "/dev/sdc1", "type": "part", "fstype": "exfat", "size": 500000000000u64, "mountpoint": null}
            ]}),
            // Lecteur de cartes vide : pas de système de fichiers
            json!({"path": "/dev/mmcblk0", "type": "disk", "rm": true, "tran": null}),
        ];
        let liste = amovibles(&disques);
        assert_eq!(liste.len(), 2, "{liste:?}");
        assert_eq!(liste[0].nom, "PHOTOS");
        assert_eq!(liste[0].genre, "usb");
        assert_eq!(liste[0].libre, Some(4_000_000_000));
        assert_eq!(liste[0].chemin.as_deref(), Some("/run/media/moi/PHOTOS"));
        assert_eq!(liste[1].nom, "Portable SSD T5");
        assert_eq!(liste[1].genre, "disque");
        assert_eq!(liste[1].chemin, None);
    }
}
