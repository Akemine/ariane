//! Windows 11 : le remplissage des espaces en ligne (OneDrive, et Dropbox ou Google Drive s'ils le
//! fournissent), tel que l'Explorateur l'affiche (« 1,6 Go utilisés sur 5 Go »).
//!
//! Chaque service déclaré à Windows (SyncRootManager) peut donner une « jauge » : on la lui demande
//! comme l'Explorateur, sans connexion à un compte ni accès à Internet de notre part.

use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Jauge {
    /// Dossier local du service (C:\Users\…\OneDrive)
    pub chemin: String,
    pub utilise: u64,
    pub total: u64,
    /// La phrase du service, dans la langue de Windows
    pub libelle: String,
}

#[tauri::command]
pub async fn jauges_nuages() -> Vec<Jauge> {
    tauri::async_runtime::spawn_blocking(plateforme::jauges).await.unwrap_or_default()
}

#[cfg(windows)]
mod plateforme {
    use super::Jauge;
    use ::windows::core::{GUID, HSTRING};
    use ::windows::Storage::Provider::IStorageProviderStatusUISourceFactory;
    use ::windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED};
    use std::path::Path;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    /// Noms des programmes en cours (en minuscules).
    fn programmes_en_cours() -> Vec<String> {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        let mut noms = Vec::new();
        unsafe {
            let instantane = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if instantane == INVALID_HANDLE_VALUE {
                return noms;
            }
            let mut p: PROCESSENTRY32W = std::mem::zeroed();
            p.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(instantane, &mut p) != 0 {
                loop {
                    let fin = p.szExeFile.iter().position(|&c| c == 0).unwrap_or(p.szExeFile.len());
                    noms.push(String::from_utf16_lossy(&p.szExeFile[..fin]).to_lowercase());
                    if Process32NextW(instantane, &mut p) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(instantane);
        }
        noms
    }

    fn demander(clsid: &str, id: &str) -> ::windows::core::Result<(u64, u64, String)> {
        let guid = GUID::try_from(clsid.trim_matches(|c| c == '{' || c == '}'))?;
        let fabrique: IStorageProviderStatusUISourceFactory = unsafe { CoCreateInstance(&guid, None, CLSCTX_LOCAL_SERVER)? };
        let quota = fabrique.GetStatusUISource(&HSTRING::from(id))?.GetStatusUI()?.QuotaUI()?;
        Ok((quota.QuotaUsedInBytes()?, quota.QuotaTotalInBytes()?, quota.QuotaUsedLabel()?.to_string()))
    }

    pub fn jauges() -> Vec<Jauge> {
        let Ok(racines) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager") else {
            return Vec::new();
        };
        let programmes = programmes_en_cours();
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut liste = Vec::new();
        for id in racines.enum_keys().flatten() {
            let Ok(cle) = racines.open_subkey(&id) else { continue };
            let Ok(clsid) = cle.get_value::<String, _>("StorageProviderStatusUISourceFactory") else { continue };
            // « OneDrive!S-1-5-…!Personal » : sans OneDrive.exe en cours, la demande le démarrerait
            let service = id.split('!').next().unwrap_or_default().to_lowercase();
            if service.is_empty() || !programmes.iter().any(|p| p.starts_with(&service)) {
                continue;
            }
            // Le dossier local, rangé par utilisateur ; ceux des autres comptes du PC sont inaccessibles
            let dossiers: Vec<String> = cle
                .open_subkey("UserSyncRoots")
                .map(|u| u.enum_values().flatten().filter_map(|(nom, _)| u.get_value::<String, _>(nom).ok()).collect())
                .unwrap_or_default();
            let Some(chemin) = dossiers.into_iter().find(|d| Path::new(d).is_dir()) else { continue };
            if let Ok((utilise, total, libelle)) = demander(&clsid, &id) {
                if total > 0 {
                    liste.push(Jauge { chemin, utilise, total, libelle });
                }
            }
        }
        liste
    }
}

#[cfg(not(windows))]
mod plateforme {
    pub fn jauges() -> Vec<super::Jauge> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    /// Les jauges de ce PC (à lancer à la main).
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn jauges_de_ce_pc() {
        let j = super::plateforme::jauges();
        println!("{j:#?}");
        for x in &j {
            assert!(x.utilise <= x.total);
        }
    }
}
