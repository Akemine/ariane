//! Actualisation automatique : Ariane suit les dossiers affichés (et seulement eux, sans leurs
//! sous-dossiers) et prévient l'interface quand un élément y apparaît, change ou disparaît,
//! même si c'est une autre application qui l'a fait.

use crate::fichiers::{en_tache, meme_chemin};
use notify::event::{AccessKind, AccessMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

pub struct Surveillance {
    veilleur: RecommendedWatcher,
    suivis: Arc<Mutex<Vec<PathBuf>>>,
}

/// Ouvrir ou lire un fichier ne change rien à la liste ; finir de l'écrire, si (taille, date).
fn compte(genre: &EventKind) -> bool {
    match genre {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        EventKind::Access(_) => false,
        _ => true,
    }
}

impl Surveillance {
    pub fn new(signaler: impl Fn(String) + Send + 'static) -> notify::Result<Self> {
        let suivis = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let liste = suivis.clone();
        let veilleur = notify::recommended_watcher(move |r: notify::Result<Event>| {
            let Ok(ev) = r else { return };
            if !compte(&ev.kind) {
                return;
            }
            let mut touches: Vec<PathBuf> = Vec::new();
            for dossier in liste.lock().unwrap().iter() {
                let concerne = ev.paths.iter().any(|p| {
                    // git status réécrit .git à chaque actualisation : sans ça, la liste se rechargerait en boucle
                    let git = p.file_name().is_some_and(|n| n == ".git");
                    !git && (p.parent().is_some_and(|parent| meme_chemin(parent, dossier)) || meme_chemin(p, dossier))
                });
                if concerne {
                    touches.push(dossier.clone());
                }
            }
            for dossier in touches {
                signaler(dossier.to_string_lossy().into_owned());
            }
        })?;
        Ok(Self { veilleur, suivis })
    }

    /// Suit exactement ces dossiers. Un dossier impossible à suivre (disparu, réseau…) est ignoré :
    /// F5 et le retour dans la fenêtre l'actualisent toujours.
    pub fn suivre(&mut self, dossiers: Vec<PathBuf>) {
        // watch / unwatch hors du verrou : la fonction ci-dessus le prend depuis le fil de notify
        let avant = self.suivis.lock().unwrap().clone();
        for d in avant.iter().filter(|d| !dossiers.contains(d)) {
            let _ = self.veilleur.unwatch(d);
        }
        let mut gardes: Vec<PathBuf> = avant.into_iter().filter(|d| dossiers.contains(d)).collect();
        for d in dossiers {
            if !gardes.contains(&d) && self.veilleur.watch(&d, RecursiveMode::NonRecursive).is_ok() {
                gardes.push(d);
            }
        }
        *self.suivis.lock().unwrap() = gardes;
    }
}

static SURVEILLANCE: Mutex<Option<Surveillance>> = Mutex::new(None);

/// Les dossiers affichés dans les panneaux ; « dossier-modifie » est émis quand l'un d'eux change.
#[tauri::command]
pub async fn surveiller(app: tauri::AppHandle, dossiers: Vec<String>) -> Result<(), String> {
    en_tache(move || {
        let mut s = SURVEILLANCE.lock().unwrap();
        if s.is_none() {
            let surveillance = Surveillance::new(move |d| {
                let _ = app.emit("dossier-modifie", d);
            })
            .map_err(|e| e.to_string())?;
            *s = Some(surveillance);
        }
        if let Some(s) = s.as_mut() {
            s.suivre(dossiers.into_iter().map(PathBuf::from).collect());
        }
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn previent_quand_un_dossier_change() {
        let dossier = std::env::temp_dir().join(format!("ariane-surveillance-{}", std::process::id()));
        let autre = dossier.join("sous-dossier");
        std::fs::create_dir_all(&autre).unwrap();
        let (envoi, reception) = mpsc::channel();
        let mut s = Surveillance::new(move |d| envoi.send(d).unwrap()).unwrap();
        s.suivre(vec![dossier.clone()]);

        // .git et le contenu des sous-dossiers ne comptent pas
        std::fs::create_dir(dossier.join(".git")).unwrap();
        std::fs::write(autre.join("dedans.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        let parasites: Vec<String> = reception.try_iter().collect();

        std::fs::write(dossier.join("nouveau.txt"), "bonjour").unwrap();
        let recu = reception.recv_timeout(Duration::from_secs(5)).expect("aucun signal");
        assert_eq!(recu, dossier.to_string_lossy());

        // Plus suivi : plus de signal
        s.suivre(Vec::new());
        std::thread::sleep(Duration::from_millis(300));
        reception.try_iter().for_each(drop);
        std::fs::write(dossier.join("encore.txt"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(reception.try_iter().count(), 0);
        std::fs::remove_dir_all(&dossier).ok();
        // Sous Windows, créer « sous-dossier/dedans.txt » change la date de « sous-dossier » lui-même :
        // un signal est alors normal (la colonne du nombre d'éléments bouge), pas pour .git.
        assert!(parasites.len() <= 1, "{parasites:?}");
    }
}
