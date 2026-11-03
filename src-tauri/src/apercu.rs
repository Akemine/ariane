//! Aperçu rapide (Espace) : le contenu des fichiers pour l'interface.
//!
//! Images, vidéos, sons et PDF passent par l'adresse « apercu:// » (servie ici, par morceaux
//! pour qu'une vidéo démarre tout de suite et se parcoure). Les textes sont lus par
//! `lire_debut`, limité au début du fichier, et les PDF pour pdf.js par `lire_fichier`.

use crate::fichiers::{en_tache, message};
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tauri::http::{header, Request, Response, StatusCode};

/// Morceau envoyé quand la demande ne dit pas où s'arrêter (vidéos, sons).
const MORCEAU: u64 = 4 * 1024 * 1024;
/// Au-delà, un fichier entier n'est pas envoyé d'un coup.
const ENTIER_MAX: u64 = 256 * 1024 * 1024;
/// Début d'un texte affiché dans l'aperçu.
const TEXTE_MAX: usize = 256 * 1024;
/// Au-delà, pdf.js mettrait trop longtemps : on propose d'ouvrir le PDF.
const PDF_MAX: u64 = 100 * 1024 * 1024;

fn type_mime(chemin: &Path) -> &'static str {
    let ext = chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" | "aac" => "audio/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// « bytes=100-199 », « bytes=100- » ou « bytes=-500 » → (début, fin incluse).
fn plage(entete: &str, total: u64) -> Option<(u64, u64)> {
    if total == 0 {
        return None;
    }
    let (debut, fin) = entete.trim().strip_prefix("bytes=")?.split(',').next()?.split_once('-')?;
    let (debut, fin) = match (debut.trim(), fin.trim()) {
        ("", n) => {
            let n: u64 = n.parse().ok()?;
            (total.saturating_sub(n), total - 1)
        }
        (d, "") => {
            let d: u64 = d.parse().ok()?;
            (d, (d + MORCEAU - 1).min(total - 1))
        }
        (d, f) => (d.parse().ok()?, f.parse::<u64>().ok()?.min(total - 1)),
    };
    (debut <= fin).then_some((debut, fin))
}

/// Chemin demandé : « apercu://localhost/%2Fhome%2F… » (Linux) ou « http://apercu.localhost/C%3A%5C… » (Windows).
fn chemin_demande(requete: &Request<Vec<u8>>) -> PathBuf {
    let brut = requete.uri().path().trim_start_matches('/');
    PathBuf::from(crate::systeme::decoder_uri(brut))
}

fn reponse(statut: StatusCode, corps: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder().status(statut).body(corps).unwrap_or_default()
}

pub fn repondre(requete: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let chemin = chemin_demande(requete);
    let lu = (|| -> std::io::Result<Response<Vec<u8>>> {
        let mut fichier = File::open(&chemin)?;
        let meta = fichier.metadata()?;
        if meta.is_dir() {
            return Ok(reponse(StatusCode::NOT_FOUND, Vec::new()));
        }
        let total = meta.len();
        let demande = requete.headers().get(header::RANGE).and_then(|v| v.to_str().ok());
        let (statut, debut, fin) = match demande.and_then(|d| plage(d, total)) {
            Some((d, f)) => (StatusCode::PARTIAL_CONTENT, d, f),
            None if total > ENTIER_MAX => (StatusCode::PARTIAL_CONTENT, 0, MORCEAU - 1),
            None => (StatusCode::OK, 0, total.saturating_sub(1)),
        };
        let longueur = if total == 0 { 0 } else { fin - debut + 1 };
        let mut corps = vec![0; longueur as usize];
        fichier.seek(SeekFrom::Start(debut))?;
        fichier.read_exact(&mut corps)?;
        let mut r = reponse(statut, corps);
        let h = r.headers_mut();
        h.insert(header::CONTENT_TYPE, type_mime(&chemin).parse().unwrap());
        h.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
        if statut == StatusCode::PARTIAL_CONTENT {
            h.insert(header::CONTENT_RANGE, format!("bytes {debut}-{fin}/{total}").parse().unwrap());
        }
        Ok(r)
    })();
    lu.unwrap_or_else(|_| reponse(StatusCode::NOT_FOUND, Vec::new()))
}

/// Le PDF entier, pour pdf.js (Linux) : sous WebKitGTK, la page ne peut pas lire « apercu:// »
/// avec fetch (adresse d'une autre origine), alors on passe par un appel direct.
#[tauri::command]
pub async fn lire_fichier(chemin: String) -> Result<tauri::ipc::Response, String> {
    en_tache(move || {
        let p = PathBuf::from(&chemin);
        let taille = std::fs::metadata(&p).map_err(|e| message(&e, &p))?.len();
        if taille > PDF_MAX {
            return Err("Ce PDF est trop gros pour l'aperçu (plus de 100 Mo).".into());
        }
        std::fs::read(&p).map(tauri::ipc::Response::new).map_err(|e| message(&e, &p))
    })
    .await
}

#[derive(Serialize)]
pub struct Texte {
    /// None quand le fichier n'est pas du texte (octets nuls)
    texte: Option<String>,
    /// Le fichier continue au-delà de ce qui est montré
    tronque: bool,
}

/// Début d'un fichier, s'il contient du texte (UTF-8, ou Windows-1252 pour les vieux .txt / .csv).
#[tauri::command]
pub async fn lire_debut(chemin: String) -> Result<Texte, String> {
    en_tache(move || {
        let p = PathBuf::from(&chemin);
        let mut octets = Vec::with_capacity(TEXTE_MAX);
        let f = File::open(&p).map_err(|e| message(&e, &p))?;
        f.take(TEXTE_MAX as u64 + 1).read_to_end(&mut octets).map_err(|e| message(&e, &p))?;
        let tronque = octets.len() > TEXTE_MAX;
        octets.truncate(TEXTE_MAX);
        Ok(Texte { texte: decoder_texte(&octets, tronque), tronque })
    })
    .await
}

fn decoder_texte(octets: &[u8], tronque: bool) -> Option<String> {
    if octets[..octets.len().min(8192)].contains(&0) {
        return None;
    }
    match std::str::from_utf8(octets) {
        Ok(t) => Some(t.to_string()),
        // Coupé au milieu d'un caractère : on retire le morceau incomplet
        Err(e) if tronque && e.error_len().is_none() => Some(String::from_utf8_lossy(&octets[..e.valid_up_to()]).into_owned()),
        // Windows-1252 : chaque octet est un caractère
        Err(_) => Some(octets.iter().map(|&o| windows_1252(o)).collect()),
    }
}

/// Latin-1, sauf 0x80 à 0x9F où Windows range €, ’, œ, «… (fréquents dans un texte français).
fn windows_1252(o: u8) -> char {
    const ZONE: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}', '\u{90}', '‘', '’', '“',
        '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
    ];
    match o {
        0x80..=0x9F => ZONE[(o - 0x80) as usize],
        _ => o as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plages() {
        assert_eq!(plage("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(plage("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(plage("bytes=0-", 10 * MORCEAU), Some((0, MORCEAU - 1)));
        assert_eq!(plage("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(plage("bytes=0-5000", 1000), Some((0, 999)));
        assert_eq!(plage("bytes=1200-", 1000), None);
        assert_eq!(plage("octets=0-1", 1000), None);
    }

    #[test]
    fn textes() {
        assert_eq!(decoder_texte("Été".as_bytes(), false).as_deref(), Some("Été"));
        assert_eq!(decoder_texte(b"\x89PNG\r\n\x1a\n\0\0", false), None);
        assert_eq!(decoder_texte(b"caf\xe9 l\x92\x9cuvre", false).as_deref(), Some("café l’œuvre"));
        // « é » coupé en deux par la limite
        assert_eq!(decoder_texte(&"aé".as_bytes()[..2], true).as_deref(), Some("a"));
    }

    #[test]
    fn sert_un_fichier_par_morceaux() {
        let dossier = std::env::temp_dir().join(format!("ariane-apercu-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let f = dossier.join("vidéo test.mp4");
        std::fs::write(&f, b"0123456789").unwrap();
        let adresse = format!("http://apercu.localhost/{}", f.to_string_lossy().replace('%', "%25").replace(' ', "%20").replace('\\', "%5C").replace(':', "%3A"));
        let r = repondre(&Request::builder().uri(&adresse).header("range", "bytes=2-4").body(Vec::new()).unwrap());
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.body(), b"234");
        assert_eq!(r.headers()[header::CONTENT_TYPE], "video/mp4");
        let r = repondre(&Request::builder().uri(&adresse).body(Vec::new()).unwrap());
        assert_eq!((r.status(), r.body().as_slice()), (StatusCode::OK, b"0123456789".as_slice()));
        std::fs::remove_dir_all(dossier).ok();
    }
}
