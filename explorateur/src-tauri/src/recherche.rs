//! Recherche par nom dans le dossier personnel et dans d'autres dossiers de départ
//! (dossiers ouverts, épingles, jeux), pour trouver aussi ce qui est hors du dossier personnel,
//! comme « C:\Program Files (x86)\World of Warcraft\_retail_\Interface\AddOns ».
//!
//! Parcours en largeur : les résultats proches de l'accueil arrivent en premier.
//! On saute les dossiers lourds sans intérêt (node_modules, target, caches…) et les
//! dossiers cachés, sauf si la recherche commence par un point.
//! Chaque nouvelle frappe annule la recherche précédente.

use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Numéro de la recherche en cours : une recherche plus ancienne s'arrête d'elle-même.
static GENERATION: AtomicU64 = AtomicU64::new(0);
const DUREE_MAX: Duration = Duration::from_millis(1500);
const MAX_RESULTATS: usize = 40;
const PROFONDEUR_MAX: usize = 14;
const IGNORES: &[&str] = &[
    "node_modules", "target", "__pycache__", "venv", "AppData", "$RECYCLE.BIN",
    "System Volume Information",
];

#[derive(Serialize)]
pub struct Trouve {
    nom: String,
    chemin: String,
    parent: String,
    est_dossier: bool,
    extension: String,
    #[serde(skip)]
    score: (u8, u8, usize, usize),
}

#[derive(Serialize)]
pub struct ResultatRecherche {
    resultats: Vec<Trouve>,
    /// faux si la recherche a été coupée (trop longue) : il peut manquer des résultats
    complet: bool,
}

/// Minuscules et sans accents : « Écran » et « ecran » se retrouvent.
fn replier(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            'ÿ' => 'y',
            c => c,
        })
        .collect()
}

#[tauri::command]
pub async fn chercher(racines: Vec<String>, requete: String) -> Result<ResultatRecherche, String> {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let racines = racines.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || parcourir(racines, &requete, generation))
        .await
        .map_err(|e| e.to_string())
}

fn parcourir(racines: Vec<PathBuf>, requete: &str, generation: u64) -> ResultatRecherche {
    let q = replier(requete.trim());
    let mut trouves: Vec<Trouve> = Vec::new();
    if q.is_empty() {
        return ResultatRecherche { resultats: trouves, complet: true };
    }
    let voir_caches = q.starts_with('.');
    let debut = Instant::now();
    let mut complet = true;
    // Dossiers déjà lus (les départs peuvent se recouvrir : un dossier ouvert est souvent dans l'accueil)
    let mut vus: HashSet<String> = HashSet::new();
    let mut a_visiter = VecDeque::new();
    for (k, racine) in racines.into_iter().enumerate() {
        if !racine.is_dir() {
            continue;
        }
        // Un dossier de départ peut lui-même correspondre (« _forever_ »), sauf l'accueil (le premier)
        if k > 0 {
            if let (Some(nom), Some(parent)) = (racine.file_name(), racine.parent()) {
                let nom = nom.to_string_lossy().into_owned();
                let replie = replier(&nom);
                if let Some(position) = replie.find(&q) {
                    let rang = if replie == q { 0 } else if position == 0 { 1 } else { 2 };
                    trouves.push(Trouve {
                        extension: String::new(),
                        chemin: racine.to_string_lossy().into_owned(),
                        parent: parent.to_string_lossy().into_owned(),
                        score: (rang, 0, 0, nom.len()),
                        nom,
                        est_dossier: true,
                    });
                }
            }
        }
        a_visiter.push_back((racine, 0usize));
    }

    while let Some((dossier, profondeur)) = a_visiter.pop_front() {
        if GENERATION.load(Ordering::Relaxed) != generation || debut.elapsed() > DUREE_MAX {
            complet = false;
            break;
        }
        if !vus.insert(dossier.to_string_lossy().to_lowercase()) {
            continue;
        }
        let Ok(lecture) = fs::read_dir(&dossier) else { continue };
        let parent = dossier.to_string_lossy().into_owned();
        for entree in lecture.flatten() {
            let nom = entree.file_name().to_string_lossy().into_owned();
            // Les liens symboliques ne sont pas suivis (évite les boucles)
            let Ok(genre) = entree.file_type() else { continue };
            let est_dossier = genre.is_dir();
            let cache = nom.starts_with('.');
            if cache && !voir_caches {
                continue;
            }
            let a_explorer = est_dossier && profondeur < PROFONDEUR_MAX && !IGNORES.contains(&nom.as_str());
            let replie = replier(&nom);
            if let Some(position) = replie.find(&q) {
                // Nom exact, puis « commence par », puis « contient »
                let rang = if replie == q { 0 } else if position == 0 { 1 } else { 2 };
                let chemin = entree.path();
                trouves.push(Trouve {
                    extension: if est_dossier {
                        String::new()
                    } else {
                        chemin.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default()
                    },
                    chemin: chemin.to_string_lossy().into_owned(),
                    parent: parent.clone(),
                    score: (rang, if est_dossier { 0 } else { 1 }, profondeur, nom.len()),
                    nom,
                    est_dossier,
                });
            }
            if a_explorer {
                a_visiter.push_back((entree.path(), profondeur + 1));
            }
        }
    }

    trouves.sort_by(|a, b| a.score.cmp(&b.score));
    let mut deja = HashSet::new();
    trouves.retain(|t| deja.insert(t.chemin.to_lowercase()));
    trouves.truncate(MAX_RESULTATS);
    ResultatRecherche { resultats: trouves, complet }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trouve_dans_les_sous_dossiers() {
        let d = std::env::temp_dir().join("ariane-test-recherche");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("work/copernic/VM_AUTH")).unwrap();
        fs::create_dir_all(d.join("node_modules/VM_AUTH")).unwrap();
        fs::create_dir_all(d.join(".cache/vm_auth")).unwrap();
        fs::write(d.join("work/notes-vm_auth.md"), "x").unwrap();

        let r = parcourir(vec![d.clone()], "vm_auth", GENERATION.load(Ordering::SeqCst));
        assert!(r.complet);
        let chemins: Vec<&str> = r.resultats.iter().map(|t| t.chemin.as_str()).collect();
        assert_eq!(r.resultats.len(), 2, "{chemins:?}");
        // Le dossier au nom exact passe devant le fichier qui le contient
        assert!(r.resultats[0].chemin.ends_with("VM_AUTH") && r.resultats[0].est_dossier);
        assert!(r.resultats[1].nom == "notes-vm_auth.md");

        // Avec un point au début, les dossiers cachés sont inclus
        let r = parcourir(vec![d.clone()], ".cache", GENERATION.load(Ordering::SeqCst));
        assert_eq!(r.resultats.len(), 1);

        // Plusieurs départs : un dossier hors de l'accueil est fouillé, et peut lui-même correspondre
        let ailleurs = std::env::temp_dir().join("ariane-test-ailleurs/World of Warcraft/_forever_");
        fs::create_dir_all(ailleurs.join("Interface/AddOns")).unwrap();
        let r = parcourir(vec![d.clone(), ailleurs.clone(), d.join("work")], "addons", GENERATION.load(Ordering::SeqCst));
        assert_eq!(r.resultats.len(), 1);
        assert!(r.resultats[0].chemin.ends_with("AddOns"));
        let r = parcourir(vec![d.clone(), ailleurs.clone()], "_forever_", GENERATION.load(Ordering::SeqCst));
        assert_eq!(r.resultats.len(), 1);
        let _ = fs::remove_dir_all(std::env::temp_dir().join("ariane-test-ailleurs"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn accents() {
        assert_eq!(replier("Écran Téléchargé"), "ecran telecharge");
    }
}
