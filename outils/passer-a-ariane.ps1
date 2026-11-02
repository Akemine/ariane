# Passage de « Explorateur » à « Ariane », à faire une seule fois, après avoir décompressé
# la nouvelle version du projet par-dessus l'ancienne :
#   powershell -ExecutionPolicy Bypass -File .\outils\passer-a-ariane.ps1
#
# Renomme le dépôt GitHub en « ariane » (GitHub redirige les anciennes adresses, donc les
# applis déjà installées trouvent quand même la mise à jour), puis publie la version 0.8.0.

param([string]$Version = "0.8.0")
. "$PSScriptRoot\commun.ps1"

$origine = Lire-Origine
if (-not $origine) { throw "Ce dossier n'est pas relié à GitHub : lancez plutôt premiere-publication.ps1." }

if ($origine.Depot -ne "ariane") {
    Etape "Renommage du dépôt $($origine.Compte)/$($origine.Depot) en ariane"
    gh repo rename ariane --repo "$($origine.Compte)/$($origine.Depot)" --yes
    Verifier "Le renommage du dépôt a échoué."
    git remote set-url origin "https://github.com/$($origine.Compte)/ariane.git"
} else {
    Write-Host "Le dépôt s'appelle déjà ariane."
}

& "$PSScriptRoot\publier.ps1" $Version
