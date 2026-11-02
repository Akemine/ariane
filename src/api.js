// Appels au backend Rust (Tauri). Les noms en camelCase sont convertis en snake_case côté Rust.
const appeler = (commande, args) => window.__TAURI__.core.invoke(commande, args);

export const api = {
  lister: (chemin) => appeler('lister_dossier', { chemin }),
  standards: () => appeler('dossiers_standards'),
  creerDossier: (parent, nom) => appeler('creer_dossier', { parent, nom }),
  creerFichier: (parent, nom) => appeler('creer_fichier', { parent, nom }),
  renommer: (chemin, nouveauNom) => appeler('renommer', { chemin, nouveauNom }),
  corbeille: (chemins) => appeler('mettre_a_la_corbeille', { chemins }),
  coller: (sources, destination, mode) => appeler('coller', { sources, destination, mode }),
  annulerCollage: (operations, mode) => appeler('annuler_collage', { operations, mode }),
  chercher: (racines, requete) => appeler('chercher', { racines, requete }),
  jeux: () => appeler('detecter_jeux'),
  iconesApplications: (executables) => appeler('icones_applications', { executables }),
  version: () => appeler('version_application'),
  verifierMiseAJour: () => appeler('verifier_mise_a_jour'),
  installerMiseAJour: () => appeler('installer_mise_a_jour'),
  git: (chemin) => appeler('etat_git', { chemin }),
  ouvrir: (chemin) => appeler('ouvrir', { chemin }),
  terminal: (chemin) => appeler('ouvrir_terminal', { chemin }),
  vscode: (chemin) => appeler('ouvrir_vscode', { chemin }),
  cheminInitial: () => appeler('chemin_initial'),
  lireConfig: () => appeler('lire_config'),
  ecrireConfig: (contenu) => appeler('ecrire_config', { contenu }),
};
