# Fonctions partagées par les scripts de publication d'Ariane.
# (Chargé par les autres scripts avec « . $PSScriptRoot\commun.ps1 ».)

# « Continue » : sous Windows PowerShell 5.1, « Stop » ferait échouer git et gh dès qu'ils écrivent
# un simple message sur la sortie d'erreur. On vérifie donc nous-mêmes leur code de retour.
$ErrorActionPreference = "Continue"
$sansBom = New-Object System.Text.UTF8Encoding $false
$OutputEncoding = $sansBom
Set-Location (Split-Path $PSScriptRoot -Parent)

function Etape($texte) { Write-Host "`n▶ $texte" -ForegroundColor Magenta }
function Verifier($message) { if ($LASTEXITCODE -ne 0) { throw $message } }

function Ecrire($fichier, $texte) { [IO.File]::WriteAllText((Resolve-Path $fichier), $texte, $sansBom) }
function Lire($fichier) { [IO.File]::ReadAllText((Resolve-Path $fichier)) }

# Après « git add -A » : refuse d'envoyer une clé privée sur le dépôt public, même si le .gitignore a sauté
function Verifier-SansCle {
    $cles = git diff --cached --name-only --diff-filter=d | Where-Object { $_ -match '\.key$' }
    if ($cles) {
        git reset -q -- $cles
        throw "Clé privée retirée de l'envoi ($($cles -join ', ')) : elle doit rester dans $HOME\.tauri\, hors du projet."
    }
}

# Clé de signature : ariane.key, ou celle créée du temps où l'appli s'appelait Explorateur
function Trouver-Cle {
    foreach ($nom in @("ariane.key", "explorateur.key")) {
        $c = Join-Path (Join-Path $HOME ".tauri") $nom
        if (Test-Path $c) { return $c }
    }
    return $null
}

# Compte et dépôt GitHub d'après l'adresse « origin » (https://github.com/compte/depot.git)
function Lire-Origine {
    $url = (git remote get-url origin 2>$null)
    if (-not $url) { return $null }
    if ($url -match 'github\.com[:/]([^/]+)/([^/]+?)(\.git)?$') { return @{ Compte = $Matches[1]; Depot = $Matches[2] } }
    return $null
}

# Écrit l'adresse des mises à jour et la clé publique dans tauri.conf.json, quel que soit leur état
function Configurer-MisesAJour($compte, $depot, $cle) {
    $conf = "src-tauri\tauri.conf.json"
    $texte = Lire $conf
    $publique = ([IO.File]::ReadAllText("$cle.pub")).Trim()
    $adresse = "https://github.com/$compte/$depot/releases/latest/download/latest.json"
    $texte = [regex]::Replace($texte, '"pubkey":\s*"[^"]*"', '"pubkey": "' + $publique + '"')
    $texte = [regex]::Replace($texte, '"https://github\.com/[^"]*/latest\.json"', '"' + $adresse + '"')
    Ecrire $conf $texte
}
