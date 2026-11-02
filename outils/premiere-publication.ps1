# Mise en ligne d'Ariane sur GitHub, en une fois.
# À lancer depuis le dossier du projet :
#   powershell -ExecutionPolicy Bypass -File .\outils\premiere-publication.ps1
#
# Le script : se connecte à GitHub (dans le navigateur), crée la clé de signature,
# règle l'adresse des mises à jour, crée le dépôt, y enregistre la clé en secret,
# envoie le code et lance la publication de la première version.

param([string]$Depot = "ariane")
. "$PSScriptRoot\commun.ps1"

# 1. Outils nécessaires
Etape "Vérification des outils"
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    winget install --id Git.Git -e --accept-source-agreements --accept-package-agreements
    throw "Git vient d'être installé : fermez et rouvrez PowerShell, puis relancez ce script."
}
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    winget install --id GitHub.cli -e --accept-source-agreements --accept-package-agreements
    throw "GitHub CLI vient d'être installé : fermez et rouvrez PowerShell, puis relancez ce script."
}

# 2. Connexion à GitHub (ouvre le navigateur la première fois)
Etape "Connexion à GitHub"
gh auth status 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { gh auth login --web --git-protocol https }
Verifier "La connexion à GitHub n'a pas abouti."
$compte = (gh api user --jq .login).Trim()
Write-Host "Connecté en tant que $compte"
if (-not (git config user.name)) { git config --global user.name $compte }
if (-not (git config user.email)) {
    $id = (gh api user --jq .id).Trim()
    git config --global user.email "$id+$compte@users.noreply.github.com"
}

# 3. Clé de signature des mises à jour (reste sur ce PC, ne jamais la partager)
Etape "Clé de signature"
$cle = Trouver-Cle
if (-not $cle) {
    $cle = Join-Path (Join-Path $HOME ".tauri") "ariane.key"
    New-Item -ItemType Directory -Force (Split-Path $cle) | Out-Null
    Write-Host "Choisissez un mot de passe pour la clé (notez-le, il sera demandé juste après)."
    npx tauri signer generate -w $cle
    Verifier "La création de la clé a échoué."
} else {
    Write-Host "Clé existante réutilisée : $cle"
}
$motDePasse = Read-Host "Mot de passe de la clé (laisser vide s'il n'y en a pas)" -AsSecureString
$motDePasse = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($motDePasse))

# 4. Adresse des mises à jour et clé publique dans la configuration de l'appli
Etape "Configuration de l'application"
Configurer-MisesAJour $compte $Depot $cle

# 5. Dépôt GitHub et code
Etape "Création du dépôt $compte/$Depot"
if (-not (Test-Path ".git")) { git init -b main | Out-Null }
git add -A
git commit -m "Ariane, première publication" 2>$null | Out-Null
gh repo view "$compte/$Depot" 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    gh repo create $Depot --public --source . --remote origin --push
    Verifier "La création du dépôt a échoué."
} else {
    if (-not (git remote 2>$null | Select-String -Quiet "^origin$")) { git remote add origin "https://github.com/$compte/$Depot.git" }
    git push -u origin main
    Verifier "L'envoi du code a échoué."
}

# 6. Secrets : la clé privée et son mot de passe, lisibles seulement par la publication
Etape "Enregistrement de la clé dans les secrets du dépôt"
# --body plutôt qu'un tube : PowerShell ajouterait un retour à la ligne qui casserait la clé
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$compte/$Depot" --body ([IO.File]::ReadAllText($cle).Trim())
Verifier "L'enregistrement de la clé a échoué."
if ($motDePasse) {
    gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$compte/$Depot" --body $motDePasse
    Verifier "L'enregistrement du mot de passe a échoué."
}

# 7. Première version
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
Etape "Publication de la version $version"
git tag -a "v$version" -m "Version $version"
git push origin "v$version"
Verifier "L'envoi de la version a échoué."

Write-Host "`nC'est lancé. Compilation et publication en cours (15 minutes environ) :" -ForegroundColor Green
Write-Host "  https://github.com/$compte/$Depot/actions"
Write-Host "La version sera ensuite ici :"
Write-Host "  https://github.com/$compte/$Depot/releases/latest"
