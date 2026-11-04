# Ariane

Ariane est un explorateur de fichiers coloré pour Linux (Fedora en priorité) et Windows 11, construit avec [Tauri 2](https://tauri.app) : backend en Rust, interface en HTML/CSS/JS sans framework ni étape de compilation.

## Ce que fait la version 0.1

- **Espaces** façon Arc : chaque espace a sa couleur, son icône, ses dossiers épinglés et ses propres onglets.
- **Deux panneaux côte à côte**, chacun avec ses onglets (bouton en bas de la barre latérale pour passer à un seul).
- **Liste à colonnes configurables** : état Git, taille, date de modification, tags. Tri par nom, taille ou date.
- **État Git** : branche, commits à pousser ou à récupérer, statut de chaque fichier (Modifié, Nouveau, Ignoré, Conflit). La colonne n'apparaît que dans un dépôt.
- **Couper / copier / coller** (Ctrl+X, Ctrl+C, Ctrl+V) et **glisser-déposer** : déposer un élément le déplace (couper/coller), sur un autre panneau, un dossier, une épingle, une tuile de la barre ou un morceau du chemin. D’un disque à un autre (vers une clé USB, depuis un disque externe), il est copié : les originaux restent.
- **Presse-papiers partagé avec le système** : un fichier copié (Ctrl+C) dans Ariane se colle dans Teams, Outlook, Thunderbird, l'Explorateur, Nautilus… et un fichier copié ou coupé dans l'Explorateur ou Nautilus se colle dans Ariane. Mêmes formats que l'Explorateur Windows (CF_HDROP) et que Nautilus (x-special/gnome-copied-files, text/uri-list). C'est l'application qui reçoit qui décide d'accepter un fichier collé : sous Linux, les sites web dans un navigateur ne prennent souvent que les images.
- **Conflits de noms** : quand le nom est déjà pris, une fenêtre compare les deux éléments (date, taille) et propose Remplacer, Garder les deux (« nom (2) ») ou Ignorer, avec une case pour appliquer le choix à tous. Échap annule le collage.
- **Annuler** le dernier collage ou déplacement (bouton dans la notification ou Ctrl+Z), y compris les remplacements : l'élément remplacé, mis à la corbeille, revient à sa place.
- **Barre latérale** : dossiers et fichiers épinglés de l'espace, dossiers de base (Accueil, Bureau, Documents, Téléchargements, Images, Musique, Vidéos), OneDrive sous Windows quand il est installé (perso et « OneDrive - Entreprise », avec son icône et, sous Windows 11, la jauge de remplissage que OneDrive donne à l'Explorateur), périphériques, tags.
- **Périphériques** : clés USB, disques externes, cartes SD et CD apparaissent dès qu'on les branche, avec leur espace libre. Un clic les ouvre (et les monte sous Linux), le bouton ⏏ les éjecte pour pouvoir les débrancher sans risque. Le disque du système n'y figure jamais.
- **Dossiers réseau** : un chemin `\\serveur\partage\dossier` (ou `smb://serveur/partage/dossier`) tapé dans la recherche ou la barre du chemin s'ouvre comme un dossier, et s'épingle. Si le serveur demande un identifiant, c'est la fenêtre de connexion du système qui s'affiche (le mot de passe ne passe pas par Ariane). Sous Linux, le partage est monté par GNOME comme dans Nautilus (paquet `gvfs-smb`, présent d'office sur Fedora Workstation), et un partage épinglé se reconnecte seul après un redémarrage.
- **Jeux (Windows)** : Steam, Battle.net, Ubisoft Connect, Epic Games, EA app et GOG Galaxy sont détectés avec leur icône ; un clic ouvre le dossier du jeu (ou déplie la liste quand il y en a plusieurs), clic droit pour lancer le launcher.
- **Recherche** depuis le champ en haut à gauche : dossier personnel, dossiers ouverts, épingles et dossiers des jeux.
- **Tags de couleur personnalisables**, rangés dans un tiroir de la barre latérale (clic sur « Tags » pour les afficher) (bouton + pour en créer, clic droit pour renommer, changer la couleur ou supprimer), et **couleur personnalisée par dossier**. Tags, couleurs et épingles suivent les éléments renommés ou déplacés.
- **Vue d'un tag** : un clic sur un tag de la barre ouvre, dans le panneau actif, tous les fichiers et dossiers qui le portent, où qu'ils soient sur le PC, avec le dossier de chacun. Clic droit › Afficher dans son dossier ; un second clic sur le tag revient d'où l'on vient.
- **Zones redimensionnables** : glisser la poignée entre la barre latérale et les panneaux (de 200 à 440 px, jamais plus de 40 % de la fenêtre), ou celle entre les deux panneaux (300 px minimum chacun). Double-clic : la barre s'ajuste au plus long nom, les panneaux reviennent à moitié-moitié. Au clavier : ← →.
- **Actions selon le type** au clic droit : extraire une archive (.zip, .tar, .tar.gz), compresser en .zip, dupliquer, calculer la taille d'un dossier, copier le contenu d'un fichier texte, exécuter en administrateur (Windows), ouvrir le dépôt Git dans le navigateur.
- **Clic droit** sur un élément ou dans le vide pour tout faire : créer un dossier ou un fichier, renommer, corbeille, ouvrir un terminal ou VS Code, copier le chemin…
- **Aperçu rapide** : Espace montre l'élément sélectionné sans ouvrir d'application, comme le Coup d'œil de macOS : images, PDF (lecteur d'Edge sous Windows, pdf.js sous Linux), vidéos, sons, textes et code, contenu d'un dossier, où l'on peut entrer (clic sur un sous-dossier ou un fichier, Retour arrière pour remonter). ↑ ↓ passent à l'élément voisin, Entrée l'ouvre, Espace ou Échap referme.
- **Actualisation automatique** : un fichier créé, renommé ou supprimé par une autre application (téléchargement fini, document enregistré…) apparaît tout seul dans la liste.
- **Ménage** (balai de la barre d'outils, ou clic droit › Faire le ménage ici) : Ariane parcourt le dossier et ses sous-dossiers et propose ce qui paraît inutile — téléchargements interrompus, fichiers temporaires (`~$devis.docx`…), installateurs de plus d'un mois dans les Téléchargements, archives (.zip, .tar, .tar.gz) déjà extraites — chacun de leurs fichiers est vérifié dans le dossier voisin, avec la même taille —, doublons identiques octet pour octet, gros fichiers de plus d'un an, vieux téléchargements, dossiers vides, `node_modules` / `target` abandonnés. Pour chacun, on choisit : Corbeille (Suppr), Garder (G, plus jamais proposé) ou Aperçu (Espace). Tout passe par la corbeille et Annuler remet tout en place. Jamais proposés : ce qui a moins de 7 jours, ce qui est tagué (avec son contenu), ce qui est épinglé (un dossier épinglé comme Téléchargements est tout de même passé en revue), les dossiers cachés et système, les dossiers de base même vides. Les fichiers OneDrive « en ligne seulement » ne sont jamais lus (ce qui les téléchargerait) : ni comparés pour les doublons, ni comptés comme gros fichiers.
- **Fichiers cachés** affichables (Ctrl+H), corbeille native du système.

## Lancer en développement

### Fedora 41 / 42

```bash
# Bibliothèques nécessaires à Tauri
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"

# Rust (via rustup) et Node.js
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
sudo dnf install nodejs

# Dans le dossier du projet
npm install
npm run dev
```

La première compilation prend quelques minutes, les suivantes sont rapides. Les modifications de `src/` (HTML, CSS, JS) se rechargent toutes seules ; celles de `src-tauri/` (Rust) relancent l'application.

### Windows 11

1. Installer les « Outils de génération Visual Studio » avec la charge de travail **Développement Desktop en C++**.
2. Installer Rust avec [rustup](https://rustup.rs) et Node.js LTS.
3. WebView2 est déjà présent sur Windows 11.
4. Installer [Git pour Windows](https://git-scm.com) pour avoir la colonne Git.
5. Dans le dossier du projet : `npm install` puis `npm run dev`.

## Construire l'application

```bash
npm run build
```

Les paquets sont dans `src-tauri/target/release/bundle/` :

- Fedora : `rpm/Ariane-0.8.0-1.x86_64.rpm`, à installer avec `sudo dnf install ./Ariane-0.8.0-1.x86_64.rpm`
- Windows : un installateur `.msi` et un `setup.exe`

### En faire le gestionnaire de fichiers par défaut (Linux)

Dans Ariane : **Réglages** (icône à côté du numéro de version, en bas de la barre latérale) › **Faire d'Ariane le gestionnaire de fichiers**. Le panneau montre ce qui passe déjà par Ariane et se défait d'un clic (« Rendre la main à Fichiers (Nautilus) »). Rien n'est modifié en dehors du dossier personnel :

- `xdg-mime default <Ariane>.desktop inode/directory` (dans `~/.config/mimeapps.list`) : les dossiers ouverts depuis une autre application s'ouvrent dans Ariane. Pour une AppImage, Ariane crée `~/.local/share/applications/ariane-appimage.desktop`.
- Le service D-Bus `org.freedesktop.FileManager1` : « Afficher dans le dossier » de Chrome, Firefox, VS Code… ouvre Ariane, même fermée, grâce à `~/.local/share/dbus-1/services/org.freedesktop.FileManager1.service`. Ce fichier se vérifie à chaque lancement : si Ariane a été désinstallée ou n'est plus le gestionnaire par défaut, il s'efface et Nautilus reprend la main. Nautilus, s'il tourne déjà en arrière-plan (il sert aussi de fenêtre « Ouvrir » à GNOME), garde ce service jusqu'à sa fermeture : le panneau le signale et propose de le fermer.

Fenêtre unique : une Ariane lancée alors qu'une autre tourne (`ariane ~/Projets`, un dossier ouvert depuis une autre application) passe sa demande à la première par D-Bus (`fr.ariane.desktop`) et s'arrête ; un onglet s'ouvre dans la fenêtre existante, qui passe au premier plan (jeton d'activation Wayland transmis).

Sous Windows, l'Explorateur reste le gestionnaire du système : « Afficher dans le dossier » des autres applications l'ouvre toujours.

## Publier sur GitHub, avec mises à jour automatiques

Une fois, depuis le dossier du projet sous Windows :

```powershell
powershell -ExecutionPolicy Bypass -File .\outils\premiere-publication.ps1
```

Le script installe Git et GitHub CLI si besoin, ouvre le navigateur pour se connecter à GitHub, crée la clé de signature (elle reste sur ce PC, dans `~\.tauri\`), crée le dépôt public `ariane`, y range la clé dans les secrets, envoie le code et lance la publication de la première version. GitHub compile ensuite Windows et Linux et publie tout sur la page *Releases* du dépôt (15 minutes environ).

Pour chaque nouvelle version :

```powershell
powershell -ExecutionPolicy Bypass -File .\outils\publier.ps1 0.8.0
```

Les nouveautés de chaque version s'écrivent dans `src/nouveautes.json`, dans l'entrée « prochaine ». `publier.ps1` lui donne le numéro et la date ; s'il n'y en a pas, il demande les nouveautés au moment de publier. Elles s'affichent dans Ariane après la mise à jour, et sur la page de version GitHub.

Les applications installées vérifient au démarrage s'il existe une version plus récente et proposent « Mettre à jour » (notification et bouton en bas de la barre latérale). Sous Linux, elle fonctionne pour l'AppImage, le .rpm et le .deb (ces deux-là demandent le mot de passe administrateur).

## Raccourcis

| Action | Souris | Clavier |
|---|---|---|
| Ouvrir | Double-clic | Entrée |
| Aperçu rapide | | Espace (↑ ↓ pour passer au voisin) |
| Ouvrir un dossier dans un onglet | Clic molette | |
| Sélection multiple | Ctrl+clic, Maj+clic, ou tracer une zone depuis un endroit libre | Ctrl+A, Maj+↑/↓ |
| Couper / copier / coller | Clic droit, ou glisser-déposer pour déplacer | Ctrl+X / Ctrl+C / Ctrl+V |
| Annuler le dernier collage | Bouton « Annuler » de la notification | Ctrl+Z |
| Renommer | Clic droit › Renommer | F2 |
| Mettre à la corbeille | Clic droit | Suppr |
| Nouveau dossier | Clic droit dans le vide | Ctrl+Maj+N |
| Précédent / suivant | Boutons latéraux de la souris | Alt+← / Alt+→, Retour arrière |
| Dossier parent | | Alt+↑ |
| Fichiers cachés | Bouton « Cachés » | Ctrl+H |
| Nouvel onglet / fermer l'onglet | Bouton + | Ctrl+T / Ctrl+W |
| Onglet suivant | | Ctrl+Tab |
| Modifier ou copier le chemin | Clic dans la barre du chemin | Ctrl+L |
| Chercher | Champ en haut à gauche | Ctrl+F |
| Actualiser | | F5 |

Dans le champ de recherche, taper un chemin (`/etc`, `~/Projets`, `C:\Users`) puis Entrée y emmène directement.

## Où sont les réglages ?

Espaces, épingles, tags, couleurs et colonnes sont enregistrés dans un fichier JSON :

- Linux : `~/.config/fr.ariane.desktop/config.json`
- Windows : `%APPDATA%\fr.ariane.desktop\config.json`

## Organisation du code

```
src/                  Interface (servie telle quelle, sans compilation)
  index.html
  styles.css          Couleurs de l'espace actif : --espace, --encre, --teinte
  app.js              État, rendu, menus, glisser-déposer, raccourcis
  api.js              Appels au backend Rust
  format.js           Tailles, dates, couleurs par type de fichier
  icones.js           Icônes SVG
  polices/            Plus Jakarta Sans et JetBrains Mono (licence OFL), embarquées
  vendor/pdfjs/       pdf.js (Mozilla, Apache 2.0) : aperçu des PDF sous Linux

src-tauri/            Backend Rust
  src/fichiers.rs     Lister, créer, renommer, copier, déplacer, corbeille
  src/git.rs          État Git (via la commande git)
  src/systeme.rs      Ouvrir un fichier, un terminal, VS Code
  src/peripheriques.rs  Clés USB, disques externes : détecter, monter, éjecter
  src/apercu.rs       Aperçu rapide : adresse apercu://, début des textes
  src/surveillance.rs Actualisation automatique des dossiers affichés
  src/menage.rs       Ménage : repérer l'inutile, ressortir de la corbeille
  src/presse_papiers.rs  Presse-papiers du système (fichiers copiés / coupés)
  src/nuages.rs       Remplissage de OneDrive (Windows 11)
  src/reseau.rs       Dossiers réseau : connexion (Windows), montage par GNOME (Linux)
  src/config.rs       Lecture / écriture des réglages
  tauri.conf.json     Fenêtre, paquets rpm / deb / msi
```

Tests du backend : `cd src-tauri && cargo test`.

## Limites connues de la v0.1

- **Dossiers en conflit** : « Remplacer » remplace le dossier entier ; il n'y a pas encore de fusion du contenu des deux dossiers.
- **Pièces jointes « virtuelles »** : un fichier copié directement depuis une pièce jointe d'Outlook (sans être enregistré) ne se colle pas encore dans Ariane.
- **Glisser-déposer avec les autres applications** (vers Firefox, VS Code…) pas encore géré.
- **Actualisation automatique sur les partages réseau sous Linux** : les changements faits depuis un autre PC n'arrivent pas (gvfs ne les signale pas) ; F5 ou le retour dans la fenêtre les affichent.
- **Pas de barre de progression** pour les grosses copies (l'interface reste utilisable pendant la copie).
- Le **tableau de bord**, la **vue d'analyse de l'espace disque** et un **thème sombre** sont les prochaines étapes.
