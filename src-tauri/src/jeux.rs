//! Launchers de jeux installés (Windows) : Steam, Battle.net, Ubisoft Connect, Epic Games,
//! EA app et GOG Galaxy, avec la liste de leurs jeux et le dossier de chacun.
//!
//! Sources : le registre Windows (« programmes installés » et clés propres à chaque launcher)
//! et les fichiers que les launchers tiennent à jour (bibliothèques Steam, manifestes Epic).
//! Les icônes sont extraites des .exe installés, puis gardées en cache.

use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Clone, Debug)]
pub struct Jeu {
    nom: String,
    dossier: String,
    executable: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Lanceur {
    id: String,
    nom: String,
    dossier: String,
    executable: Option<String>,
    jeux: Vec<Jeu>,
}

#[tauri::command]
pub async fn detecter_jeux() -> Vec<Lanceur> {
    tauri::async_runtime::spawn_blocking(detecter).await.unwrap_or_default()
}

/// Icônes des applications, en images PNG prêtes à afficher (data URL), par chemin d'exécutable.
#[tauri::command]
pub async fn icones_applications(app: tauri::AppHandle, executables: Vec<String>) -> HashMap<String, String> {
    let cache = app.path().app_cache_dir().ok().map(|d| d.join("icones"));
    tauri::async_runtime::spawn_blocking(move || extraire_icones(cache, executables))
        .await
        .unwrap_or_default()
}

// ———————————————————————————— outils communs (testés sous Linux) ————————————————————————————

/// Lit les paires « "clé"  "valeur" » d'un fichier VDF de Steam (libraryfolders.vdf, appmanifest_*.acf).
#[cfg_attr(not(windows), allow(dead_code))]
fn paires_vdf(texte: &str) -> Vec<(String, String)> {
    texte
        .lines()
        .filter_map(|ligne| {
            let morceaux: Vec<&str> = ligne.split('"').collect();
            // "clé"<tab><tab>"valeur" → ["", clé, "\t\t", valeur, ""]
            (morceaux.len() >= 5).then(|| (morceaux[1].to_lowercase(), morceaux[3].replace("\\\\", "\\")))
        })
        .collect()
}

/// « "C:\Jeux\app.exe",0 » → « C:\Jeux\app.exe »
#[cfg_attr(not(windows), allow(dead_code))]
fn nettoyer_icone(brut: &str) -> String {
    let s = brut.trim().trim_matches('"');
    let s = match s.rsplit_once(',') {
        Some((avant, apres)) if apres.trim().trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => avant,
        _ => s,
    };
    s.trim().trim_matches('"').to_string()
}

/// Chemin Windows homogène : barres obliques inversées, sans barre finale.
#[cfg_attr(not(windows), allow(dead_code))]
fn normaliser(chemin: &str) -> String {
    let c = chemin.trim().trim_matches('"').replace('/', "\\");
    let c = c.trim_end_matches('\\');
    // « C: » seul redevient « C:\ »
    if c.len() == 2 && c.ends_with(':') {
        format!("{c}\\")
    } else {
        c.to_string()
    }
}

/// Chemins de dossiers (« C:/Program Files (x86)/World of Warcraft ») trouvés dans un fichier
/// binaire, comme la base product.db de Battle.net. Le contrôle d'existence se fait ensuite.
#[cfg_attr(not(windows), allow(dead_code))]
fn chemins_dans_octets(octets: &[u8]) -> Vec<String> {
    let texte = String::from_utf8_lossy(octets);
    let mut chemins = Vec::new();
    // On découpe sur les caractères de contrôle : un chemin n'en contient jamais
    for morceau in texte.split(|c: char| c.is_control() || c == '\u{FFFD}') {
        let o = morceau.as_bytes();
        for i in 0..o.len().saturating_sub(2) {
            let lecteur = o[i].is_ascii_alphabetic() && (i == 0 || !o[i - 1].is_ascii_alphanumeric());
            if lecteur && o[i + 1] == b':' && (o[i + 2] == b'/' || o[i + 2] == b'\\') {
                let c = morceau[i..].trim().to_string();
                if c.len() > 3 && !chemins.contains(&c) {
                    chemins.push(c);
                }
            }
        }
    }
    chemins
}

/// « _retail_ » → « WoW Retail », « _classic_era_ » → « WoW Classic Era »…
/// Tout autre dossier « _xxx_ » devient « WoW Xxx » (par exemple une nouvelle version du jeu).
#[cfg_attr(not(windows), allow(dead_code))]
fn nom_version_wow(dossier: &str) -> Option<String> {
    let interieur = dossier.strip_prefix('_')?.strip_suffix('_')?;
    if interieur.is_empty() {
        return None;
    }
    let connu = match interieur {
        "retail" => Some("WoW Retail"),
        "classic" => Some("WoW Classic"),
        "classic_era" => Some("WoW Classic Era"),
        "anniversary" => Some("WoW Anniversary"),
        "ptr" => Some("WoW PTR"),
        "xptr" => Some("WoW PTR 2"),
        "beta" => Some("WoW Beta"),
        "classic_ptr" => Some("WoW Classic PTR"),
        "classic_era_ptr" => Some("WoW Classic Era PTR"),
        "classic_beta" => Some("WoW Classic Beta"),
        _ => None,
    };
    Some(connu.map(String::from).unwrap_or_else(|| {
        let mots: Vec<String> = interieur
            .split('_')
            .filter(|m| !m.is_empty())
            .map(|m| {
                if m.len() <= 3 {
                    m.to_uppercase()
                } else {
                    let mut c = m.chars();
                    c.next().map(|p| p.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                }
            })
            .collect();
        format!("WoW {}", mots.join(" "))
    }))
}

/// Parmi les .exe d'un dossier de jeu, celui qui est vraisemblablement le jeu (pour son icône).
#[cfg_attr(not(windows), allow(dead_code))]
fn choisir_exe(noms: &[String], nom_jeu: &str) -> Option<String> {
    const EXCLUS: &[&str] = &[
        "unins", "launcher", "crash", "error", "report", "setup", "redist", "helper", "vc_", "dxsetup",
        "easyanticheat", "battleye", "update", "install", "agent",
    ];
    let simple = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    let cible = simple(nom_jeu);
    noms.iter()
        .filter(|n| n.to_lowercase().ends_with(".exe"))
        .filter(|n| {
            let b = n.to_lowercase();
            !EXCLUS.iter().any(|x| b.contains(x))
        })
        .map(|n| {
            let tige = simple(n.trim_end_matches(".exe").trim_end_matches(".EXE"));
            let score = if tige == cible { 3 } else if !cible.is_empty() && (tige.contains(&cible) || cible.contains(&tige)) { 2 } else { 1 };
            (score, std::cmp::Reverse(n.len()), n)
        })
        .max()
        .map(|(_, _, n)| n.clone())
}

// ———————————————————————————— Windows ————————————————————————————

#[cfg(windows)]
fn detecter() -> Vec<Lanceur> {
    windows::detecter()
}

#[cfg(windows)]
fn extraire_icones(cache: Option<PathBuf>, executables: Vec<String>) -> HashMap<String, String> {
    windows::extraire_icones(cache, executables)
}

/// Hors Windows, rien à détecter. La variable ARIANE_DEMO_JEUX affiche des exemples,
/// pratique pour travailler sur l'interface depuis Linux.
#[cfg(not(windows))]
fn detecter() -> Vec<Lanceur> {
    let Ok(base) = std::env::var("ARIANE_DEMO_JEUX") else { return Vec::new() };
    let jeu = |nom: &str, dossier: &str| Jeu { nom: nom.into(), dossier: format!("{base}/{dossier}"), executable: None };
    vec![
        Lanceur { id: "battlenet".into(), nom: "Battle.net".into(), dossier: format!("{base}/Battle.net"), executable: None,
                  jeux: vec![jeu("World of Warcraft", "World of Warcraft")] },
        Lanceur { id: "steam".into(), nom: "Steam".into(), dossier: format!("{base}/Steam"), executable: None,
                  jeux: vec![jeu("Baldur's Gate 3", "Steam/steamapps/common/Baldurs Gate 3"), jeu("Hades II", "Steam/steamapps/common/Hades II")] },
        Lanceur { id: "ubisoft".into(), nom: "Ubisoft Connect".into(), dossier: format!("{base}/Ubisoft"), executable: None, jeux: vec![] },
    ]
}

#[cfg(not(windows))]
fn extraire_icones(_cache: Option<PathBuf>, _executables: Vec<String>) -> HashMap<String, String> {
    HashMap::new()
}

#[cfg(windows)]
mod windows {
    use super::{chemins_dans_octets, choisir_exe, nettoyer_icone, nom_version_wow, normaliser, paires_vdf, Jeu, Lanceur};
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashMap;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    /// Une entrée de « Applications installées ».
    struct Programme {
        nom: String,
        editeur: String,
        dossier: Option<String>,
        icone: Option<String>,
    }

    fn lire(cle: &RegKey, nom: &str) -> Option<String> {
        cle.get_value::<String, _>(nom).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    fn est_dossier(c: &str) -> bool {
        Path::new(c).is_dir()
    }

    fn est_fichier(c: &str) -> bool {
        Path::new(c).is_file()
    }

    fn nom_dossier(c: &str) -> String {
        Path::new(c).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| c.to_string())
    }

    fn programmes_installes() -> Vec<Programme> {
        let sources = [
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
            (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        ];
        let mut liste = Vec::new();
        for (racine, chemin) in sources {
            let Ok(cle) = RegKey::predef(racine).open_subkey(chemin) else { continue };
            for sous in cle.enum_keys().flatten() {
                let Ok(p) = cle.open_subkey(&sous) else { continue };
                let Some(nom) = lire(&p, "DisplayName") else { continue };
                liste.push(Programme {
                    nom,
                    editeur: lire(&p, "Publisher").unwrap_or_default(),
                    dossier: lire(&p, "InstallLocation").map(|d| normaliser(&d)),
                    icone: lire(&p, "DisplayIcon").map(|i| nettoyer_icone(&i)),
                });
            }
        }
        liste
    }

    /// Premier programme dont le nom commence par l'un des motifs (sans tenir compte de la casse).
    fn trouver<'a>(programmes: &'a [Programme], motifs: &[&str]) -> Option<&'a Programme> {
        programmes.iter().find(|p| {
            let n = p.nom.to_lowercase();
            motifs.iter().any(|m| n.starts_with(&m.to_lowercase()))
        })
    }

    /// Dossier d'un programme : InstallLocation, sinon le dossier de son icône.
    fn dossier_de(p: &Programme) -> Option<String> {
        p.dossier.clone().or_else(|| {
            p.icone.as_ref().and_then(|i| Path::new(i).parent().map(|d| normaliser(&d.to_string_lossy())))
        })
    }

    fn executable(p: &Programme) -> Option<String> {
        p.icone.clone().filter(|i| est_fichier(i))
    }

    fn jeu_depuis(p: &Programme) -> Option<Jeu> {
        Some(Jeu { nom: p.nom.clone(), dossier: dossier_de(p)?, executable: executable(p) })
    }

    pub fn detecter() -> Vec<Lanceur> {
        let programmes = programmes_installes();
        let mut lanceurs = Vec::new();

        if let Some(l) = steam() {
            lanceurs.push(l);
        }

        if let Some(bn) = trouver(&programmes, &["Battle.net"]) {
            if let Some(dossier) = dossier_de(bn) {
                let jeux = jeux_battlenet(&programmes, &dossier);
                lanceurs.push(Lanceur { id: "battlenet".into(), nom: "Battle.net".into(), dossier, executable: executable(bn), jeux });
            }
        }

        if let Some(u) = trouver(&programmes, &["Ubisoft Connect", "Uplay"]) {
            if let Some(dossier) = dossier_de(u) {
                let exe = [r"UbisoftConnect.exe", r"upc.exe"]
                    .iter()
                    .map(|e| format!(r"{dossier}\{e}"))
                    .find(|e| est_fichier(e))
                    .or_else(|| executable(u));
                lanceurs.push(Lanceur { id: "ubisoft".into(), nom: "Ubisoft Connect".into(), dossier, executable: exe, jeux: jeux_ubisoft(&programmes) });
            }
        }

        let jeux_epic = jeux_epic();
        let epic = trouver(&programmes, &["Epic Games Launcher"]);
        if epic.is_some() || !jeux_epic.is_empty() {
            let defaut = r"C:\Program Files (x86)\Epic Games".to_string();
            let dossier = epic.and_then(dossier_de).filter(|d| est_dossier(d)).unwrap_or(defaut);
            let exe = [
                format!(r"{dossier}\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe"),
                format!(r"{dossier}\Launcher\Portal\Binaries\Win32\EpicGamesLauncher.exe"),
            ]
            .into_iter()
            .find(|e| est_fichier(e))
            .or_else(|| epic.and_then(executable));
            lanceurs.push(Lanceur { id: "epic".into(), nom: "Epic Games".into(), dossier, executable: exe, jeux: jeux_epic });
        }

        if let Some(ea) = trouver(&programmes, &["EA app", "Origin"]) {
            let jeux = programmes
                .iter()
                .filter(|p| p.editeur.contains("Electronic Arts"))
                .filter(|p| !p.nom.eq_ignore_ascii_case("EA app") && !p.nom.eq_ignore_ascii_case("Origin"))
                .filter_map(jeu_depuis)
                .collect();
            if let Some(dossier) = dossier_de(ea) {
                lanceurs.push(Lanceur { id: "ea".into(), nom: "EA app".into(), dossier, executable: executable(ea), jeux });
            }
        }

        if let Some(g) = trouver(&programmes, &["GOG GALAXY"]) {
            if let Some(dossier) = dossier_de(g) {
                lanceurs.push(Lanceur { id: "gog".into(), nom: "GOG Galaxy".into(), dossier, executable: executable(g), jeux: jeux_gog() });
            }
        }

        // On ne garde que ce qui existe vraiment sur le disque, sans doublon
        lanceurs.retain(|l| est_dossier(&l.dossier));
        for l in &mut lanceurs {
            let mut vus = std::collections::HashSet::new();
            l.jeux.retain(|j| est_dossier(&j.dossier) && vus.insert(j.dossier.to_lowercase()));
            // Un jeu sans exécutable connu : on cherche le sien dans son dossier, pour l'icône
            for j in &mut l.jeux {
                if j.executable.is_none() {
                    j.executable = exe_principal(&j.dossier, &j.nom);
                }
            }
            l.jeux.sort_by_key(|j| j.nom.to_lowercase());
        }
        lanceurs
    }

    /// Jeux Blizzard : entrées « Applications installées » + dossiers notés par Battle.net
    /// dans sa base product.db. World of Warcraft est éclaté en ses versions (Retail, Classic…).
    fn jeux_battlenet(programmes: &[Programme], dossier_launcher: &str) -> Vec<Jeu> {
        let mut jeux: Vec<Jeu> = programmes
            .iter()
            .filter(|p| p.editeur.contains("Blizzard") && !p.nom.to_lowercase().starts_with("battle.net"))
            .filter_map(jeu_depuis)
            .collect();

        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
        if let Ok(octets) = fs::read(format!(r"{base}\Battle.net\Agent\product.db")) {
            for brut in chemins_dans_octets(&octets) {
                // Un octet parasite peut suivre le chemin : on raccourcit jusqu'à tomber sur un dossier
                let mut c = normaliser(&brut);
                for _ in 0..3 {
                    if est_dossier(&c) {
                        break;
                    }
                    c.pop();
                    c = normaliser(&c);
                }
                let bas = c.to_lowercase();
                let a_ignorer = !est_dossier(&c)
                    || c.len() <= 3
                    || bas.starts_with(&base.to_lowercase())
                    || bas.starts_with(&dossier_launcher.to_lowercase())
                    || bas.contains(r"\appdata\")
                    || jeux.iter().any(|j| j.dossier.eq_ignore_ascii_case(&c));
                if !a_ignorer {
                    jeux.push(Jeu { nom: nom_dossier(&c), dossier: c, executable: None });
                }
            }
        }

        // World of Warcraft : un sous-dossier « _xxx_ » par version installée
        let mut resultat = Vec::new();
        for j in jeux {
            let versions = versions_wow(&j.dossier);
            if versions.is_empty() {
                resultat.push(j);
            } else {
                resultat.extend(versions);
            }
        }
        resultat
    }

    fn versions_wow(dossier: &str) -> Vec<Jeu> {
        let Ok(lecture) = fs::read_dir(dossier) else { return Vec::new() };
        lecture
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let nom = nom_version_wow(&e.file_name().to_string_lossy())?;
                let chemin = e.path().to_string_lossy().into_owned();
                // Une vraie version contient un exécutable Wow*.exe
                let exes = noms_exe(&chemin);
                let exe = exes.iter().filter(|n| n.to_lowercase().starts_with("wow")).min_by_key(|n| n.len())?;
                Some(Jeu { nom, executable: Some(format!(r"{chemin}\{exe}")), dossier: chemin })
            })
            .collect()
    }

    fn noms_exe(dossier: &str) -> Vec<String> {
        fs::read_dir(dossier)
            .map(|l| {
                l.flatten()
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n.to_lowercase().ends_with(".exe"))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn exe_principal(dossier: &str, nom_jeu: &str) -> Option<String> {
        choisir_exe(&noms_exe(dossier), nom_jeu).map(|n| format!(r"{dossier}\{n}"))
    }

    fn steam() -> Option<Lanceur> {
        let chemin = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Valve\Steam")
            .ok()
            .and_then(|c| lire(&c, "SteamPath"))
            .or_else(|| {
                RegKey::predef(HKEY_LOCAL_MACHINE)
                    .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
                    .ok()
                    .and_then(|c| lire(&c, "InstallPath"))
            })?;
        let racine = normaliser(&chemin);

        // Toutes les bibliothèques Steam, y compris sur d'autres disques
        let mut bibliotheques = vec![racine.clone()];
        if let Ok(vdf) = fs::read_to_string(format!(r"{racine}\steamapps\libraryfolders.vdf")) {
            for (cle, valeur) in paires_vdf(&vdf) {
                if cle == "path" {
                    let b = normaliser(&valeur);
                    if !bibliotheques.iter().any(|x| x.eq_ignore_ascii_case(&b)) {
                        bibliotheques.push(b);
                    }
                }
            }
        }

        let mut jeux = Vec::new();
        for b in &bibliotheques {
            let apps = PathBuf::from(format!(r"{b}\steamapps"));
            let Ok(lecture) = fs::read_dir(&apps) else { continue };
            for f in lecture.flatten() {
                let nom_fichier = f.file_name().to_string_lossy().into_owned();
                if !nom_fichier.starts_with("appmanifest_") || !nom_fichier.ends_with(".acf") {
                    continue;
                }
                let Ok(acf) = fs::read_to_string(f.path()) else { continue };
                let paires = paires_vdf(&acf);
                let valeur = |k: &str| paires.iter().find(|(c, _)| c == k).map(|(_, v)| v.clone());
                let (Some(nom), Some(dossier)) = (valeur("name"), valeur("installdir")) else { continue };
                // Les composants techniques de Steam ne sont pas des jeux
                if nom.starts_with("Steamworks") || nom.contains("Redistributable") || nom.starts_with("Proton") {
                    continue;
                }
                jeux.push(Jeu { nom, dossier: format!(r"{}\common\{dossier}", apps.display()), executable: None });
            }
        }

        let exe = format!(r"{racine}\steam.exe");
        Some(Lanceur { id: "steam".into(), nom: "Steam".into(), executable: est_fichier(&exe).then_some(exe), dossier: racine, jeux })
    }

    fn jeux_ubisoft(programmes: &[Programme]) -> Vec<Jeu> {
        let Ok(cle) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs") else {
            return Vec::new();
        };
        cle.enum_keys()
            .flatten()
            .filter_map(|id| {
                let sous = cle.open_subkey(&id).ok()?;
                let dossier = normaliser(&lire(&sous, "InstallDir")?);
                // Le vrai nom du jeu est dans « Applications installées », au même dossier
                let p = programmes.iter().find(|p| p.dossier.as_deref().is_some_and(|d| d.eq_ignore_ascii_case(&dossier)));
                Some(Jeu {
                    nom: p.map(|p| p.nom.clone()).unwrap_or_else(|| nom_dossier(&dossier)),
                    executable: p.and_then(executable),
                    dossier,
                })
            })
            .collect()
    }

    fn jeux_epic() -> Vec<Jeu> {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
        let Ok(lecture) = fs::read_dir(format!(r"{base}\Epic\EpicGamesLauncher\Data\Manifests")) else {
            return Vec::new();
        };
        lecture
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|e| e == "item"))
            .filter_map(|f| {
                let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(f.path()).ok()?).ok()?;
                let nom = v.get("DisplayName")?.as_str()?.to_string();
                let dossier = normaliser(v.get("InstallLocation")?.as_str()?);
                let executable = v
                    .get("LaunchExecutable")
                    .and_then(|e| e.as_str())
                    .filter(|e| !e.is_empty())
                    .map(|e| format!(r"{dossier}\{}", e.replace('/', "\\")))
                    .filter(|e| est_fichier(e));
                Some(Jeu { nom, dossier, executable })
            })
            .collect()
    }

    fn jeux_gog() -> Vec<Jeu> {
        let Ok(cle) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\GOG.com\Games") else {
            return Vec::new();
        };
        cle.enum_keys()
            .flatten()
            .filter_map(|id| {
                let sous = cle.open_subkey(&id).ok()?;
                let dossier = normaliser(&lire(&sous, "path")?);
                Some(Jeu {
                    nom: lire(&sous, "gameName").unwrap_or_else(|| nom_dossier(&dossier)),
                    executable: lire(&sous, "exe").filter(|e| est_fichier(e)),
                    dossier,
                })
            })
            .collect()
    }

    // ———————————————————————————— icônes ————————————————————————————

    /// Extrait l'icône de chaque .exe avec PowerShell (une seule fois, pour tous),
    /// la garde en cache en PNG, et la renvoie en data URL.
    pub fn extraire_icones(cache: Option<PathBuf>, executables: Vec<String>) -> HashMap<String, String> {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        let Some(cache) = cache else { return HashMap::new() };
        let _ = fs::create_dir_all(&cache);
        let fichier_cache = |exe: &str| {
            let mut h = DefaultHasher::new();
            exe.to_lowercase().hash(&mut h);
            cache.join(format!("{:016x}.png", h.finish()))
        };

        let manquants: Vec<(String, PathBuf)> = executables
            .iter()
            .filter(|e| est_fichier(e))
            .map(|e| (e.clone(), fichier_cache(e)))
            .filter(|(_, f)| !f.is_file())
            .collect();

        if !manquants.is_empty() {
            let liste = cache.join("a-extraire.txt");
            let contenu: String = manquants.iter().map(|(e, f)| format!("{}\t{}\r\n", e, f.display())).collect();
            if fs::write(&liste, contenu).is_ok() {
                let script = format!(
                    r#"Add-Type -AssemblyName System.Drawing
foreach ($ligne in Get-Content -LiteralPath '{}' -Encoding UTF8) {{
  $p = $ligne -split "`t"
  try {{
    $icone = [System.Drawing.Icon]::ExtractAssociatedIcon($p[0])
    if ($icone) {{
      $image = $icone.ToBitmap()
      $image.Save($p[1], [System.Drawing.Imaging.ImageFormat]::Png)
      $image.Dispose(); $icone.Dispose()
    }}
  }} catch {{}}
}}"#,
                    liste.display().to_string().replace('\'', "''")
                );
                // -EncodedCommand attend le script en UTF-16LE encodé en base64
                let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
                let mut commande = Command::new("powershell");
                commande.args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &STANDARD.encode(utf16)]);
                crate::systeme::sans_fenetre(&mut commande);
                let _ = commande.output();
                let _ = fs::remove_file(&liste);
            }
        }

        executables
            .into_iter()
            .filter_map(|e| {
                let octets = fs::read(fichier_cache(&e)).ok()?;
                Some((e, format!("data:image/png;base64,{}", STANDARD.encode(octets))))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_vdf() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t\t\"label\"\t\t\"\"\n\t}\n}";
        let p = paires_vdf(vdf);
        assert!(p.contains(&("path".into(), "D:\\SteamLibrary".into())));
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1086940\"\n\t\"name\"\t\t\"Baldur's Gate 3\"\n\t\"installdir\"\t\t\"Baldurs Gate 3\"\n}";
        let p = paires_vdf(acf);
        assert!(p.contains(&("name".into(), "Baldur's Gate 3".into())));
        assert!(p.contains(&("installdir".into(), "Baldurs Gate 3".into())));
    }

    #[test]
    fn chemins_de_battlenet() {
        // Imitation d'un extrait de product.db : chaînes protobuf collées à des octets de contrôle
        let mut octets = vec![0x0a, 0x03];
        octets.extend_from_slice(b"wow");
        octets.extend_from_slice(&[0x12, 0x29]);
        octets.extend_from_slice(b"C:/Program Files (x86)/World of Warcraft");
        octets.extend_from_slice(&[0x1a, 0x05]);
        octets.extend_from_slice(b"frFR");
        octets.extend_from_slice(&[0x22, 0x1f]);
        octets.extend_from_slice(b"D:\\Jeux\\Diablo IV");
        octets.push(0x00);
        let c = chemins_dans_octets(&octets);
        assert!(c.contains(&"C:/Program Files (x86)/World of Warcraft".to_string()), "{c:?}");
        assert!(c.iter().any(|x| x.starts_with("D:\\Jeux\\Diablo IV")), "{c:?}");
    }

    #[test]
    fn versions_de_wow() {
        assert_eq!(nom_version_wow("_retail_").as_deref(), Some("WoW Retail"));
        assert_eq!(nom_version_wow("_classic_era_").as_deref(), Some("WoW Classic Era"));
        assert_eq!(nom_version_wow("_forever_").as_deref(), Some("WoW Forever"));
        assert_eq!(nom_version_wow("_CommonRedist").as_deref(), None);
        assert_eq!(nom_version_wow("Interface"), None);
    }

    #[test]
    fn exe_du_jeu() {
        let noms: Vec<String> = ["Overwatch Launcher.exe", "Overwatch.exe", "BlizzardError.exe"].iter().map(|s| s.to_string()).collect();
        assert_eq!(choisir_exe(&noms, "Overwatch").as_deref(), Some("Overwatch.exe"));
        let noms: Vec<String> = ["Diablo IV.exe", "unins000.exe"].iter().map(|s| s.to_string()).collect();
        assert_eq!(choisir_exe(&noms, "Diablo IV").as_deref(), Some("Diablo IV.exe"));
        assert_eq!(choisir_exe(&[], "Rien"), None);
    }

    #[test]
    fn icones_et_chemins() {
        assert_eq!(nettoyer_icone(r#""C:\Program Files (x86)\Battle.net\Battle.net.exe",0"#), r"C:\Program Files (x86)\Battle.net\Battle.net.exe");
        assert_eq!(nettoyer_icone(r"C:\Jeux\jeu.exe,-101"), r"C:\Jeux\jeu.exe");
        assert_eq!(nettoyer_icone(r"C:\Jeux\jeu.ico"), r"C:\Jeux\jeu.ico");
        assert_eq!(normaliser("C:/Program Files (x86)/Ubisoft/Games/Anno 1800/"), r"C:\Program Files (x86)\Ubisoft\Games\Anno 1800");
        assert_eq!(normaliser("D:\\"), "D:\\");
    }

    /// L'icône d'un vrai .exe de ce PC (à lancer à la main) : ARIANE_EXE=chemin, écrite dans ARIANE_SORTIE.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn icone_d_un_exe() {
        let (Ok(exe), Ok(sortie)) = (std::env::var("ARIANE_EXE"), std::env::var("ARIANE_SORTIE")) else { return };
        let cache = std::env::temp_dir().join(format!("ariane-icones-{}", std::process::id()));
        let icones = extraire_icones(Some(cache.clone()), vec![exe.clone()]);
        std::fs::remove_dir_all(cache).ok();
        let image = icones.get(&exe).expect("pas d'icône");
        assert!(image.starts_with("data:image/"));
        std::fs::write(sortie, image).unwrap();
    }
}
