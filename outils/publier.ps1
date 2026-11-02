# Publie une nouvelle version d'Ariane : change le numéro, enregistre, envoie sur GitHub.
# GitHub compile, signe et publie tout seul ; les applis installées proposent la mise à jour.
#   powershell -ExecutionPolicy Bypass -File .\outils\publier.ps1 0.8.0

param([Parameter(Mandatory = $true)][string]$Version)
. "$PSScriptRoot\commun.ps1"

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Numéro attendu au format 1.2.3 (par exemple 0.8.0)." }

# Adresse des mises à jour et clé publique toujours justes, même après avoir remplacé des fichiers
$origine = Lire-Origine
$cle = Trouver-Cle
if (-not $origine) { throw "Ce dossier n'est pas encore relié à GitHub : lancez d'abord premiere-publication.ps1." }
if (-not $cle) { throw "Clé de signature introuvable dans $HOME\.tauri\ : sans elle, les mises à jour seraient refusées." }
Configurer-MisesAJour $origine.Compte $origine.Depot $cle

function Remplacer($fichier, $motif, $par) {
    # Première occurrence seulement
    Ecrire $fichier (([regex]$motif).Replace((Lire $fichier), $par, 1))
}
Remplacer "package.json" '"version":\s*"[^"]*"' "`"version`": `"$Version`""
Remplacer "src-tauri\tauri.conf.json" '"version":\s*"[^"]*"' "`"version`": `"$Version`""
Remplacer "src-tauri\Cargo.toml" '(?m)^version = "[^"]*"' "version = `"$Version`""

# Notes de version : l'entrée « prochaine » de src\nouveautes.json reçoit ce numéro et la date du jour.
# S'il n'y en a pas, on demande les nouveautés ici (une par ligne, ligne vide pour finir).
$fichierNotes = "src\nouveautes.json"
$notes = Lire $fichierNotes
$aujourdhui = Get-Date -Format "yyyy-MM-dd"
if ($notes -match ('"version":\s*"' + [regex]::Escape($Version) + '"')) {
    Write-Host "Notes de la version $Version déjà présentes."
} elseif ($notes -match '"version":\s*"prochaine"') {
    $notes = ([regex]'"version":\s*"prochaine",\s*"date":\s*"[^"]*"').Replace($notes, "`"version`": `"$Version`",`n    `"date`": `"$aujourdhui`"", 1)
    Ecrire $fichierNotes $notes
    Write-Host "Notes de la version $Version datées du $aujourdhui."
} else {
    Write-Host "Quoi de neuf dans la $Version ? Une nouveauté par ligne, ligne vide pour terminer." -ForegroundColor Cyan
    $points = @()
    while ($true) { $ligne = Read-Host "  -"; if (-not $ligne) { break }; $points += $ligne }
    if ($points.Count -gt 0) {
        $echappe = $points | ForEach-Object { '      "' + ($_ -replace '\\', '\\' -replace '"', '\"') + '"' }
        $entree = "  {`n    `"version`": `"$Version`",`n    `"date`": `"$aujourdhui`",`n    `"points`": [`n" + ($echappe -join ",`n") + "`n    ]`n  },"
        $notes = $notes.Substring(0, $notes.IndexOf('[') + 1) + "`n" + $entree + $notes.Substring($notes.IndexOf('[') + 1)
        Ecrire $fichierNotes $notes
    }
}

Etape "Publication de la version $Version"
git add -A
git commit -m "Version $Version"
git tag -a "v$Version" -m "Version $Version"
git push origin HEAD --follow-tags
Verifier "L'envoi sur GitHub a échoué."

Write-Host "`nVersion $Version envoyée. Publication en cours (15 minutes environ) :" -ForegroundColor Green
Write-Host "  https://github.com/$($origine.Compte)/$($origine.Depot)/actions"
