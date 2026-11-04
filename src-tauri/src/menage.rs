//! Ménage : repère ce qui paraît inutile dans un dossier et ses sous-dossiers (téléchargements
//! interrompus, fichiers temporaires, installateurs déjà utilisés, doublons…) pour le proposer.
//! Rien n'est supprimé ici : l'interface met à la corbeille ce que l'utilisateur choisit, un par
//! un, et `ressortir_de_la_corbeille` le remet en place.

use crate::fichiers::en_tache;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const JOUR: f64 = 86_400.0;
/// Rien de ce qui a été modifié depuis moins longtemps n'est proposé (sauf les restes d'un jour).
const RECENT_JOURS: f64 = 7.0;
const MAX_ELEMENTS: u64 = 300_000;
const TEMPS_MAX: Duration = Duration::from_secs(25);
/// Les doublons plus petits ne valent pas la peine (et ce sont souvent des fichiers de réglages).
const DOUBLON_MIN: u64 = 100 * 1024;
const GROS: u64 = 500 * 1024 * 1024;

#[derive(Serialize, Debug)]
pub struct Suggestion {
    chemin: String,
    nom: String,
    est_dossier: bool,
    taille: u64,
    modifie: Option<u64>,
    /// interrompu, temporaire, installateur, archive, doublon, gros, vieux, vide, compilation
    categorie: &'static str,
    /// Doublon : l'exemplaire qui reste
    original: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Analyse {
    suggestions: Vec<Suggestion>,
    parcourus: u64,
    /// false quand l'analyse s'est arrêtée avant la fin (dossier immense)
    complet: bool,
}

/// Dossiers système ou de réglages, jamais parcourus.
const DOSSIERS_SYSTEME: &[&str] = &[
    "appdata",
    "$recycle.bin",
    "system volume information",
    "program files",
    "program files (x86)",
    "programdata",
    "windows",
    "snap",
];
const INTERROMPUS: &[&str] = &["crdownload", "part", "partial", "download"];
const INSTALLATEURS: &[&str] = &["exe", "msi", "rpm", "deb", "appimage", "dmg", "iso", "pkg", "flatpakref"];
const ARCHIVES: &[&str] = &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".tar", ".zip", ".7z", ".rar"];

/// OneDrive, Dropbox… (Windows) : fichier « en ligne seulement ». Le lire le téléchargerait, et il ne
/// prend pas de place sur le disque : il n'est ni comparé pour les doublons, ni compté comme gros.
#[cfg(windows)]
fn en_ligne_seulement(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    attributs_en_ligne(meta.file_attributes())
}

#[cfg(not(windows))]
fn en_ligne_seulement(_meta: &fs::Metadata) -> bool {
    false
}

/// FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
#[cfg_attr(not(windows), allow(dead_code))]
fn attributs_en_ligne(attributs: u32) -> bool {
    attributs & (0x1000 | 0x4_0000 | 0x40_0000) != 0
}

fn age_jours(maintenant: SystemTime, meta: &fs::Metadata) -> f64 {
    meta.modified()
        .ok()
        .and_then(|m| maintenant.duration_since(m).ok())
        .map(|d| d.as_secs_f64() / JOUR)
        .unwrap_or(0.0)
}

fn millisecondes(meta: &fs::Metadata) -> Option<u64> {
    meta.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

/// « plans.tar.gz » → « plans » (pour reconnaître le dossier où l'archive a été extraite).
fn sans_archive(nom: &str) -> Option<&str> {
    let min = nom.to_lowercase();
    ARCHIVES.iter().find(|e| min.ends_with(*e) && min.len() > e.len()).map(|e| &nom[..nom.len() - e.len()])
}

/// Catégorie d'un fichier, ou None s'il n'a rien de suspect.
fn categorie_fichier(nom: &str, taille: u64, age: f64, dans_telechargements: bool, dossiers_voisins: &HashSet<String>) -> Option<&'static str> {
    let min = nom.to_lowercase();
    let ext = min.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    // Restes d'un téléchargement ou d'un document ouvert : un jour suffit
    if INTERROMPUS.contains(&ext) && age > 1.0 {
        return Some("interrompu");
    }
    let verrou = min.starts_with("~$") || (min.starts_with(".~lock.") && min.ends_with('#'));
    if (verrou || ext == "tmp" || min == "thumbs.db" || min == ".ds_store") && age > 1.0 {
        return Some("temporaire");
    }
    if age < RECENT_JOURS {
        return None;
    }
    if dans_telechargements && INSTALLATEURS.contains(&ext) && age > 30.0 {
        return Some("installateur");
    }
    if sans_archive(nom).is_some_and(|base| dossiers_voisins.contains(&base.to_lowercase())) {
        return Some("archive");
    }
    if taille > GROS && age > 365.0 {
        return Some("gros");
    }
    if dans_telechargements && age > 180.0 {
        return Some("vieux");
    }
    None
}

/// Dossier régénérable par un outil de développement : on peut le supprimer, il revient au besoin.
fn dossier_compilation(chemin: &Path, nom: &str) -> Option<PathBuf> {
    let parent = chemin.parent()?;
    let repere = match nom {
        "node_modules" => parent.join("package.json"),
        "target" => parent.join("Cargo.toml"),
        "__pycache__" => chemin.to_path_buf(),
        _ => return None,
    };
    repere.exists().then_some(repere)
}

fn taille_arbre(chemin: &Path, budget: &mut u64) -> u64 {
    let mut total = 0;
    let mut pile = vec![chemin.to_path_buf()];
    while let Some(d) = pile.pop() {
        let Ok(lecture) = fs::read_dir(&d) else { continue };
        for e in lecture.flatten() {
            if *budget == 0 {
                return total;
            }
            *budget -= 1;
            match e.metadata() {
                Ok(m) if m.is_dir() => pile.push(e.path()),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

fn empreinte_debut(chemin: &Path) -> Option<u64> {
    let mut tampon = vec![0; 64 * 1024];
    let n = File::open(chemin).ok()?.read(&mut tampon).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tampon[..n].hash(&mut h);
    Some(h.finish())
}

/// Comparaison octet par octet : un doublon n'est proposé que s'il est vraiment identique.
fn identiques(a: &Path, b: &Path) -> bool {
    let (Ok(mut fa), Ok(mut fb)) = (File::open(a), File::open(b)) else { return false };
    let (mut ta, mut tb) = (vec![0; 256 * 1024], vec![0; 256 * 1024]);
    loop {
        let na = match fa.read(&mut ta) {
            Ok(n) => n,
            Err(_) => return false,
        };
        if na == 0 {
            return fb.read(&mut tb).map(|n| n == 0).unwrap_or(false);
        }
        if fb.read_exact(&mut tb[..na]).is_err() || ta[..na] != tb[..na] {
            return false;
        }
    }
}

struct Candidat {
    chemin: PathBuf,
    taille: u64,
    modifie: Option<u64>,
}

pub fn analyser(racine: &Path, telechargements: Option<&Path>, exclus: &[PathBuf], proteges: &[PathBuf], maintenant: SystemTime) -> Analyse {
    let debut = Instant::now();
    let moi: Vec<PathBuf> = [std::env::var("APPIMAGE").ok().map(PathBuf::from), std::env::current_exe().ok()].into_iter().flatten().collect();
    // `exclus` : avec tout leur contenu, jamais parcourus. `proteges` : jamais proposés eux-mêmes,
    // mais un dossier protégé (épinglé, de base) est quand même parcouru.
    let exclu = |p: &Path| exclus.iter().any(|x| p.starts_with(x)) || moi.iter().any(|m| m == p);
    let protege = |p: &Path| proteges.iter().any(|x| x == p);
    let mut suggestions = Vec::new();
    let mut par_taille: HashMap<u64, Vec<Candidat>> = HashMap::new();
    let mut parcourus = 0u64;
    let mut complet = true;
    let mut pile = vec![racine.to_path_buf()];

    'parcours: while let Some(dossier) = pile.pop() {
        let Ok(lecture) = fs::read_dir(&dossier) else { continue };
        let entrees: Vec<_> = lecture.flatten().collect();
        // Noms des sous-dossiers, pour reconnaître une archive déjà extraite à côté
        let voisins: HashSet<String> = entrees
            .iter()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().to_lowercase())
            .collect();
        let dans_telechargements = telechargements.is_some_and(|t| dossier.starts_with(t));
        for e in entrees {
            parcourus += 1;
            if parcourus > MAX_ELEMENTS || debut.elapsed() > TEMPS_MAX {
                complet = false;
                break 'parcours;
            }
            let chemin = e.path();
            let Ok(meta) = fs::symlink_metadata(&chemin) else { continue };
            if meta.file_type().is_symlink() || exclu(&chemin) {
                continue;
            }
            let nom = e.file_name().to_string_lossy().into_owned();
            let age = age_jours(maintenant, &meta);
            if meta.is_dir() {
                if nom.starts_with('.') || DOSSIERS_SYSTEME.contains(&nom.to_lowercase().as_str()) {
                    continue;
                }
                if let Some(repere) = dossier_compilation(&chemin, &nom) {
                    let repere_age = fs::metadata(&repere).map(|m| age_jours(maintenant, &m)).unwrap_or(0.0);
                    if age > 90.0 && repere_age > 90.0 && !protege(&chemin) {
                        let mut budget = 200_000;
                        let taille = taille_arbre(&chemin, &mut budget);
                        suggestions.push(Suggestion { chemin: chemin.to_string_lossy().into_owned(), nom, est_dossier: true, taille, modifie: millisecondes(&meta), categorie: "compilation", original: None });
                    }
                    continue; // jamais de détail à l'intérieur
                }
                let vide = fs::read_dir(&chemin).map(|mut l| l.next().is_none()).unwrap_or(false);
                if vide {
                    if age > RECENT_JOURS && !protege(&chemin) {
                        suggestions.push(Suggestion { chemin: chemin.to_string_lossy().into_owned(), nom, est_dossier: true, taille: 0, modifie: millisecondes(&meta), categorie: "vide", original: None });
                    }
                    continue;
                }
                pile.push(chemin);
                continue;
            }
            let taille = meta.len();
            if protege(&chemin) {
                continue;
            }
            let en_ligne = en_ligne_seulement(&meta);
            let categorie = categorie_fichier(&nom, taille, age, dans_telechargements, &voisins).filter(|c| !(en_ligne && *c == "gros"));
            if let Some(categorie) = categorie {
                suggestions.push(Suggestion { chemin: chemin.to_string_lossy().into_owned(), nom, est_dossier: false, taille, modifie: millisecondes(&meta), categorie, original: None });
            } else if taille >= DOUBLON_MIN && age > RECENT_JOURS && !nom.starts_with('.') && !en_ligne {
                par_taille.entry(taille).or_default().push(Candidat { chemin, taille, modifie: millisecondes(&meta) });
            }
        }
    }

    // Doublons : même taille, même début, puis comparaison complète avec l'exemplaire gardé
    for (_, groupe) in par_taille.into_iter().filter(|(_, g)| g.len() > 1) {
        if debut.elapsed() > TEMPS_MAX {
            complet = false;
            break;
        }
        let mut par_debut: HashMap<u64, Vec<Candidat>> = HashMap::new();
        for c in groupe {
            if let Some(h) = empreinte_debut(&c.chemin) {
                par_debut.entry(h).or_default().push(c);
            }
        }
        for (_, mut memes) in par_debut.into_iter().filter(|(_, g)| g.len() > 1) {
            // On garde le plus ancien (souvent l'original), à égalité le chemin le plus court
            memes.sort_by_key(|c| (c.modifie.unwrap_or(u64::MAX), c.chemin.as_os_str().len()));
            let original = memes.remove(0);
            for c in memes {
                if identiques(&original.chemin, &c.chemin) {
                    let nom = c.chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    suggestions.push(Suggestion { chemin: c.chemin.to_string_lossy().into_owned(), nom, est_dossier: false, taille: c.taille, modifie: c.modifie, categorie: "doublon", original: Some(original.chemin.to_string_lossy().into_owned()) });
                }
            }
        }
    }
    suggestions.sort_by(|a, b| b.taille.cmp(&a.taille));
    Analyse { suggestions, parcourus, complet }
}

/// Analyse `racine` et ses sous-dossiers. `exclus` : éléments tagués ou gardés, avec tout leur
/// contenu. `epingles` : ils restent, mais pas forcément ce qu'ils contiennent (épingler
/// Téléchargements pour y aller vite ne veut pas dire tout y garder).
#[tauri::command]
pub async fn analyser_menage(racine: String, exclus: Vec<String>, epingles: Vec<String>) -> Result<Analyse, String> {
    en_tache(move || {
        let racine = PathBuf::from(racine);
        if !racine.is_dir() {
            return Err(format!("« {} » n'est pas un dossier.", racine.to_string_lossy()));
        }
        let exclus: Vec<PathBuf> = exclus.into_iter().map(PathBuf::from).collect();
        // Les dossiers de base restent, même vides
        let proteges: Vec<PathBuf> = [dirs::home_dir(), dirs::desktop_dir(), dirs::document_dir(), dirs::download_dir(), dirs::picture_dir(), dirs::audio_dir(), dirs::video_dir(), dirs::public_dir(), dirs::template_dir()]
            .into_iter()
            .flatten()
            .chain([racine.clone()])
            .chain(epingles.into_iter().map(PathBuf::from))
            .collect();
        Ok(analyser(&racine, dirs::download_dir().as_deref(), &exclus, &proteges, SystemTime::now()))
    })
    .await
}

/// Annuler : remet à leur place les éléments que le ménage vient de mettre à la corbeille.
#[tauri::command]
pub async fn ressortir_de_la_corbeille(chemins: Vec<String>) -> Result<(), String> {
    en_tache(move || {
        let liste = trash::os_limited::list().map_err(|e| format!("Corbeille illisible : {e}"))?;
        let mut elements = Vec::new();
        for c in &chemins {
            // Le plus récent, si un élément du même nom a déjà été jeté avant
            let item = liste.iter().filter(|i| i.original_path() == Path::new(c)).max_by_key(|i| i.time_deleted);
            match item {
                Some(i) => elements.push(i.clone()),
                None => return Err(format!("« {c} » n'est plus dans la corbeille.")),
            }
        }
        trash::os_limited::restore_all(elements).map_err(|e| format!("Impossible de remettre en place : {e}"))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories() {
        let rien = HashSet::new();
        let plans: HashSet<String> = ["plans".to_string()].into();
        assert_eq!(categorie_fichier("film.mp4.crdownload", 10, 3.0, false, &rien), Some("interrompu"));
        assert_eq!(categorie_fichier("film.mp4.part", 10, 0.5, false, &rien), None);
        assert_eq!(categorie_fichier("~$devis.docx", 10, 2.0, false, &rien), Some("temporaire"));
        assert_eq!(categorie_fichier(".~lock.budget.ods#", 10, 2.0, false, &rien), Some("temporaire"));
        assert_eq!(categorie_fichier("setup.exe", 10, 40.0, true, &rien), Some("installateur"));
        assert_eq!(categorie_fichier("setup.exe", 10, 40.0, false, &rien), None);
        assert_eq!(categorie_fichier("Plans.tar.gz", 10, 10.0, false, &plans), Some("archive"));
        assert_eq!(categorie_fichier("plans.zip", 10, 3.0, false, &plans), None);
        assert_eq!(categorie_fichier("film.mkv", 2 * GROS, 400.0, false, &rien), Some("gros"));
        assert_eq!(categorie_fichier("facture.pdf", 10, 200.0, true, &rien), Some("vieux"));
        assert_eq!(categorie_fichier("facture.pdf", 10, 200.0, false, &rien), None);
    }

    #[test]
    fn analyse_d_un_dossier() {
        let racine = std::env::temp_dir().join(format!("ariane-menage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&racine);
        // Les dossiers ont la date d'aujourd'hui : on se place 100 jours plus tard,
        // et chaque fichier reçoit son âge par rapport à ce « maintenant ».
        let maintenant = SystemTime::now() + Duration::from_secs(100 * 86_400);
        let ecrire = |chemin: &str, contenu: &[u8], age_jours: u64| {
            let p = racine.join(chemin);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, contenu).unwrap();
            File::options().write(true).open(&p).unwrap().set_modified(maintenant - Duration::from_secs(age_jours * 86_400)).unwrap();
        };
        let photo: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut autre = photo.clone();
        *autre.last_mut().unwrap() = 7;
        ecrire("dl/setup.exe", b"x", 40);
        ecrire("dl/recent.exe", b"x", 2);
        ecrire("film.mp4.crdownload", b"x", 3);
        ecrire("~$devis.docx", b"x", 2);
        ecrire("plans.zip", b"x", 10);
        ecrire("plans/plan.pdf", b"x", 10);
        ecrire("a/photo.jpg", &photo, 30);
        ecrire("b/photo copie.jpg", &photo, 20);
        ecrire("b/presque.jpg", &autre, 20);
        ecrire("garde/important.tmp", b"x", 30);
        // Dossier épinglé : son contenu reste proposé ; fichier épinglé : jamais
        ecrire("epingle/~$vieux.docx", b"x", 30);
        ecrire("note-epinglee.tmp", b"x", 30);
        ecrire(".cache/x.tmp", b"x", 30);
        ecrire("projet/package.json", b"{}", 120);
        ecrire("projet/node_modules/lib/index.js", b"12345", 120);
        fs::create_dir_all(racine.join("vide")).unwrap();

        let proteges = [racine.clone(), racine.join("epingle"), racine.join("note-epinglee.tmp")];
        let a = analyser(&racine, Some(&racine.join("dl")), &[racine.join("garde")], &proteges, maintenant);
        let mut vus: Vec<(String, &str)> = a
            .suggestions
            .iter()
            .map(|s| (Path::new(&s.chemin).strip_prefix(&racine).unwrap().to_string_lossy().replace('\\', "/"), s.categorie))
            .collect();
        vus.sort();
        assert_eq!(
            vus,
            [
                ("b/photo copie.jpg".to_string(), "doublon"),
                ("dl/setup.exe".into(), "installateur"),
                ("epingle/~$vieux.docx".into(), "temporaire"),
                ("film.mp4.crdownload".into(), "interrompu"),
                ("plans.zip".into(), "archive"),
                ("projet/node_modules".into(), "compilation"),
                ("vide".into(), "vide"),
                ("~$devis.docx".into(), "temporaire"),
            ]
        );
        let doublon = a.suggestions.iter().find(|s| s.categorie == "doublon").unwrap();
        assert!(doublon.original.as_deref().unwrap().ends_with("photo.jpg"));
        assert_eq!(a.suggestions.iter().find(|s| s.categorie == "compilation").unwrap().taille, 5);
        assert!(a.complet);
        fs::remove_dir_all(&racine).ok();
    }

    #[test]
    fn fichiers_en_ligne() {
        assert!(attributs_en_ligne(0x40_0020)); // archive + « en ligne seulement » (OneDrive)
        assert!(attributs_en_ligne(0x1000));
        assert!(!attributs_en_ligne(0x20)); // fichier ordinaire
        assert!(!attributs_en_ligne(0x8_0020)); // OneDrive « toujours garder sur cet appareil »
    }

    /// Le vrai OneDrive de ce PC (à lancer à la main) : l'analyse ne doit télécharger aucun fichier
    /// « en ligne seulement » (leur nombre ne bouge pas).
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn onedrive_de_ce_pc_reste_en_ligne() {
        let Some(racine) = std::env::var_os("OneDrive").map(PathBuf::from) else { return };
        let compter = || {
            let mut n = 0;
            let mut pile = vec![racine.clone()];
            while let Some(d) = pile.pop() {
                for e in fs::read_dir(&d).into_iter().flatten().flatten() {
                    let Ok(m) = fs::symlink_metadata(e.path()) else { continue };
                    if m.is_dir() {
                        pile.push(e.path());
                    } else if en_ligne_seulement(&m) {
                        n += 1;
                    }
                }
            }
            n
        };
        let avant = compter();
        let a = analyser(&racine, None, &[], &[racine.clone()], SystemTime::now());
        let apres = compter();
        println!("{avant} fichiers en ligne seulement ; {} propositions ; {} parcourus", a.suggestions.len(), a.parcourus);
        assert_eq!(avant, apres, "l'analyse a téléchargé des fichiers");
    }

    /// Le vrai dossier Téléchargements de ce PC (à lancer à la main) : ce que le balai y propose.
    #[test]
    #[ignore]
    fn telechargements_de_ce_pc() {
        let dl = dirs::download_dir().unwrap();
        let a = analyser(&dl, Some(&dl), &[], &[dl.clone()], SystemTime::now());
        let mut par_categorie: HashMap<&str, (usize, u64)> = HashMap::new();
        for s in &a.suggestions {
            let c = par_categorie.entry(s.categorie).or_default();
            c.0 += 1;
            c.1 += s.taille;
        }
        println!("{} éléments parcourus, complet : {}", a.parcourus, a.complet);
        for (c, (n, t)) in &par_categorie {
            println!("  {c} : {n} ({} Mo)", t / 1_000_000);
        }
        for s in a.suggestions.iter().take(12) {
            println!("    {} · {} · {:?}", s.categorie, s.nom, s.original.as_deref().map(|o| Path::new(o).file_name().unwrap().to_string_lossy().into_owned()));
        }
    }
}
