//! État Git du dossier affiché : branche, commits à pousser, statut de chaque élément.
//!
//! On passe par la commande `git` installée sur la machine : pas de bibliothèque
//! à compiler, et le comportement est exactement celui de Git (y compris .gitignore).

use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Default)]
pub struct EtatGit {
    depot: bool,
    branche: Option<String>,
    en_avance: u32,
    en_retard: u32,
    /// nom de l'élément → « modifie » | « nouveau » | « ignore » | « conflit »
    statuts: HashMap<String, String>,
}

fn git(dossier: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let mut commande = Command::new("git");
    commande.arg("-C").arg(dossier).args(args);
    crate::systeme::sans_fenetre(&mut commande);
    let sortie = commande.output().ok()?;
    if !sortie.status.success() {
        return None;
    }
    Some(sortie.stdout)
}

fn ligne(octets: &[u8]) -> String {
    String::from_utf8_lossy(octets).trim().to_string()
}

fn classer(x: u8, y: u8) -> &'static str {
    match (x, y) {
        (b'?', b'?') => "nouveau",
        (b'!', b'!') => "ignore",
        (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => "conflit",
        (b'A', _) => "nouveau",
        _ => "modifie",
    }
}

fn priorite(statut: &str) -> u8 {
    match statut {
        "conflit" => 4,
        "modifie" => 3,
        "nouveau" => 2,
        "ignore" => 1,
        _ => 0,
    }
}

#[tauri::command]
pub async fn etat_git(chemin: String) -> EtatGit {
    tauri::async_runtime::spawn_blocking(move || calculer(Path::new(&chemin)))
        .await
        .unwrap_or_default()
}

fn calculer(dossier: &Path) -> EtatGit {
    // Le dossier fait-il partie d'un dépôt ? (faux aussi à l'intérieur de .git)
    match git(dossier, &["rev-parse", "--is-inside-work-tree"]) {
        Some(s) if ligne(&s) == "true" => {}
        _ => return EtatGit::default(),
    }
    let mut etat = EtatGit { depot: true, ..Default::default() };

    etat.branche = git(dossier, &["rev-parse", "--abbrev-ref", "HEAD"]).map(|s| ligne(&s));

    // Commits en retard / en avance sur la branche distante suivie
    if let Some(c) = git(dossier, &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"]) {
        let t = ligne(&c);
        let mut nombres = t.split_whitespace().map(|n| n.parse().unwrap_or(0));
        etat.en_retard = nombres.next().unwrap_or(0);
        etat.en_avance = nombres.next().unwrap_or(0);
    }

    // Chemin du dossier courant par rapport à la racine du dépôt (« src/composants/ »)
    let prefixe = git(dossier, &["rev-parse", "--show-prefix"])
        .map(|s| ligne(&s))
        .unwrap_or_default();

    let Some(sortie) = git(
        dossier,
        &["status", "--porcelain=v1", "-z", "--ignored=matching", "--untracked-files=normal", "--", "."],
    ) else {
        return etat;
    };

    // Format : « XY chemin\0 », et pour un renommage un second chemin à sauter
    let morceaux: Vec<&[u8]> = sortie.split(|o| *o == 0).collect();
    let mut i = 0;
    while i < morceaux.len() {
        let m = morceaux[i];
        i += 1;
        if m.len() < 4 {
            continue;
        }
        let (x, y) = (m[0], m[1]);
        if x == b'R' || x == b'C' {
            i += 1;
        }
        let statut = classer(x, y);
        let chemin = String::from_utf8_lossy(&m[3..]).to_string();
        let Some(relatif) = chemin.strip_prefix(prefixe.as_str()) else {
            continue;
        };
        let (premier, reste) = match relatif.split_once('/') {
            Some((a, b)) => (a, b),
            None => (relatif, ""),
        };
        if premier.is_empty() {
            continue;
        }
        // Un fichier ignoré à l'intérieur d'un dossier suivi ne rend pas le dossier « ignoré »
        if statut == "ignore" && !reste.is_empty() {
            continue;
        }
        let actuel = etat.statuts.get(premier).map(|s| priorite(s)).unwrap_or(0);
        if priorite(statut) > actuel {
            etat.statuts.insert(premier.to_string(), statut.to_string());
        }
    }
    etat
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn lancer(d: &Path, args: &[&str]) {
        let ok = Command::new("git").arg("-C").arg(d).args(args).output().map(|s| s.status.success());
        assert!(matches!(ok, Ok(true)), "git {args:?} a échoué");
    }

    #[test]
    fn statuts_du_depot() {
        let d = std::env::temp_dir().join("ariane-test-git");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src")).unwrap();
        fs::create_dir_all(d.join("node_modules/paquet")).unwrap();
        fs::write(d.join("src/a.js"), "1").unwrap();
        fs::write(d.join("lisez-moi.md"), "1").unwrap();
        fs::write(d.join(".gitignore"), "node_modules/\n*.log\n").unwrap();
        fs::write(d.join("node_modules/paquet/x.js"), "1").unwrap();
        lancer(&d, &["init", "-q", "-b", "main"]);
        lancer(&d, &["add", "."]);
        lancer(&d, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "init"]);
        fs::write(d.join("src/a.js"), "2").unwrap();
        fs::write(d.join("src/debug.log"), "x").unwrap();
        fs::write(d.join("nouveau.txt"), "x").unwrap();

        let e = calculer(&d);
        assert!(e.depot);
        assert_eq!(e.branche.as_deref(), Some("main"));
        assert_eq!(e.statuts.get("src").map(String::as_str), Some("modifie"));
        assert_eq!(e.statuts.get("nouveau.txt").map(String::as_str), Some("nouveau"));
        assert_eq!(e.statuts.get("node_modules").map(String::as_str), Some("ignore"));
        assert_eq!(e.statuts.get("lisez-moi.md"), None); // à jour

        // Depuis un sous-dossier : les noms sont relatifs à ce sous-dossier
        let s = calculer(&d.join("src"));
        assert_eq!(s.statuts.get("a.js").map(String::as_str), Some("modifie"));
        assert_eq!(s.statuts.get("debug.log").map(String::as_str), Some("ignore"));

        // Hors dépôt
        assert!(!calculer(&std::env::temp_dir()).depot);
        let _ = fs::remove_dir_all(&d);
    }
}
