//! Opérations sur les fichiers : lister, créer, renommer, copier, déplacer, corbeille.
//!
//! Tout le travail disque se fait hors du fil principal (`spawn_blocking`)
//! pour que l'interface reste fluide pendant une grosse copie.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Nombre maximal d'éléments comptés dans un sous-dossier (affiché « 10 000+ »).
const LIMITE_COMPTE: usize = 10_000;

#[derive(Serialize)]
pub struct Entree {
    nom: String,
    chemin: String,
    est_dossier: bool,
    est_lien: bool,
    taille: Option<u64>,
    nb_elements: Option<u64>,
    /// Date de modification en millisecondes depuis 1970
    modifie: Option<u64>,
    cache: bool,
    extension: String,
}

#[derive(Serialize)]
pub struct Segment {
    nom: String,
    chemin: String,
}

#[derive(Serialize)]
pub struct Contenu {
    chemin: String,
    parent: Option<String>,
    segments: Vec<Segment>,
    entrees: Vec<Entree>,
}

#[derive(Serialize)]
pub struct Standards {
    accueil: String,
    telechargements: Option<String>,
    bureau: Option<String>,
    documents: Option<String>,
    images: Option<String>,
    musique: Option<String>,
    videos: Option<String>,
    separateur: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Operation {
    source: String,
    cible: String,
    /// Identifiant dans la corbeille de l'élément remplacé, pour le remettre si on annule
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remplace: Option<String>,
}

#[derive(Serialize)]
pub struct ResultatCollage {
    operations: Vec<Operation>,
    erreurs: Vec<String>,
}

/// Ce qu'on montre d'un élément pour comparer les deux versions d'un conflit.
#[derive(Serialize)]
pub struct Apercu {
    est_dossier: bool,
    taille: Option<u64>,
    nb_elements: Option<u64>,
    modifie: Option<u64>,
}

/// Un élément collé porte le même nom qu'un élément déjà présent dans la destination.
#[derive(Serialize)]
pub struct Conflit {
    source: String,
    nom: String,
    nouveau: Apercu,
    existant: Apercu,
}

// ———————————————————————————— utilitaires ————————————————————————————

pub(crate) fn texte(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn nom_de(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| texte(p))
}

fn accueil() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

/// Remplace « ~ » par le dossier personnel.
fn developper(chemin: &str) -> PathBuf {
    let chemin = chemin.trim();
    if chemin.is_empty() || chemin == "~" {
        return accueil();
    }
    if let Some(reste) = chemin.strip_prefix("~/").or_else(|| chemin.strip_prefix("~\\")) {
        return accueil().join(reste);
    }
    // « //serveur/partage » vaut « \\serveur\partage » : Windows n'accepte pas « / » dans un nom
    if cfg!(windows) {
        return PathBuf::from(chemin.replace('/', "\\"));
    }
    PathBuf::from(chemin)
}

/// Message d'erreur lisible, en français.
pub(crate) fn message(e: &io::Error, chemin: &Path) -> String {
    if let Some(m) = crate::reseau::message(e, chemin) {
        return m;
    }
    match e.kind() {
        io::ErrorKind::NotFound => format!("« {} » est introuvable.", texte(chemin)),
        io::ErrorKind::PermissionDenied => format!("Accès refusé à « {} ».", texte(chemin)),
        io::ErrorKind::AlreadyExists => format!("« {} » existe déjà.", nom_de(chemin)),
        _ => format!("{} ({})", e, texte(chemin)),
    }
}

/// Lance un travail bloquant hors du fil principal.
pub(crate) async fn en_tache<T, F>(travail: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(travail)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn est_cache(nom: &str, chemin: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const ATTRIBUT_CACHE: u32 = 0x2;
    nom.starts_with('.')
        || fs::symlink_metadata(chemin)
            .map(|m| m.file_attributes() & ATTRIBUT_CACHE != 0)
            .unwrap_or(false)
}

#[cfg(not(windows))]
fn est_cache(nom: &str, _chemin: &Path) -> bool {
    nom.starts_with('.')
}

fn millisecondes(meta: &fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

/// Fil d'Ariane : « Accueil › Projets › appli » quand on est dans le dossier perso.
fn segments(chemin: &Path) -> Vec<Segment> {
    let perso = accueil();
    let mut liste = Vec::new();
    for ancetre in chemin.ancestors() {
        if ancetre.as_os_str().is_empty() {
            continue;
        }
        if ancetre == perso {
            liste.push(Segment { nom: "Accueil".into(), chemin: texte(ancetre) });
            break;
        }
        let nom = match ancetre.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => {
                let t = texte(ancetre);
                if t == "/" {
                    "Racine".into()
                } else {
                    t.trim_end_matches(['\\', '/']).to_string()
                }
            }
        };
        liste.push(Segment { nom, chemin: texte(ancetre) });
    }
    liste.reverse();
    liste
}

/// Vérifie qu'un nom de fichier est acceptable sur le système.
fn valider_nom(nom: &str) -> Result<(), String> {
    if nom.is_empty() || nom == "." || nom == ".." {
        return Err("Ce nom n'est pas valide.".into());
    }
    let interdits: &[char] = if cfg!(windows) {
        &['/', '\\', ':', '*', '?', '"', '<', '>', '|']
    } else {
        &['/']
    };
    if let Some(c) = nom.chars().find(|c| interdits.contains(c)) {
        return Err(format!("Le caractère « {c} » n'est pas autorisé dans un nom."));
    }
    Ok(())
}

/// Sépare « rapport.final.pdf » en (« rapport.final », « .pdf »).
fn separer(nom: &str) -> (String, String) {
    match nom.rfind('.') {
        Some(i) if i > 0 => (nom[..i].to_string(), nom[i..].to_string()),
        _ => (nom.to_string(), String::new()),
    }
}

/// Trouve un nom libre : « photo (2).jpg », ou « photo (copie).jpg » dans le même dossier.
pub(crate) fn nom_libre(dossier: &Path, nom: &str, est_dossier: bool, copie: bool) -> PathBuf {
    let libre = |p: &Path| fs::symlink_metadata(p).is_err();
    let cible = dossier.join(nom);
    if libre(&cible) {
        return cible;
    }
    let (base, ext) = if est_dossier {
        (nom.to_string(), String::new())
    } else {
        separer(nom)
    };
    for n in 1..100_000u32 {
        let essai = match (copie, n) {
            (true, 1) => format!("{base} (copie){ext}"),
            (true, n) => format!("{base} (copie {n}){ext}"),
            (false, n) => format!("{base} ({}){ext}", n + 1),
        };
        let c = dossier.join(essai);
        if libre(&c) {
            return c;
        }
    }
    cible
}

// ———————————————————————————— copie / déplacement ————————————————————————————

fn copier(src: &Path, cible: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        copier_lien(src, cible)
    } else if meta.is_dir() {
        copier_dossier(src, cible)
    } else {
        fs::copy(src, cible).map(|_| ())
    }
}

fn copier_dossier(src: &Path, cible: &Path) -> io::Result<()> {
    fs::create_dir(cible)?;
    for entree in fs::read_dir(src)? {
        let entree = entree?;
        copier(&entree.path(), &cible.join(entree.file_name()))?;
    }
    Ok(())
}

/// Sous Linux, un lien symbolique est recréé tel quel.
#[cfg(unix)]
fn copier_lien(src: &Path, cible: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(src)?, cible)
}

/// Sous Windows, on copie ce que pointe le lien.
#[cfg(not(unix))]
fn copier_lien(src: &Path, cible: &Path) -> io::Result<()> {
    if fs::metadata(src)?.is_dir() {
        copier_dossier(src, cible)
    } else {
        fs::copy(src, cible).map(|_| ())
    }
}

/// Copie, et nettoie la copie partielle en cas d'échec.
fn copier_proprement(src: &Path, cible: &Path) -> io::Result<()> {
    if let Err(e) = copier(src, cible) {
        let _ = supprimer(cible);
        return Err(e);
    }
    Ok(())
}

fn supprimer(p: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(p)?;
    if meta.is_dir() {
        fs::remove_dir_all(p)
    } else {
        fs::remove_file(p)
    }
}

/// Déplace : renommage direct si possible, sinon copie puis suppression (autre disque).
fn deplacer(src: &Path, cible: &Path) -> io::Result<()> {
    if fs::rename(src, cible).is_ok() {
        return Ok(());
    }
    copier_proprement(src, cible)?;
    supprimer(src)
}

fn existe(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok()
}

fn apercu(p: &Path) -> Option<Apercu> {
    let meta = fs::metadata(p).or_else(|_| fs::symlink_metadata(p)).ok()?;
    let est_dossier = meta.is_dir();
    Some(Apercu {
        est_dossier,
        taille: if est_dossier { None } else { Some(meta.len()) },
        nb_elements: if est_dossier {
            fs::read_dir(p).ok().map(|l| l.take(LIMITE_COMPTE).count() as u64)
        } else {
            None
        },
        modifie: millisecondes(&meta),
    })
}

/// Même chemin, aux différences d'écriture près (casse et séparateurs sous Windows).
fn meme_chemin(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        let t = |p: &Path| texte(p).replace('/', "\\").trim_end_matches('\\').to_lowercase();
        t(a) == t(b)
    } else {
        a == b
    }
}

/// Identifiants de tout ce qui est dans la corbeille.
fn ids_corbeille() -> std::collections::HashSet<String> {
    trash::os_limited::list()
        .map(|l| l.into_iter().map(|i| i.id.to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

/// Remplace `cible` par `src` : le nouvel élément est d'abord posé sous un nom provisoire,
/// et l'ancien ne part à la corbeille qu'une fois la copie terminée.
fn remplacer(src: &Path, cible: &Path, couper: bool) -> Result<(), String> {
    let nom = nom_de(cible);
    if src.starts_with(cible) {
        return Err(format!("« {nom} » ne peut pas remplacer le dossier qui le contient."));
    }
    let dossier = cible.parent().ok_or("Impossible de remplacer la racine.")?;
    let provisoire = nom_libre(dossier, &format!(".{nom}.ariane"), true, false);
    let poser = if couper { deplacer(src, &provisoire) } else { copier_proprement(src, &provisoire) };
    poser.map_err(|e| format!("« {nom} » : {}", message(&e, src)))?;
    if let Err(e) = trash::delete(cible) {
        // On remet tout comme avant
        let _ = if couper { deplacer(&provisoire, src) } else { supprimer(&provisoire) };
        return Err(format!("« {nom} » n'a pas pu être remplacé : {e}"));
    }
    fs::rename(&provisoire, cible).map_err(|e| {
        format!("« {nom} » est arrivé sous le nom « {} » : {}", nom_de(&provisoire), message(&e, cible))
    })
}

/// Le collage lui-même. `choix` : source → « remplacer » ou « ignorer » quand le nom est déjà pris
/// (par défaut, les deux sont gardés : « photo (2).jpg »).
fn coller_dans(
    sources: Vec<String>,
    dest: &Path,
    couper: bool,
    choix: &HashMap<String, String>,
) -> Result<ResultatCollage, String> {
    if !dest.is_dir() {
        return Err(format!("« {} » n'est pas un dossier.", texte(dest)));
    }
    let remplacements = choix.values().any(|c| c == "remplacer");
    let avant = if remplacements { ids_corbeille() } else { Default::default() };
    let mut operations = Vec::new();
    let mut remplaces = Vec::new();
    let mut erreurs = Vec::new();
    for s in sources {
        let src = PathBuf::from(&s);
        let nom = nom_de(&src);
        let meta = match fs::symlink_metadata(&src) {
            Ok(m) => m,
            Err(e) => {
                erreurs.push(message(&e, &src));
                continue;
            }
        };
        let est_dossier = meta.is_dir();
        if est_dossier && dest.starts_with(&src) {
            erreurs.push(format!("« {nom} » ne peut pas être placé dans lui-même."));
            continue;
        }
        let meme_dossier = src.parent() == Some(dest);
        if couper && meme_dossier {
            continue; // rien à faire
        }
        let occupe = dest.join(&nom);
        let decision = if !meme_dossier && existe(&occupe) { choix.get(&s).map(String::as_str) } else { None };
        if decision == Some("ignorer") {
            continue;
        }
        if decision == Some("remplacer") {
            match remplacer(&src, &occupe, couper) {
                Ok(()) => {
                    remplaces.push(operations.len());
                    operations.push(Operation { source: s, cible: texte(&occupe), remplace: None });
                }
                Err(e) => erreurs.push(e),
            }
            continue;
        }
        let cible = nom_libre(dest, &nom, est_dossier, meme_dossier);
        let resultat = if couper {
            deplacer(&src, &cible)
        } else {
            copier_proprement(&src, &cible)
        };
        match resultat {
            Ok(()) => operations.push(Operation { source: s, cible: texte(&cible), remplace: None }),
            Err(e) => erreurs.push(format!("« {nom} » : {}", message(&e, &src))),
        }
    }
    // Ce qui vient d'arriver dans la corbeille, pour pouvoir annuler les remplacements
    if !remplaces.is_empty() {
        let nouveaux: Vec<_> = trash::os_limited::list()
            .unwrap_or_default()
            .into_iter()
            .filter(|i| !avant.contains(&*i.id.to_string_lossy()))
            .collect();
        for &n in &remplaces {
            let op = &mut operations[n];
            op.remplace = nouveaux
                .iter()
                .find(|i| meme_chemin(&i.original_path(), Path::new(&op.cible)))
                .map(|i| i.id.to_string_lossy().into_owned());
        }
    }
    Ok(ResultatCollage { operations, erreurs })
}

/// Remet en place les éléments remplacés, d'après leur identifiant dans la corbeille.
fn restaurer(ids: &[String], erreurs: &mut Vec<String>) {
    if ids.is_empty() {
        return;
    }
    let elements: Vec<_> = match trash::os_limited::list() {
        Ok(l) => l.into_iter().filter(|i| ids.contains(&i.id.to_string_lossy().into_owned())).collect(),
        Err(e) => return erreurs.push(format!("Corbeille illisible : {e}")),
    };
    if elements.len() < ids.len() {
        erreurs.push("Certains éléments remplacés ne sont plus dans la corbeille.".into());
    }
    if let Err(e) = trash::os_limited::restore_all(elements) {
        erreurs.push(format!("Impossible de remettre les éléments remplacés : {e}"));
    }
}

// ———————————————————————————— commandes ————————————————————————————

/// Une ligne de la liste ; `None` si l'élément a disparu entre-temps.
fn decrire(chemin_e: PathBuf, nom: String) -> Option<Entree> {
    let lien = fs::symlink_metadata(&chemin_e).ok()?;
    let est_lien = lien.file_type().is_symlink();
    // On suit les liens pour savoir s'ils mènent à un dossier
    let meta = fs::metadata(&chemin_e).unwrap_or(lien);
    let est_dossier = meta.is_dir();
    let nb_elements = if est_dossier {
        fs::read_dir(&chemin_e)
            .ok()
            .map(|l| l.take(LIMITE_COMPTE).count() as u64)
    } else {
        None
    };
    let extension = if est_dossier {
        String::new()
    } else {
        chemin_e
            .extension()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    Some(Entree {
        cache: est_cache(&nom, &chemin_e),
        nom,
        chemin: texte(&chemin_e),
        est_dossier,
        est_lien,
        taille: if est_dossier { None } else { Some(meta.len()) },
        nb_elements,
        modifie: millisecondes(&meta),
        extension,
    })
}

/// Les éléments donnés, où qu'ils soient (vue d'un tag). Ceux qui n'existent plus sont omis.
#[tauri::command]
pub async fn decrire_chemins(chemins: Vec<String>) -> Result<Vec<Entree>, String> {
    en_tache(move || {
        Ok(chemins
            .into_iter()
            .filter_map(|c| {
                let p = PathBuf::from(&c);
                let nom = nom_de(&p);
                decrire(p, nom)
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn lister_dossier(chemin: String) -> Result<Contenu, String> {
    en_tache(move || {
        let dossier = developper(&chemin);
        let lecture = fs::read_dir(&dossier).map_err(|e| message(&e, &dossier))?;
        let entrees = lecture
            .flatten()
            .filter_map(|e| decrire(e.path(), e.file_name().to_string_lossy().into_owned()))
            .collect();
        Ok(Contenu {
            chemin: texte(&dossier),
            parent: dossier.parent().map(texte),
            segments: segments(&dossier),
            entrees,
        })
    })
    .await
}

#[tauri::command]
pub fn dossiers_standards() -> Standards {
    let t = |p: Option<PathBuf>| p.filter(|d| d.is_dir()).map(|d| texte(&d));
    Standards {
        accueil: texte(&accueil()),
        telechargements: t(dirs::download_dir()),
        bureau: t(dirs::desktop_dir()),
        documents: t(dirs::document_dir()),
        images: t(dirs::picture_dir()),
        musique: t(dirs::audio_dir()),
        videos: t(dirs::video_dir()),
        separateur: std::path::MAIN_SEPARATOR_STR.to_string(),
    }
}

#[tauri::command]
pub async fn creer_dossier(parent: String, nom: String) -> Result<String, String> {
    en_tache(move || {
        valider_nom(&nom)?;
        let cible = nom_libre(Path::new(&parent), &nom, true, false);
        fs::create_dir(&cible).map_err(|e| message(&e, &cible))?;
        Ok(texte(&cible))
    })
    .await
}

#[tauri::command]
pub async fn creer_fichier(parent: String, nom: String) -> Result<String, String> {
    en_tache(move || {
        valider_nom(&nom)?;
        let cible = nom_libre(Path::new(&parent), &nom, false, false);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&cible)
            .map_err(|e| message(&e, &cible))?;
        Ok(texte(&cible))
    })
    .await
}

#[tauri::command]
pub async fn renommer(chemin: String, nouveau_nom: String) -> Result<String, String> {
    en_tache(move || {
        let nom = nouveau_nom.trim().to_string();
        valider_nom(&nom)?;
        let source = PathBuf::from(&chemin);
        let parent = source.parent().ok_or("Impossible de renommer la racine.")?;
        let cible = parent.join(&nom);
        if cible == source {
            return Ok(texte(&cible));
        }
        // Sous Windows, « photo.JPG » → « photo.jpg » doit rester possible
        let seulement_la_casse = nom.to_lowercase() == nom_de(&source).to_lowercase();
        if fs::symlink_metadata(&cible).is_ok() && !seulement_la_casse {
            return Err(format!("« {nom} » existe déjà dans ce dossier."));
        }
        fs::rename(&source, &cible).map_err(|e| message(&e, &source))?;
        Ok(texte(&cible))
    })
    .await
}

#[tauri::command]
pub async fn mettre_a_la_corbeille(chemins: Vec<String>) -> Result<(), String> {
    en_tache(move || {
        trash::delete_all(&chemins)
            .map_err(|e| format!("Impossible de mettre à la corbeille : {e}"))
    })
    .await
}

/// Avant de coller : les éléments dont le nom est déjà pris dans `destination`.
#[tauri::command]
pub async fn conflits_collage(
    sources: Vec<String>,
    destination: String,
) -> Result<Vec<Conflit>, String> {
    en_tache(move || {
        let dest = PathBuf::from(&destination);
        let mut conflits = Vec::new();
        for s in sources {
            let src = PathBuf::from(&s);
            // Coller dans le même dossier fait « nom (copie) » : pas de conflit
            if src.parent() == Some(dest.as_path()) {
                continue;
            }
            let nom = nom_de(&src);
            let occupe = dest.join(&nom);
            if let (Some(nouveau), Some(existant)) = (apercu(&src), apercu(&occupe)) {
                conflits.push(Conflit { source: s, nom, nouveau, existant });
            }
        }
        Ok(conflits)
    })
    .await
}

/// Colle des éléments dans `destination`.
/// `mode` vaut « copier » ou « couper » (couper = déplacer, comme le glisser-déposer).
/// `choix` : pour chaque source en conflit, « remplacer », « ignorer » ou « garder » (les deux).
#[tauri::command]
pub async fn coller(
    sources: Vec<String>,
    destination: String,
    mode: String,
    choix: Option<HashMap<String, String>>,
) -> Result<ResultatCollage, String> {
    let couper = mode == "couper";
    en_tache(move || coller_dans(sources, Path::new(&destination), couper, &choix.unwrap_or_default())).await
}

/// Annule un collage : remet en place ce qui a été déplacé, ou met les copies à la corbeille.
/// Les éléments remplacés ressortent de la corbeille.
#[tauri::command]
pub async fn annuler_collage(operations: Vec<Operation>, mode: String) -> Result<(), String> {
    en_tache(move || {
        let mut erreurs = Vec::new();
        let mut a_restaurer = Vec::new();
        if mode == "couper" {
            for op in operations.iter().rev() {
                let (cible, source) = (Path::new(&op.cible), Path::new(&op.source));
                if fs::symlink_metadata(source).is_ok() {
                    erreurs.push(format!(
                        "« {} » existe déjà à son emplacement d'origine.",
                        nom_de(source)
                    ));
                    continue;
                }
                match deplacer(cible, source) {
                    Ok(()) => a_restaurer.extend(op.remplace.clone()),
                    Err(e) => erreurs.push(message(&e, cible)),
                }
            }
        } else {
            let cibles: Vec<&String> = operations.iter().map(|o| &o.cible).collect();
            match trash::delete_all(cibles) {
                Ok(()) => a_restaurer.extend(operations.iter().filter_map(|o| o.remplace.clone())),
                Err(e) => erreurs.push(format!("Impossible d'annuler la copie : {e}")),
            }
        }
        restaurer(&a_restaurer, &mut erreurs);
        if erreurs.is_empty() {
            Ok(())
        } else {
            Err(erreurs.join("\n"))
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_libres() {
        let d = std::env::temp_dir().join("ariane-test-noms");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("a.txt"), "x").unwrap();
        assert_eq!(nom_libre(&d, "a.txt", false, false), d.join("a (2).txt"));
        assert_eq!(nom_libre(&d, "a.txt", false, true), d.join("a (copie).txt"));
        assert_eq!(nom_libre(&d, "b.txt", false, true), d.join("b.txt"));
        fs::create_dir(d.join("v1.2")).unwrap();
        assert_eq!(nom_libre(&d, "v1.2", true, false), d.join("v1.2 (2)"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn copie_et_deplacement() {
        let d = std::env::temp_dir().join("ariane-test-copie");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src/sous")).unwrap();
        fs::write(d.join("src/sous/f.txt"), "bonjour").unwrap();
        fs::create_dir(d.join("dest")).unwrap();
        copier_proprement(&d.join("src"), &d.join("dest/src")).unwrap();
        assert_eq!(fs::read_to_string(d.join("dest/src/sous/f.txt")).unwrap(), "bonjour");
        deplacer(&d.join("src"), &d.join("dest/autre")).unwrap();
        assert!(!d.join("src").exists());
        assert!(d.join("dest/autre/sous/f.txt").exists());
        let _ = fs::remove_dir_all(&d);
    }

    fn choix(source: &Path, decision: &str) -> HashMap<String, String> {
        HashMap::from([(texte(source), decision.to_string())])
    }

    #[test]
    fn conflits_ignorer_et_garder() {
        let d = std::env::temp_dir().join("ariane-test-conflits");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src")).unwrap();
        fs::create_dir(d.join("dest")).unwrap();
        fs::write(d.join("src/a.txt"), "nouveau").unwrap();
        fs::write(d.join("dest/a.txt"), "ancien").unwrap();
        let a = d.join("src/a.txt");

        // Ignorer : rien ne bouge
        let r = coller_dans(vec![texte(&a)], &d.join("dest"), true, &choix(&a, "ignorer")).unwrap();
        assert!(r.operations.is_empty() && r.erreurs.is_empty());
        assert!(a.exists());
        assert_eq!(fs::read_to_string(d.join("dest/a.txt")).unwrap(), "ancien");

        // Sans choix : on garde les deux
        let r = coller_dans(vec![texte(&a)], &d.join("dest"), false, &HashMap::new()).unwrap();
        assert_eq!(r.operations[0].cible, texte(&d.join("dest").join("a (2).txt")));
        assert_eq!(fs::read_to_string(d.join("dest/a.txt")).unwrap(), "ancien");
        let _ = fs::remove_dir_all(&d);
    }

    /// Passe par la vraie corbeille : `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn remplacer_puis_annuler() {
        let d = std::env::temp_dir().join("ariane-test-remplacer");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src")).unwrap();
        fs::create_dir(d.join("dest")).unwrap();
        fs::write(d.join("src/a.txt"), "nouveau").unwrap();
        fs::write(d.join("dest/a.txt"), "ancien").unwrap();
        let a = d.join("src/a.txt");

        let r = coller_dans(vec![texte(&a)], &d.join("dest"), true, &choix(&a, "remplacer")).unwrap();
        assert!(r.erreurs.is_empty(), "{:?}", r.erreurs);
        assert_eq!(fs::read_to_string(d.join("dest/a.txt")).unwrap(), "nouveau");
        assert!(!a.exists());
        assert!(r.operations[0].remplace.is_some(), "l'ancien doit être retrouvé dans la corbeille");
        assert_eq!(fs::read_dir(d.join("dest")).unwrap().count(), 1, "pas de fichier provisoire oublié");

        tauri::async_runtime::block_on(annuler_collage(r.operations, "couper".into())).unwrap();
        assert_eq!(fs::read_to_string(&a).unwrap(), "nouveau");
        assert_eq!(fs::read_to_string(d.join("dest/a.txt")).unwrap(), "ancien");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn remplacer_refuse_son_propre_parent() {
        let d = std::env::temp_dir().join("ariane-test-parent");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("b/b")).unwrap();
        let interieur = d.join("b/b");
        let r = coller_dans(vec![texte(&interieur)], &d, true, &choix(&interieur, "remplacer")).unwrap();
        assert_eq!(r.erreurs.len(), 1);
        assert!(interieur.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[cfg(windows)]
    #[test]
    fn fil_d_un_dossier_reseau() {
        let noms: Vec<String> = segments(&developper("//srv/data/Bureaux_etudes")).into_iter().map(|s| s.nom).collect();
        assert_eq!(noms, [r"\\srv\data", "Bureaux_etudes"]);
    }

    #[test]
    fn decrire_des_chemins_epars() {
        let d = std::env::temp_dir().join("ariane-test-decrire");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("a/sous")).unwrap();
        fs::write(d.join("a/rapport.pdf"), "12345").unwrap();
        let chemins = vec![texte(&d.join("a/rapport.pdf")), texte(&d.join("a/sous")), texte(&d.join("disparu.txt"))];
        let r = tauri::async_runtime::block_on(decrire_chemins(chemins)).unwrap();
        assert_eq!(r.len(), 2, "l'élément disparu est omis");
        assert_eq!((r[0].nom.as_str(), r[0].taille, r[0].extension.as_str()), ("rapport.pdf", Some(5), "pdf"));
        assert!(r[1].est_dossier && r[1].nb_elements == Some(0));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn noms_invalides() {
        assert!(valider_nom("").is_err());
        assert!(valider_nom("..").is_err());
        assert!(valider_nom("a/b").is_err());
        assert!(valider_nom("rapport final.pdf").is_ok());
    }
}
