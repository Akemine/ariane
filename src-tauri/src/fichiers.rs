//! Opérations sur les fichiers : lister, créer, renommer, copier, déplacer, corbeille.
//!
//! Tout le travail disque se fait hors du fil principal (`spawn_blocking`)
//! pour que l'interface reste fluide pendant une grosse copie.

use serde::{Deserialize, Serialize};
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
}

#[derive(Serialize)]
pub struct ResultatCollage {
    operations: Vec<Operation>,
    erreurs: Vec<String>,
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
    PathBuf::from(chemin)
}

/// Message d'erreur lisible, en français.
pub(crate) fn message(e: &io::Error, chemin: &Path) -> String {
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

// ———————————————————————————— commandes ————————————————————————————

#[tauri::command]
pub async fn lister_dossier(chemin: String) -> Result<Contenu, String> {
    en_tache(move || {
        let dossier = developper(&chemin);
        let lecture = fs::read_dir(&dossier).map_err(|e| message(&e, &dossier))?;
        let mut entrees = Vec::new();
        for entree in lecture.flatten() {
            let chemin_e = entree.path();
            let nom = entree.file_name().to_string_lossy().into_owned();
            let est_lien = entree.file_type().map(|t| t.is_symlink()).unwrap_or(false);
            // On suit les liens pour savoir s'ils mènent à un dossier
            let meta = match fs::metadata(&chemin_e).or_else(|_| entree.metadata()) {
                Ok(m) => m,
                Err(_) => continue,
            };
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
            entrees.push(Entree {
                cache: est_cache(&nom, &chemin_e),
                nom,
                chemin: texte(&chemin_e),
                est_dossier,
                est_lien,
                taille: if est_dossier { None } else { Some(meta.len()) },
                nb_elements,
                modifie: millisecondes(&meta),
                extension,
            });
        }
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

/// Colle des éléments dans `destination`.
/// `mode` vaut « copier » ou « couper » (couper = déplacer, comme le glisser-déposer).
#[tauri::command]
pub async fn coller(
    sources: Vec<String>,
    destination: String,
    mode: String,
) -> Result<ResultatCollage, String> {
    let couper = mode == "couper";
    en_tache(move || {
        let dest = PathBuf::from(&destination);
        if !dest.is_dir() {
            return Err(format!("« {} » n'est pas un dossier.", texte(&dest)));
        }
        let mut operations = Vec::new();
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
            let meme_dossier = src.parent() == Some(dest.as_path());
            if couper && meme_dossier {
                continue; // rien à faire
            }
            let cible = nom_libre(&dest, &nom, est_dossier, meme_dossier);
            let resultat = if couper {
                deplacer(&src, &cible)
            } else {
                copier_proprement(&src, &cible)
            };
            match resultat {
                Ok(()) => operations.push(Operation { source: s, cible: texte(&cible) }),
                Err(e) => erreurs.push(format!("« {nom} » : {}", message(&e, &src))),
            }
        }
        Ok(ResultatCollage { operations, erreurs })
    })
    .await
}

/// Annule un collage : remet en place ce qui a été déplacé, ou met les copies à la corbeille.
#[tauri::command]
pub async fn annuler_collage(operations: Vec<Operation>, mode: String) -> Result<(), String> {
    en_tache(move || {
        let mut erreurs = Vec::new();
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
                if let Err(e) = deplacer(cible, source) {
                    erreurs.push(message(&e, cible));
                }
            }
        } else {
            let cibles: Vec<&String> = operations.iter().map(|o| &o.cible).collect();
            if let Err(e) = trash::delete_all(cibles) {
                erreurs.push(format!("Impossible d'annuler la copie : {e}"));
            }
        }
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

    #[test]
    fn noms_invalides() {
        assert!(valider_nom("").is_err());
        assert!(valider_nom("..").is_err());
        assert!(valider_nom("a/b").is_err());
        assert!(valider_nom("rapport final.pdf").is_ok());
    }
}
