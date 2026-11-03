//! Intégration avec le système : ouvrir un fichier, un terminal, VS Code.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Sous Windows, empêche une console noire de clignoter pour les commandes en arrière-plan.
pub fn sans_fenetre(commande: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = commande;
}

/// Lance un programme détaché, dans le dossier donné.
fn lancer(programme: &str, args: &[&str], dossier: &Path, cacher: bool) -> io::Result<()> {
    let mut commande = Command::new(programme);
    commande
        .args(args)
        .current_dir(dossier)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if cacher {
        sans_fenetre(&mut commande);
    }
    let mut enfant = commande.spawn()?;
    // On récupère le processus quand il se termine, pour ne pas laisser de zombie
    std::thread::spawn(move || {
        let _ = enfant.wait();
    });
    Ok(())
}

/// Essaie chaque programme dans l'ordre ; passe au suivant s'il n'est pas installé.
fn premier_disponible(candidats: &[(&str, Vec<&str>)], dossier: &Path, cacher: bool) -> Result<bool, String> {
    for (programme, args) in candidats {
        match lancer(programme, args, dossier, cacher) {
            Ok(()) => return Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("Impossible de lancer {programme} : {e}")),
        }
    }
    Ok(false)
}

fn dossier_de(chemin: &str) -> &Path {
    let p = Path::new(chemin);
    if p.is_dir() {
        p
    } else {
        p.parent().unwrap_or(p)
    }
}

#[tauri::command]
pub fn ouvrir(chemin: String) -> Result<(), String> {
    open::that_detached(&chemin).map_err(|e| format!("Impossible d'ouvrir « {chemin} » : {e}"))
}

#[tauri::command]
pub fn ouvrir_terminal(chemin: String) -> Result<(), String> {
    let dossier = dossier_de(&chemin);
    let d = dossier.to_str().unwrap_or(".");

    #[cfg(windows)]
    let candidats: Vec<(&str, Vec<&str>)> = vec![
        ("wt", vec!["-d", d]),
        ("cmd", vec!["/C", "start", "", "powershell", "-NoExit"]),
    ];

    #[cfg(not(windows))]
    let terminal_perso = std::env::var("TERMINAL").ok();
    #[cfg(not(windows))]
    let mut candidats: Vec<(&str, Vec<&str>)> = Vec::new();
    #[cfg(not(windows))]
    {
        if let Some(t) = terminal_perso.as_deref() {
            candidats.push((t, vec![]));
        }
        candidats.extend([
            ("ptyxis", vec!["--new-window", "--working-directory", d]), // Fedora 41+
            ("gnome-terminal", vec!["--working-directory", d]),
            ("kgx", vec!["--working-directory", d]),
            ("konsole", vec!["--workdir", d]),
            ("xfce4-terminal", vec!["--working-directory", d]),
            ("kitty", vec!["--directory", d]),
            ("alacritty", vec!["--working-directory", d]),
            ("wezterm", vec!["start", "--cwd", d]),
            ("xterm", vec![]),
        ]);
    }

    if premier_disponible(&candidats, dossier, false)? {
        Ok(())
    } else {
        Err("Aucun terminal trouvé. Installez-en un (par exemple Ptyxis) ou définissez la variable TERMINAL.".into())
    }
}

#[tauri::command]
pub fn ouvrir_vscode(chemin: String) -> Result<(), String> {
    let dossier = dossier_de(&chemin);
    let c = chemin.as_str();

    // Sous Windows, « code » est un script .cmd : il faut passer par cmd
    #[cfg(windows)]
    let candidats: Vec<(&str, Vec<&str>)> = vec![("cmd", vec!["/C", "code", c])];

    #[cfg(not(windows))]
    let candidats: Vec<(&str, Vec<&str>)> = vec![
        ("code", vec![c]),
        ("codium", vec![c]),
        ("code-oss", vec![c]),
        ("flatpak", vec!["run", "com.visualstudio.code", c]),
    ];

    if premier_disponible(&candidats, dossier, true)? {
        Ok(())
    } else {
        Err("VS Code est introuvable (commande « code »).".into())
    }
}

/// Dossier passé au lancement (« ariane ~/Projets » ou depuis un autre programme).
#[tauri::command]
pub fn chemin_initial() -> Option<String> {
    let argument = std::env::args().skip(1).find(|a| !a.starts_with('-'))?;
    let chemin = match argument.strip_prefix("file://") {
        Some(reste) => decoder_uri(reste),
        None => argument,
    };
    let p = Path::new(&chemin);
    if p.is_dir() {
        Some(chemin)
    } else {
        p.parent().filter(|d| d.is_dir()).map(|d| d.to_string_lossy().into_owned())
    }
}

/// Décode les « %20 » d'une adresse file://.
pub(crate) fn decoder_uri(s: &str) -> String {
    let octets = s.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while i < octets.len() {
        if octets[i] == b'%' && i + 2 < octets.len() {
            let hexa = std::str::from_utf8(&octets[i + 1..i + 3]).ok();
            if let Some(v) = hexa.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                sortie.push(v);
                i += 3;
                continue;
            }
        }
        sortie.push(octets[i]);
        i += 1;
    }
    String::from_utf8_lossy(&sortie).into_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode() {
        assert_eq!(super::decoder_uri("/home/st%C3%A9phane/Mes%20docs"), "/home/stéphane/Mes docs");
    }
}
