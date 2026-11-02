//! Actions proposées selon le type d'élément : extraire une archive, compresser,
//! dupliquer, calculer la taille d'un dossier, copier le contenu d'un texte,
//! ouvrir le dépôt Git dans le navigateur, exécuter en administrateur (Windows).

use crate::fichiers::{en_tache, message, nom_libre};
use serde::Serialize;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

fn texte(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// « photos.tar.gz » → « photos »
fn nom_sans_extension_archive(nom: &str) -> String {
    let bas = nom.to_lowercase();
    for ext in [".tar.gz", ".tgz", ".tar", ".zip"] {
        if bas.ends_with(ext) {
            return nom[..nom.len() - ext.len()].to_string();
        }
    }
    nom.to_string()
}

fn deballer(archive: &Path, vers: &Path) -> io::Result<()> {
    let bas = archive.to_string_lossy().to_lowercase();
    let fichier = File::open(archive)?;
    if bas.ends_with(".zip") {
        // extract() refuse les chemins qui sortiraient du dossier de destination
        zip::ZipArchive::new(fichier).and_then(|mut z| z.extract(vers)).map_err(io::Error::other)
    } else if bas.ends_with(".tar.gz") || bas.ends_with(".tgz") {
        tar::Archive::new(flate2::read::GzDecoder::new(fichier)).unpack(vers)
    } else if bas.ends_with(".tar") {
        tar::Archive::new(fichier).unpack(vers)
    } else {
        Err(io::Error::other("Format d'archive non pris en charge (zip, tar, tar.gz)."))
    }
}

/// Extrait une archive. `mode` : « dossier » (dans un nouveau dossier au nom de l'archive)
/// ou « ici » (dans le dossier de l'archive). Rien n'est jamais écrasé : un nom déjà pris
/// devient « nom (2) ». Renvoie les éléments créés.
#[tauri::command]
pub async fn extraire_archive(chemin: String, mode: String) -> Result<Vec<String>, String> {
    en_tache(move || {
        let archive = PathBuf::from(&chemin);
        let parent = archive.parent().ok_or("Dossier de l'archive introuvable.")?.to_path_buf();
        let nom = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let base = nom_sans_extension_archive(&nom);

        if mode == "dossier" {
            let cible = nom_libre(&parent, &base, true, false);
            fs::create_dir(&cible).map_err(|e| message(&e, &cible))?;
            if let Err(e) = deballer(&archive, &cible) {
                let _ = fs::remove_dir_all(&cible);
                return Err(format!("Extraction impossible : {e}"));
            }
            return Ok(vec![texte(&cible)]);
        }

        // « ici » : extraction à part, puis chaque élément est rangé à côté de l'archive
        let temporaire = nom_libre(&parent, &format!(".extraction-{base}"), true, false);
        fs::create_dir(&temporaire).map_err(|e| message(&e, &temporaire))?;
        let resultat = (|| -> Result<Vec<String>, String> {
            deballer(&archive, &temporaire).map_err(|e| format!("Extraction impossible : {e}"))?;
            let mut crees = Vec::new();
            for entree in fs::read_dir(&temporaire).map_err(|e| e.to_string())?.flatten() {
                let nom = entree.file_name().to_string_lossy().into_owned();
                let est_dossier = entree.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let cible = nom_libre(&parent, &nom, est_dossier, false);
                fs::rename(entree.path(), &cible).map_err(|e| message(&e, &cible))?;
                crees.push(texte(&cible));
            }
            Ok(crees)
        })();
        let _ = fs::remove_dir_all(&temporaire);
        resultat
    })
    .await
}

fn ajouter(zip: &mut zip::ZipWriter<File>, chemin: &Path, nom: &str) -> io::Result<()> {
    let meta = fs::symlink_metadata(chemin)?;
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(meta.len() >= u32::MAX as u64);
    if meta.is_dir() {
        zip.add_directory(format!("{nom}/"), options).map_err(io::Error::other)?;
        for entree in fs::read_dir(chemin)? {
            let entree = entree?;
            ajouter(zip, &entree.path(), &format!("{nom}/{}", entree.file_name().to_string_lossy()))?;
        }
    } else if meta.is_file() {
        zip.start_file(nom, options).map_err(io::Error::other)?;
        io::copy(&mut File::open(chemin)?, zip)?;
    }
    Ok(())
}

/// Compresse les éléments en un .zip dans `destination`. Renvoie le chemin de l'archive.
#[tauri::command]
pub async fn compresser(chemins: Vec<String>, destination: String) -> Result<String, String> {
    en_tache(move || {
        let dest = PathBuf::from(&destination);
        let nom = match chemins.as_slice() {
            [un] => {
                let p = Path::new(un);
                let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Archive".into());
                // « rapport.pdf » → « rapport.zip », un dossier garde son nom
                if p.is_dir() { n } else { n.rsplit_once('.').map(|(a, _)| a.to_string()).filter(|a| !a.is_empty()).unwrap_or(n) }
            }
            _ => "Archive".into(),
        };
        let cible = nom_libre(&dest, &format!("{nom}.zip"), false, false);
        let resultat = (|| -> io::Result<()> {
            let mut zip = zip::ZipWriter::new(File::create(&cible)?);
            for c in &chemins {
                let p = Path::new(c);
                let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                ajouter(&mut zip, p, &n)?;
            }
            zip.finish().map_err(io::Error::other)?;
            Ok(())
        })();
        if let Err(e) = resultat {
            let _ = fs::remove_file(&cible);
            return Err(format!("Compression impossible : {e}"));
        }
        Ok(texte(&cible))
    })
    .await
}

#[derive(Serialize, Default)]
pub struct Taille {
    octets: u64,
    fichiers: u64,
    dossiers: u64,
}

fn mesurer(chemin: &Path, total: &mut Taille) {
    let Ok(lecture) = fs::read_dir(chemin) else { return };
    for entree in lecture.flatten() {
        let Ok(genre) = entree.file_type() else { continue };
        if genre.is_dir() {
            total.dossiers += 1;
            mesurer(&entree.path(), total);
        } else if genre.is_file() {
            total.fichiers += 1;
            total.octets += entree.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
}

/// Taille totale d'un dossier, sous-dossiers compris (les liens ne sont pas suivis).
#[tauri::command]
pub async fn taille_dossier(chemin: String) -> Result<Taille, String> {
    en_tache(move || {
        let mut total = Taille::default();
        mesurer(Path::new(&chemin), &mut total);
        Ok(total)
    })
    .await
}

/// Contenu d'un fichier texte (2 Mo au plus), pour le copier dans le presse-papiers.
#[tauri::command]
pub async fn lire_texte(chemin: String) -> Result<String, String> {
    en_tache(move || {
        let p = PathBuf::from(&chemin);
        let taille = fs::metadata(&p).map_err(|e| message(&e, &p))?.len();
        if taille > 2 * 1024 * 1024 {
            return Err("Fichier trop gros pour être copié (plus de 2 Mo).".into());
        }
        let octets = fs::read(&p).map_err(|e| message(&e, &p))?;
        String::from_utf8(octets).map_err(|_| "Ce fichier ne contient pas du texte.".into())
    })
    .await
}

/// Adresse web du dépôt (« git@github.com:moi/projet.git » → « https://github.com/moi/projet »).
fn adresse_web(remote: &str) -> Option<String> {
    let r = remote.trim().trim_end_matches('/').trim_end_matches(".git");
    if let Some(reste) = r.strip_prefix("git@") {
        let (hote, chemin) = reste.split_once(':')?;
        return Some(format!("https://{hote}/{chemin}"));
    }
    if r.starts_with("https://") || r.starts_with("http://") {
        // Retire un éventuel « utilisateur@ » de l'adresse
        let (schema, reste) = r.split_once("://")?;
        let reste = reste.split_once('@').map(|(_, apres)| apres).unwrap_or(reste);
        return Some(format!("{schema}://{reste}"));
    }
    None
}

#[tauri::command]
pub async fn adresse_depot(chemin: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut c = std::process::Command::new("git");
        c.args(["-C", &chemin, "remote", "get-url", "origin"]);
        crate::systeme::sans_fenetre(&mut c);
        let sortie = c.output().ok()?;
        if !sortie.status.success() {
            return None;
        }
        adresse_web(&String::from_utf8_lossy(&sortie.stdout))
    })
    .await
    .ok()
    .flatten()
}

/// Windows : lance un programme en tant qu'administrateur (Windows demande la confirmation).
#[tauri::command]
pub fn executer_admin(chemin: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let commande = format!("Start-Process -FilePath '{}' -Verb RunAs", chemin.replace('\'', "''"));
        let mut c = std::process::Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-Command", &commande]);
        crate::systeme::sans_fenetre(&mut c);
        c.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = chemin;
        Err("Disponible sous Windows uniquement.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_et_adresses() {
        assert_eq!(nom_sans_extension_archive("photos.tar.gz"), "photos");
        assert_eq!(nom_sans_extension_archive("Rapport.ZIP"), "Rapport");
        assert_eq!(adresse_web("git@github.com:moi/projet.git").as_deref(), Some("https://github.com/moi/projet"));
        assert_eq!(adresse_web("https://jeton@github.com/moi/projet.git\n").as_deref(), Some("https://github.com/moi/projet"));
        assert_eq!(adresse_web("/chemin/local"), None);
    }

    #[test]
    fn compresser_puis_extraire() {
        let d = std::env::temp_dir().join("ariane-test-archives");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("projet/src")).unwrap();
        fs::write(d.join("projet/src/a.txt"), "bonjour").unwrap();
        fs::write(d.join("notes.md"), "# notes").unwrap();
        tauri::async_runtime::block_on(async {
            let zip = compresser(vec![texte(&d.join("projet")), texte(&d.join("notes.md"))], texte(&d)).await.unwrap();
            assert!(zip.ends_with("Archive.zip"));
            // Dans un nouveau dossier
            let crees = extraire_archive(zip.clone(), "dossier".into()).await.unwrap();
            assert_eq!(fs::read_to_string(Path::new(&crees[0]).join("projet/src/a.txt")).unwrap(), "bonjour");
            // Ici : rien n'est écrasé, « projet » existe déjà → « projet (2) »
            let crees = extraire_archive(zip, "ici".into()).await.unwrap();
            assert!(crees.iter().any(|c| c.ends_with("projet (2)")));
            assert!(crees.iter().any(|c| c.ends_with("notes (2).md")));
            let t = taille_dossier(texte(&d.join("projet"))).await.unwrap();
            assert_eq!((t.fichiers, t.dossiers, t.octets), (1, 1, 7));
        });
        let _ = fs::remove_dir_all(&d);
    }
}
