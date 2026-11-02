// Ariane — interface.
// Rendu en HTML simple + délégation d'événements : pas de framework, pas de compilation.

import { api } from './api.js';
import { taille, elements, dateRelative, dateComplete, typeFichier, echapper as esc, normaliser, pluriel } from './format.js';
import { ic, icDossier, icFichier, icDossierSpecial } from './icones.js';

// ———————————————————————————— Constantes ————————————————————————————

const PALETTE = {
  violet: { couleur: '#6D28D9', encre: '#4C1D95', teinte: '#ECE7FD', nom: 'Violet' },
  bleu: { couleur: '#2563EB', encre: '#1E40AF', teinte: '#E3ECFD', nom: 'Bleu' },
  turquoise: { couleur: '#0891B2', encre: '#155E75', teinte: '#DFF3F7', nom: 'Turquoise' },
  vert: { couleur: '#16A34A', encre: '#166534', teinte: '#E2F5E7', nom: 'Vert' },
  orange: { couleur: '#EA580C', encre: '#9A3412', teinte: '#FFEEDF', nom: 'Orange' },
  rose: { couleur: '#DB2777', encre: '#9D174D', teinte: '#FCE8F2', nom: 'Rose' },
  rouge: { couleur: '#DC2626', encre: '#991B1B', teinte: '#FDE7E7', nom: 'Rouge' },
  ardoise: { couleur: '#475569', encre: '#1E293B', teinte: '#E8ECF1', nom: 'Ardoise' },
};

// Couleurs proposées pour les tags : point, fond de la pastille, texte de la pastille
const COULEURS_TAG = {
  rouge: { nom: 'Rouge', couleur: '#DC2626', fond: '#FEE2E2', texte: '#991B1B' },
  orange: { nom: 'Orange', couleur: '#EA580C', fond: '#FFEDD5', texte: '#9A3412' },
  ambre: { nom: 'Ambre', couleur: '#D97706', fond: '#FEF3C7', texte: '#92400E' },
  vert: { nom: 'Vert', couleur: '#16A34A', fond: '#DCFCE7', texte: '#166534' },
  turquoise: { nom: 'Turquoise', couleur: '#0891B2', fond: '#CFFAFE', texte: '#155E75' },
  bleu: { nom: 'Bleu', couleur: '#2563EB', fond: '#DBEAFE', texte: '#1E40AF' },
  violet: { nom: 'Violet', couleur: '#7C3AED', fond: '#EDE9FE', texte: '#5B21B6' },
  rose: { nom: 'Rose', couleur: '#DB2777', fond: '#FCE7F3', texte: '#9D174D' },
  ardoise: { nom: 'Ardoise', couleur: '#475569', fond: '#E2E8F0', texte: '#1E293B' },
};

const TAGS_PAR_DEFAUT = [
  { id: 'urgent', libelle: 'Urgent', couleur: 'rouge' },
  { id: 'en-cours', libelle: 'En cours', couleur: 'orange' },
  { id: 'a-relire', libelle: 'À relire', couleur: 'bleu' },
  { id: 'perso', libelle: 'Perso', couleur: 'rose' },
];

/** Tags définis par l'utilisateur, avec leurs couleurs. */
function listeTags() {
  return config.listeTags.map((t) => ({ ...t, ...(COULEURS_TAG[t.couleur] || COULEURS_TAG.ardoise), nomCouleur: t.couleur }));
}
function trouverTag(id) {
  return listeTags().find((t) => t.id === id);
}

const COULEURS_DOSSIER = ['#7C3AED', '#2563EB', '#0891B2', '#16A34A', '#EA580C', '#DB2777', '#DC2626', '#475569'];
const COULEUR_DOSSIER_DEFAUT = '#5B8DEF';
const COULEUR_DOSSIER_CACHE = '#A1A1AA';
const ICONES_ESPACE = ['maison', 'code', 'manette', 'etoile', 'livre', 'valise', 'musique', 'dossier'];

const GIT = {
  modifie: { libelle: 'Modifié', fond: '#FFEDD5', texte: '#9A3412', point: '#EA580C' },
  nouveau: { libelle: 'Nouveau', fond: '#DCFCE7', texte: '#166534', point: '#16A34A' },
  ignore: { libelle: 'Ignoré', fond: '#F1F1F4', texte: '#52525B', point: '#A1A1AA' },
  conflit: { libelle: 'Conflit', fond: '#FEE2E2', texte: '#991B1B', point: '#DC2626' },
  a_jour: { libelle: 'À jour', fond: '#F6F6F8', texte: '#52525B', point: '#22C55E' },
};

// ———————————————————————————— État ————————————————————————————

let config = null; // réglages enregistrés sur le disque
let standards = null; // dossiers personnels du système
let SEP = '/';

const etat = {
  panneaux: [],
  actif: 0,
  pressePapiers: null, // { mode: 'copier' | 'couper', chemins: [] }
  menu: null, // contexte du menu ouvert
  glisse: null, // glisser-déposer en cours
  survolEl: null,
  renduDiffere: new Set(),
  filtreTag: null,
  recherche: '',
  recherchePanneau: 0,
  colonnesOuvert: null,
  annulation: null, // dernière action annulable (Ctrl+Z)
  lanceurs: [], // launchers de jeux détectés (Windows)
  iconesApps: {}, // chemin d'un .exe → image de son icône
  version: '', // version installée
  miseAJour: null, // { version, notes } quand une version plus récente est publiée
  installationEnCours: false,
  verificationEnCours: false,
  majAJour: false, // coche verte quelques secondes après une vérification sans nouveauté
  majAnimee: false, // l'animation d'arrivée d'une mise à jour ne se joue qu'une fois
};

const $ = (s, racine = document) => racine.querySelector(s);

const DELAI_DOUBLE_CLIC = 500; // ms
let dernierClic = { chemin: null, temps: 0 };

// ———————————————————————————— Outils ————————————————————————————

function espace() {
  return config.espaces.find((e) => e.id === config.espaceActif) || config.espaces[0];
}
function couleursEspace(e = espace()) {
  return PALETTE[e.couleur] || PALETTE.violet;
}
function nomDe(chemin) {
  const morceaux = chemin.split(/[\\/]+/).filter(Boolean);
  return morceaux.length ? morceaux[morceaux.length - 1] : chemin;
}
function nomAffiche(chemin) {
  return chemin === standards.accueil ? 'Accueil' : nomDe(chemin);
}
/** `chemin` est-il `parent` ou à l'intérieur ? */
function estSous(chemin, parent) {
  if (chemin === parent) return true;
  const base = parent.endsWith(SEP) ? parent : parent + SEP;
  return chemin.startsWith(base);
}
/** Nom du dossier spécial (« telechargements », « images »…) ou null. */
function dossierSpecial(chemin) {
  const s = standards;
  for (const cle of ['accueil', 'telechargements', 'documents', 'images', 'musique', 'videos', 'bureau']) if (s[cle] && s[cle] === chemin) return cle;
  return null;
}

function couleurDossier(chemin, entree) {
  if (config.couleursDossiers[chemin]) return config.couleursDossiers[chemin];
  const s = standards;
  const speciaux = {
    [s.accueil]: '#2563EB',
    [s.telechargements]: '#16A34A',
    [s.bureau]: '#0891B2',
    [s.documents]: '#7C3AED',
    [s.images]: '#DB2777',
    [s.musique]: '#E11D48',
    [s.videos]: '#9333EA',
  };
  if (speciaux[chemin]) return speciaux[chemin];
  if (entree?.cache) return COULEUR_DOSSIER_CACHE;
  return COULEUR_DOSSIER_DEFAUT;
}
function panneau(i = etat.actif) {
  return etat.panneaux[i];
}
function ongletCourant(p) {
  return p.onglets[p.ongletActif];
}
function dossierCourant(i = etat.actif) {
  return panneau(i)?.contenu?.chemin ?? null;
}
function nbPanneaux() {
  return config.deuxPanneaux ? 2 : 1;
}
function erreur(e) {
  notifier(String(e?.message ?? e), { erreur: true });
}
async function essayer(promesse) {
  try {
    return await promesse;
  } catch (e) {
    erreur(e);
    return undefined;
  }
}
function libelleElements(chemins) {
  return chemins.length === 1 ? `« ${nomDe(chemins[0])} »` : `${chemins.length} éléments`;
}

// ———————————————————————————— Configuration ————————————————————————————

function configParDefaut() {
  const a = standards.accueil;
  const t = standards.telechargements || a;
  return {
    version: 1,
    espaces: [
      { id: 'maison', nom: 'Maison', couleur: 'vert', icone: 'maison', epingles: [], onglets: [{ chemins: [a], actif: 0 }, { chemins: [t], actif: 0 }] },
      { id: 'dev', nom: 'Dev', couleur: 'violet', icone: 'code', epingles: [], onglets: [{ chemins: [a], actif: 0 }, { chemins: [a], actif: 0 }] },
      { id: 'loisirs', nom: 'Loisirs', couleur: 'orange', icone: 'manette', epingles: [], onglets: [{ chemins: [a], actif: 0 }, { chemins: [t], actif: 0 }] },
    ],
    espaceActif: 'maison',
    couleursDossiers: {},
    tags: {},
    colonnes: { git: true, taille: true, modifie: true, tags: true },
    listeTags: TAGS_PAR_DEFAUT.map((t) => ({ ...t })),
    afficherCaches: false,
    deuxPanneaux: true,
    jeuxDeplies: {},
    groupesDate: {},
    theme: matchMedia('(prefers-color-scheme: dark)').matches ? 'sombre' : 'clair',
  };
}

function completer(lu) {
  const base = configParDefaut();
  const c = { ...base, ...lu };
  c.colonnes = { ...base.colonnes, ...(lu.colonnes || {}) };
  if (!Array.isArray(c.espaces) || !c.espaces.length) c.espaces = base.espaces;
  // Épingles : anciennes versions = simples chemins de dossiers
  for (const e of c.espaces) e.epingles = (e.epingles || []).map((x) => (typeof x === 'string' ? { chemin: x, dossier: true } : x));
  if (!Array.isArray(c.listeTags)) c.listeTags = TAGS_PAR_DEFAUT.map((t) => ({ ...t }));
  c.couleursDossiers ??= {};
  c.jeuxDeplies ??= {};
  c.groupesDate ??= {};
  // Premier lancement : on suit le réglage clair/sombre du système
  c.theme ??= matchMedia('(prefers-color-scheme: dark)').matches ? 'sombre' : 'clair';
  c.tags ??= {};
  return c;
}

let minuterieSauvegarde = null;
function sauver() {
  clearTimeout(minuterieSauvegarde);
  minuterieSauvegarde = setTimeout(() => {
    memoriserOnglets();
    api.ecrireConfig(JSON.stringify(config, null, 2)).catch((e) => console.error('Sauvegarde impossible', e));
  }, 300);
}

/** Chaque espace garde ses propres onglets, comme dans Arc. */
function memoriserOnglets() {
  const e = espace();
  const anciens = e.onglets || [];
  e.onglets = [0, 1].map((i) => {
    const p = etat.panneaux[i];
    return p ? { chemins: p.onglets.map((o) => o.chemin), actif: p.ongletActif } : anciens[i];
  });
}

function appliquerTheme() {
  const c = couleursEspace();
  const racine = document.documentElement;
  const r = racine.style;
  const sombre = config.theme === 'sombre';
  racine.dataset.theme = sombre ? 'sombre' : 'clair';
  r.colorScheme = sombre ? 'dark' : 'light';
  r.setProperty('--espace', c.couleur);
  // En sombre : fond teinté de la couleur de l'espace, et encre éclaircie pour rester lisible
  r.setProperty('--encre', sombre ? `color-mix(in srgb, ${c.couleur} 45%, #FFFFFF)` : c.encre);
  r.setProperty('--teinte', sombre ? `color-mix(in srgb, ${c.couleur} 18%, #110F18)` : c.teinte);
}

function basculerModeSombre() {
  const racine = document.documentElement;
  racine.classList.add('changement-theme');
  config.theme = config.theme === 'sombre' ? 'clair' : 'sombre';
  appliquerTheme();
  renderBarre();
  sauver();
  setTimeout(() => racine.classList.remove('changement-theme'), 350);
}

// ———————————————————————————— Panneaux et navigation ————————————————————————————

function nouveauPanneau(chemins, actif = 0) {
  return {
    onglets: chemins.map((c) => ({ chemin: c, histo: [c], pos: 0 })),
    ongletActif: Math.min(actif, chemins.length - 1),
    contenu: null,
    git: null,
    erreur: null,
    selection: new Set(),
    ancre: null,
    tri: { col: 'nom', sens: 1 },
    renommage: null,
    jeton: 0,
    visibles: [],
    reveler: null,
    hautDePage: false,
  };
}

function construirePanneaux(e) {
  const defauts = [standards.accueil, standards.telechargements || standards.accueil];
  etat.panneaux = [0, 1].map((i) => {
    const s = e.onglets?.[i];
    const chemins = s?.chemins?.length ? s.chemins : [defauts[i]];
    return nouveauPanneau(chemins, s?.actif || 0);
  });
  etat.actif = 0;
}

/** Recharge le dossier affiché (sans toucher à l'historique). */
async function charger(i) {
  const p = panneau(i);
  if (!p) return;
  const o = ongletCourant(p);
  const jeton = ++p.jeton;
  try {
    const c = await api.lister(o.chemin);
    if (jeton !== p.jeton) return;
    p.contenu = c;
    p.erreur = null;
    o.chemin = c.chemin;
    const existants = new Set(c.entrees.map((x) => x.chemin));
    p.selection = new Set([...p.selection].filter((x) => existants.has(x)));
    if (p.renommage && !existants.has(p.renommage)) p.renommage = null;
  } catch (e) {
    if (jeton !== p.jeton) return;
    // Dossier disparu au démarrage : on retombe sur l'accueil
    if (!p.contenu && o.chemin !== standards.accueil) {
      o.chemin = standards.accueil;
      o.histo = [o.chemin];
      o.pos = 0;
      return charger(i);
    }
    p.contenu = null;
    p.erreur = String(e);
  }
  renderPanneau(i);
  chargerGit(i, jeton);
}

async function chargerGit(i, jeton) {
  const p = panneau(i);
  if (!p?.contenu) return;
  try {
    const g = await api.git(p.contenu.chemin);
    if (jeton !== p.jeton) return;
    p.git = g.depot ? g : null;
    renderPanneau(i);
  } catch {
    /* pas de git sur la machine : on n'affiche simplement rien */
  }
}

/** Va dans un dossier. Ne change rien si le dossier ne peut pas être lu. */
async function naviguer(i, chemin, { histo = true } = {}) {
  const p = panneau(i);
  const o = ongletCourant(p);
  const jeton = ++p.jeton;
  let c;
  try {
    c = await api.lister(chemin);
  } catch (e) {
    if (jeton === p.jeton) erreur(e);
    return false;
  }
  if (jeton !== p.jeton) return false;
  const precedent = o.chemin;
  if (histo && c.chemin !== o.chemin) {
    o.histo = o.histo.slice(0, o.pos + 1);
    o.histo.push(c.chemin);
    o.pos = o.histo.length - 1;
  }
  o.chemin = c.chemin;
  p.contenu = c;
  p.erreur = null;
  p.git = null;
  p.renommage = null;
  p.selection = new Set();
  p.ancre = null;
  p.hautDePage = true;
  // En remontant, on retrouve le dossier d'où l'on vient déjà sélectionné
  if (c.entrees.some((x) => x.chemin === precedent)) {
    p.selection.add(precedent);
    p.ancre = precedent;
    p.reveler = precedent;
  }
  if (etat.recherche && etat.recherchePanneau === i) viderRecherche();
  renderPanneau(i);
  sauver();
  chargerGit(i, jeton);
  return true;
}

async function historique(i, delta) {
  const o = ongletCourant(panneau(i));
  const cible = o.histo[o.pos + delta];
  if (cible == null) return;
  if (await naviguer(i, cible, { histo: false })) {
    o.pos += delta;
    renderPanneau(i); // met à jour les boutons précédent / suivant
  }
}

function remonter(i) {
  const parent = panneau(i).contenu?.parent;
  if (parent) naviguer(i, parent);
}

function nouvelOnglet(i, chemin) {
  const p = panneau(i);
  p.onglets.push({ chemin, histo: [chemin], pos: 0 });
  changerOnglet(i, p.onglets.length - 1);
}

function changerOnglet(i, k) {
  const p = panneau(i);
  if (k < 0 || k >= p.onglets.length) return;
  p.ongletActif = k;
  p.contenu = null;
  p.git = null;
  p.selection = new Set();
  p.renommage = null;
  p.hautDePage = true;
  renderPanneau(i);
  charger(i);
  sauver();
}

function fermerOnglet(i, k) {
  const p = panneau(i);
  if (p.onglets.length <= 1) return;
  p.onglets.splice(k, 1);
  const actif = k < p.ongletActif || p.ongletActif >= p.onglets.length ? Math.max(0, p.ongletActif - 1) : p.ongletActif;
  changerOnglet(i, actif);
}

/** Recharge les panneaux, sauf celui où l'on est en train de taper un nouveau nom. */
// ———————————————————————————— Saisie du chemin ————————————————————————————

function editerChemin(i) {
  const p = panneau(i);
  if (!p.contenu) return;
  p.editionChemin = true;
  p.valeurChemin = null;
  renderPanneau(i);
}

function fermerEditionChemin(i) {
  const p = panneau(i);
  if (!p.editionChemin) return;
  p.editionChemin = false;
  p.valeurChemin = null;
  renderPanneau(i);
}

async function validerChemin(i, champ) {
  const p = panneau(i);
  const valeur = champ.value.trim();
  p.editionChemin = false;
  champ.blur();
  if (!valeur || valeur === dossierCourant(i)) return renderPanneau(i);
  if (!(await naviguer(i, valeur))) {
    // Chemin introuvable : on rouvre la saisie avec ce qui a été tapé, pour corriger
    p.editionChemin = true;
    p.valeurChemin = valeur;
    renderPanneau(i);
  }
}

function rafraichirTout() {
  const saisie = document.activeElement;
  for (let i = 0; i < nbPanneaux(); i++) {
    const sec = document.querySelector(`.panneau[data-i="${i}"]`);
    const enTrainDeRenommer = !!panneau(i).renommage && saisie?.classList.contains('renommage') && sec?.contains(saisie);
    if (!enTrainDeRenommer) charger(i);
  }
}

function activerPanneau(i) {
  if (i === etat.actif) return;
  etat.actif = i;
  document.querySelectorAll('.panneau').forEach((s) => s.classList.toggle('actif', +s.dataset.i === i));
}

function changerEspace(id, { memoriser = true } = {}) {
  if (id === config.espaceActif) return;
  if (memoriser) memoriserOnglets();
  config.espaceActif = id;
  etat.filtreTag = null;
  etat.colonnesOuvert = null;
  viderRecherche();
  appliquerTheme();
  construirePanneaux(espace());
  renderStructure();
  renderBarre();
  for (let i = 0; i < nbPanneaux(); i++) charger(i);
  sauver();
}

// ———————————————————————————— Liste visible ————————————————————————————

function calculerVisibles(i) {
  const p = panneau(i);
  if (!p.contenu) return [];
  let liste = p.contenu.entrees;
  if (!config.afficherCaches) liste = liste.filter((e) => !e.cache);
  if (etat.filtreTag) liste = liste.filter((e) => (config.tags[e.chemin] || []).includes(etat.filtreTag));
  if (etat.recherche && etat.recherchePanneau === i) {
    const q = normaliser(etat.recherche);
    liste = liste.filter((e) => normaliser(e.nom).includes(q));
  }
  const { col, sens } = p.tri;
  const parNom = (a, b) => a.nom.localeCompare(b.nom, 'fr', { numeric: true, sensitivity: 'base' });
  if (estGroupeParDate(i)) return [...liste].sort((a, b) => (b.modifie ?? 0) - (a.modifie ?? 0) || parNom(a, b));
  return [...liste].sort((a, b) => {
    if (a.est_dossier !== b.est_dossier) return a.est_dossier ? -1 : 1; // dossiers d'abord
    let r = 0;
    if (col === 'taille') r = a.est_dossier ? (a.nb_elements ?? 0) - (b.nb_elements ?? 0) : (a.taille ?? 0) - (b.taille ?? 0);
    else if (col === 'modifie') r = (a.modifie ?? 0) - (b.modifie ?? 0);
    return (r || parNom(a, b)) * sens;
  });
}

// ——— Regroupement par date, comme dans l'Explorateur Windows ———

function estGroupeParDate(i) {
  const chemin = dossierCourant(i);
  if (!chemin) return false;
  return config.groupesDate[chemin] ?? chemin === standards.telechargements;
}

/** « Aujourd'hui », « Hier », « Plus tôt cette semaine »… puis l'année pour les plus anciens. */
function groupeDate(ms) {
  if (ms == null) return 'Date inconnue';
  const maintenant = new Date();
  const jour = new Date(maintenant.getFullYear(), maintenant.getMonth(), maintenant.getDate()).getTime();
  const lundi = jour - ((new Date(jour).getDay() + 6) % 7) * 86400000;
  const debutMois = new Date(maintenant.getFullYear(), maintenant.getMonth(), 1).getTime();
  const debutMoisDernier = new Date(maintenant.getFullYear(), maintenant.getMonth() - 1, 1).getTime();
  const debutAnnee = new Date(maintenant.getFullYear(), 0, 1).getTime();
  if (ms >= jour) return 'Aujourd’hui';
  if (ms >= jour - 86400000) return 'Hier';
  if (ms >= lundi) return 'Plus tôt cette semaine';
  if (ms >= lundi - 7 * 86400000) return 'La semaine dernière';
  if (ms >= debutMois) return 'Plus tôt ce mois-ci';
  if (ms >= debutMoisDernier) return 'Le mois dernier';
  if (ms >= debutAnnee) return 'Plus tôt cette année';
  return String(new Date(ms).getFullYear());
}

/** Range la liste triée en groupes ; les groupes repliés ne gardent que leur titre. */
function grouper(i, liste) {
  const p = panneau(i);
  p.replies ??= new Set();
  const groupes = [];
  for (const e of liste) {
    const libelle = groupeDate(e.modifie);
    if (groupes.at(-1)?.libelle !== libelle) groupes.push({ libelle, entrees: [] });
    groupes.at(-1).entrees.push(e);
  }
  p.groupes = groupes;
  return groupes.flatMap((g) => (p.replies.has(g.libelle) ? [] : g.entrees));
}

function statutGit(p, e) {
  if (!p.git || e.nom === '.git') return null;
  return p.git.statuts[e.nom] || 'a_jour';
}

// ———————————————————————————— Rendu des panneaux ————————————————————————————

function renderStructure() {
  const zone = $('#panneaux');
  zone.classList.toggle('deux', nbPanneaux() === 2);
  zone.innerHTML = Array.from({ length: nbPanneaux() }, (_, i) => `<section class="panneau${i === etat.actif ? ' actif' : ''}" data-i="${i}" aria-label="Panneau ${i + 1}"></section>`).join('');
  if (etat.actif >= nbPanneaux()) etat.actif = 0;
  for (let i = 0; i < nbPanneaux(); i++) renderPanneau(i);
}

function renderTous() {
  for (let i = 0; i < nbPanneaux(); i++) renderPanneau(i);
}

function renderPanneau(i) {
  const sec = document.querySelector(`.panneau[data-i="${i}"]`);
  if (!sec) return;
  const p = panneau(i);
  // Ne pas détruire la ligne pendant un glisser-déposer ou un renommage en cours
  if (etat.glisse) {
    etat.renduDiffere.add(i);
    return;
  }
  const saisie = document.activeElement;
  if (p.renommage && saisie?.classList.contains('renommage') && sec.contains(saisie)) return;
  if (p.editionChemin && saisie?.classList.contains('champ-chemin') && sec.contains(saisie)) return;
  if (etat.lasso?.panneau === i) {
    etat.renduDiffere.add(i);
    return;
  }

  const ancienDefilement = sec.querySelector('.defilement');
  const defilement = p.hautDePage ? 0 : ancienDefilement?.scrollTop ?? 0;
  p.hautDePage = false;

  p.visibles = calculerVisibles(i);
  if (estGroupeParDate(i)) p.visibles = grouper(i, p.visibles);
  else p.groupes = null;
  const cols = config.colonnes;
  const avecGit = !!p.git && cols.git;
  // Largeurs des colonnes définies en CSS (--l-…), plus étroites quand le panneau l'est
  const grille = ['minmax(140px, 1fr)', avecGit && 'var(--l-git)', cols.taille && 'var(--l-taille)', cols.modifie && 'var(--l-modifie)', cols.tags && 'var(--l-tags)'].filter(Boolean).join(' ');
  sec.classList.toggle('actif', i === etat.actif);

  sec.innerHTML = `
    ${htmlOnglets(i)}
    ${htmlOutils(i)}
    ${etat.colonnesOuvert === i ? htmlColonnes(i) : ''}
    <div class="zone-liste" ${p.contenu ? `data-depot="${esc(p.contenu.chemin)}"` : ''}>
      <div class="defilement">
        <div class="table" role="grid" aria-label="Contenu du dossier" style="--grille: ${grille}">
          ${htmlEntete(i, avecGit)}
          ${htmlCorps(i, avecGit)}
        </div>
      </div>
    </div>
    ${htmlPied(i)}`;

  const liste = sec.querySelector('.defilement');
  if (liste) liste.scrollTop = defilement;
  const fil = sec.querySelector('.fil');
  if (fil) fil.scrollLeft = fil.scrollWidth;

  if (p.editionChemin) {
    const champ = sec.querySelector('.champ-chemin');
    if (champ && document.activeElement !== champ) {
      champ.focus();
      champ.select();
    }
  }
  if (p.reveler) {
    const idx = p.visibles.findIndex((e) => e.chemin === p.reveler);
    sec.querySelector(`.ligne[data-idx="${idx}"]`)?.scrollIntoView({ block: 'nearest' });
    p.reveler = null;
  }
  if (p.renommage) {
    const champ = sec.querySelector('.renommage');
    if (champ) {
      champ.focus();
      const entree = p.visibles.find((e) => e.chemin === p.renommage);
      const point = champ.value.lastIndexOf('.');
      champ.setSelectionRange(0, point > 0 && !entree?.est_dossier ? point : champ.value.length);
    }
  }
}

function htmlOnglets(i) {
  const p = panneau(i);
  const onglets = p.onglets
    .map((o, k) => {
      const actif = k === p.ongletActif;
      return `<div class="onglet${actif ? ' actif' : ''}">
        <button class="onglet-btn" role="tab" aria-selected="${actif}" data-action="onglet" data-k="${k}" title="${esc(o.chemin)}">
          ${icDossier(couleurDossier(o.chemin), 16, 14)}<span>${esc(nomAffiche(o.chemin))}</span>
        </button>
        ${p.onglets.length > 1 ? `<button class="icone-btn petit" data-action="fermer-onglet" data-k="${k}" aria-label="Fermer l'onglet" title="Fermer l'onglet (Ctrl+W)">${ic('x', 13)}</button>` : ''}
      </div>`;
    })
    .join('');
  return `<div class="onglets" role="tablist" aria-label="Onglets">${onglets}
    <button class="icone-btn" data-action="nouvel-onglet" aria-label="Nouvel onglet" title="Nouvel onglet (Ctrl+T)">${ic('plus', 16)}</button></div>`;
}

function htmlOutils(i) {
  const p = panneau(i);
  const o = ongletCourant(p);
  const segments = p.contenu?.segments ?? [];
  const fil = segments
    .map((s, k) => {
      const dernier = k === segments.length - 1;
      // Le dernier segment (le dossier ouvert) ouvre la saisie du chemin, comme un clic dans la barre
      const action = dernier ? 'editer-chemin' : 'aller';
      return `<button class="segment${dernier ? ' dernier' : ''}" data-action="${action}" data-chemin="${esc(s.chemin)}" data-depot="${esc(s.chemin)}">${esc(s.nom)}</button>${dernier ? '' : `<span class="chevron">${ic('avance', 14)}</span>`}`;
    })
    .join('');
  const g = p.git;
  const puceGit = g
    ? `<span class="puce-git" title="Branche Git en cours">${ic('branche', 14)}<span class="mono">${esc(g.branche || '?')}</span>${g.en_avance ? `<span title="Commits à pousser">↑${g.en_avance}</span>` : ''}${g.en_retard ? `<span title="Commits à récupérer">↓${g.en_retard}</span>` : ''}</span>`
    : '';
  return `<div class="outils">
    <button class="icone-btn" data-action="retour" aria-label="Précédent" title="Précédent (Alt+←)" ${o.pos > 0 ? '' : 'disabled'}>${ic('retour')}</button>
    <button class="icone-btn" data-action="avance" aria-label="Suivant" title="Suivant (Alt+→)" ${o.pos < o.histo.length - 1 ? '' : 'disabled'}>${ic('avance')}</button>
    <button class="icone-btn" data-action="haut" aria-label="Dossier parent" title="Dossier parent (Alt+↑)" ${p.contenu?.parent ? '' : 'disabled'}>${ic('haut')}</button>
    ${
      p.editionChemin
        ? `<input class="champ-chemin" value="${esc(p.valeurChemin ?? dossierCourant(i) ?? '')}" aria-label="Chemin du dossier" spellcheck="false">`
        : `<div class="fil" aria-label="Chemin" title="Cliquez dans la barre pour modifier le chemin (Ctrl+L)">${fil}</div>`
    }
    ${puceGit}
    <button class="bascule${config.afficherCaches ? ' on' : ''}" data-action="caches" aria-pressed="${config.afficherCaches}" title="Fichiers cachés (Ctrl+H)">${ic('oeil', 17)}<span>Cachés</span></button>
    <button class="texte-btn" data-action="colonnes" aria-expanded="${etat.colonnesOuvert === i}" title="Choisir les colonnes">${ic('colonnes', 17)}<span>Colonnes</span></button>
    <button class="icone-btn" data-action="terminal-ici" aria-label="Ouvrir un terminal ici" title="Ouvrir un terminal ici">${ic('terminal')}</button>
    <button class="icone-btn" data-action="vscode-ici" aria-label="Ouvrir dans VS Code" title="Ouvrir dans VS Code">${ic('code')}</button>
  </div>`;
}

function htmlColonnes(i) {
  const p = panneau(i);
  const options = [
    ['git', 'État Git', p.git ? '' : 'dépôts seulement'],
    ['taille', 'Taille', ''],
    ['modifie', 'Date de modification', ''],
    ['tags', 'Tags', ''],
  ];
  return `<div class="popover" role="dialog" aria-label="Colonnes affichées">
    <h3>Colonnes affichées</h3>
    ${options.map(([id, libelle, note]) => `<label class="case"><input type="checkbox" data-action="colonne" data-col="${id}" ${config.colonnes[id] ? 'checked' : ''}>${libelle}${note ? `<small>${note}</small>` : ''}</label>`).join('')}
  </div>`;
}

function htmlEntete(i, avecGit) {
  const p = panneau(i);
  const cols = config.colonnes;
  const groupe = estGroupeParDate(i);
  const tri = (col, libelle) => {
    const actif = groupe ? col === 'modifie' : p.tri.col === col;
    const fleche = actif ? (groupe || p.tri.sens === -1 ? ' ↓' : ' ↑') : '';
    return `<button class="tri${actif ? ' actif' : ''}" data-action="trier" data-col="${col}">${libelle}${fleche}</button>`;
  };
  return `<div class="entete" role="row">
    <span>${tri('nom', 'Nom')}</span>
    ${avecGit ? '<span>Git</span>' : ''}
    ${cols.taille ? `<span>${tri('taille', 'Taille')}</span>` : ''}
    ${cols.modifie ? `<span>${tri('modifie', 'Modifié')}</span>` : ''}
    ${cols.tags ? '<span>Tags</span>' : ''}
  </div>`;
}

function icone(e) {
  if (e.est_dossier) return icDossierSpecial(couleurDossier(e.chemin, e), dossierSpecial(e.chemin), 26, 22);
  // « .env », « .gitignore » : pas d'extension, on affiche le nom (ENV, GITI…)
  return icFichier(typeFichier(e.extension || (e.nom.startsWith('.') ? e.nom.slice(1).toLowerCase() : '')));
}

function pastilleGit(statut) {
  const g = GIT[statut];
  return `<span class="pastille" style="background:${g.fond};color:${g.texte}"><i style="background:${g.point}"></i>${g.libelle}</span>`;
}

function pastilleTag(id) {
  const t = trouverTag(id);
  if (!t) return '';
  return `<span class="pastille" title="${t.libelle}" style="background:${t.fond};color:${t.texte}"><i style="background:${t.couleur}"></i><span class="txt">${t.libelle}</span></span>`;
}

function htmlCorps(i, avecGit) {
  const p = panneau(i);
  if (p.erreur) {
    return `<div class="vide"><p>${esc(p.erreur)}</p><button data-action="aller" data-chemin="${esc(standards.accueil)}">Revenir à l'accueil</button></div>`;
  }
  if (!p.contenu) return '<div class="vide">Chargement…</div>';
  if (!p.visibles.length) {
    let message = 'Ce dossier est vide. Faites un clic droit pour créer un dossier ou un fichier.';
    if (etat.recherche && etat.recherchePanneau === i) message = `Aucun élément ne contient « ${esc(etat.recherche)} ».`;
    else if (etat.filtreTag) message = `Aucun élément avec le tag « ${esc(trouverTag(etat.filtreTag)?.libelle)} » ici.`;
    else if (p.contenu.entrees.length) message = 'Ce dossier ne contient que des fichiers cachés. Affichez-les avec Ctrl+H.';
    return `<div class="vide">${message}</div>`;
  }
  const cols = config.colonnes;
  const coupes = etat.pressePapiers?.mode === 'couper' ? new Set(etat.pressePapiers.chemins) : null;
  const ligne = (e, idx) => {
      const classes = ['ligne'];
      if (p.selection.has(e.chemin)) classes.push('sel');
      if (coupes?.has(e.chemin)) classes.push('coupe');
      if (e.cache) classes.push('cache');
      const g = avecGit ? statutGit(p, e) : null;
      const nom =
        p.renommage === e.chemin
          ? `<input class="renommage" value="${esc(e.nom)}" aria-label="Nouveau nom" spellcheck="false">`
          : `<span class="nom">${esc(e.nom)}</span>`;
      return `<div class="${classes.join(' ')}" role="row" aria-selected="${p.selection.has(e.chemin)}" data-idx="${idx}" ${e.est_dossier ? `data-depot="${esc(e.chemin)}"` : ''}>
        <span class="c-nom"><span class="poignee" draggable="${p.renommage === e.chemin ? 'false' : 'true'}">${icone(e)}${nom}</span>${e.cache ? '<span class="pastille-cache">caché</span>' : ''}${e.est_lien ? '<span class="pastille-cache" title="Lien symbolique">lien</span>' : ''}</span>
        ${avecGit ? `<span>${g ? pastilleGit(g) : '<span class="tiret">—</span>'}</span>` : ''}
        ${cols.taille ? `<span class="c-num">${e.est_dossier ? elements(e.nb_elements) : taille(e.taille)}</span>` : ''}
        ${cols.modifie ? `<span class="c-num" title="${esc(dateComplete(e.modifie))}">${dateRelative(e.modifie)}</span>` : ''}
        ${cols.tags ? `<span class="c-tags">${(config.tags[e.chemin] || []).map(pastilleTag).join('')}</span>` : ''}
      </div>`;
  };
  if (!p.groupes) return p.visibles.map(ligne).join('');
  let idx = 0;
  return p.groupes
    .map((g) => {
      const replie = p.replies.has(g.libelle);
      const titre = `<button class="groupe${replie ? ' replie' : ''}" data-action="groupe" data-libelle="${esc(g.libelle)}" aria-expanded="${!replie}">
        ${ic('avance', 14, 'class="chevron-groupe"')}<span>${esc(g.libelle)}</span><span class="groupe-compte">${g.entrees.length}</span></button>`;
      return titre + (replie ? '' : g.entrees.map((e) => ligne(e, idx++)).join(''));
    })
    .join('');
}

function htmlPied(i) {
  const p = panneau(i);
  if (!p.contenu) return '<div class="pied"></div>';
  const total = p.visibles.length;
  const caches = config.afficherCaches ? 0 : p.contenu.entrees.filter((e) => e.cache).length;
  let texte = pluriel(total, 'élément', 'éléments');
  if (p.selection.size) texte += `, ${pluriel(p.selection.size, 'sélectionné', 'sélectionnés')}`;
  if (caches) texte += `, ${pluriel(caches, 'caché', 'cachés')}`;
  let resume = '';
  if (p.git) {
    const statuts = Object.values(p.git.statuts);
    const mod = statuts.filter((s) => s === 'modifie').length;
    const nouv = statuts.filter((s) => s === 'nouveau').length;
    const conf = statuts.filter((s) => s === 'conflit').length;
    const parts = [];
    if (conf) parts.push(pluriel(conf, 'conflit', 'conflits'));
    if (mod) parts.push(pluriel(mod, 'modifié', 'modifiés'));
    if (nouv) parts.push(pluriel(nouv, 'nouveau', 'nouveaux'));
    resume = parts.length ? parts.join(', ') : 'Tout est à jour';
  }
  return `<div class="pied"><span>${texte}</span>${resume ? `<span class="git-resume">${resume}</span>` : ''}</div>`;
}

// ———————————————————————————— Barre latérale ————————————————————————————

/** Dossiers de base du système, dans l'ordre de Nautilus. */
function dossiersDeBase() {
  const s = standards;
  return [
    { chemin: s.accueil, libelle: 'Accueil', icone: 'maison', couleur: '#2563EB' },
    s.bureau && { chemin: s.bureau, libelle: 'Bureau', icone: 'ecran', couleur: '#0891B2' },
    s.documents && { chemin: s.documents, libelle: 'Documents', icone: 'document', couleur: '#7C3AED' },
    s.telechargements && { chemin: s.telechargements, libelle: 'Téléchargements', icone: 'telecharger', couleur: '#16A34A' },
    s.images && { chemin: s.images, libelle: 'Images', icone: 'image', couleur: '#DB2777' },
    s.musique && { chemin: s.musique, libelle: 'Musique', icone: 'musique', couleur: '#E11D48' },
    s.videos && { chemin: s.videos, libelle: 'Vidéos', icone: 'video', couleur: '#9333EA' },
  ].filter(Boolean);
}

function badgeFichier(nom, extension) {
  return icFichier(typeFichier(extension || (nom.startsWith('.') ? nom.slice(1).toLowerCase() : '')), 18, 21);
}

function extensionDe(chemin) {
  const nom = nomDe(chemin);
  const point = nom.lastIndexOf('.');
  return point > 0 ? nom.slice(point + 1).toLowerCase() : '';
}

function parentDe(chemin) {
  const i = Math.max(chemin.lastIndexOf('/'), chemin.lastIndexOf('\\'));
  if (i <= 0) return SEP === '/' ? '/' : chemin;
  const parent = chemin.slice(0, i);
  return /^[a-zA-Z]:$/.test(parent) ? parent + SEP : parent;
}

// ———————————————————————————— Jeux (launchers détectés) ————————————————————————————

const COULEURS_LANCEUR = { steam: '#1E40AF', battlenet: '#0EA5E9', ubisoft: '#2563EB', epic: '#334155', ea: '#DC2626', gog: '#7C3AED' };

function iconeApplication(exe, repli, taille = 20) {
  const image = exe && etat.iconesApps[exe];
  if (image) return `<img class="ic-app" src="${image}" width="${taille}" height="${taille}" alt="">`;
  return repli;
}

function htmlJeux() {
  if (!etat.lanceurs.length) return '';
  const lignes = etat.lanceurs
    .map((l) => {
      const couleur = COULEURS_LANCEUR[l.id] || '#475569';
      const repli = `<span class="ic-lettre" style="background:${couleur}">${esc(l.nom[0])}</span>`;
      const plusieurs = l.jeux.length > 1;
      const deplie = plusieurs && config.jeuxDeplies[l.id];
      const titre = l.jeux.length === 1 ? l.jeux[0].dossier : plusieurs ? `${l.jeux.length} jeux` : l.dossier;
      let html = `<button class="epingle lanceur" data-action="lanceur" data-id="${esc(l.id)}" data-lanceur="${esc(l.id)}" ${plusieurs ? `aria-expanded="${!!deplie}"` : ''} title="${esc(titre)}">
        ${iconeApplication(l.executable, repli)}<span>${esc(l.nom)}</span>
        ${plusieurs ? `<span class="compte">${l.jeux.length}</span>${ic('avance', 14, `class="chevron${deplie ? ' ouvert' : ''}"`)}` : ''}
      </button>`;
      if (deplie) {
        html += l.jeux
          .map((j) => `<button class="epingle jeu" data-action="aller" data-chemin="${esc(j.dossier)}" data-jeu="${esc(j.dossier)}" title="${esc(j.dossier)}">${iconeApplication(j.executable, icDossier(couleur, 18, 15), 16)}<span>${esc(j.nom)}</span></button>`)
          .join('');
      }
      return html;
    })
    .join('');
  return `<section aria-label="Jeux"><div class="titre-bloc"><span>Jeux</span></div>${lignes}</section>`;
}

/** Un seul jeu : on va dans son dossier. Plusieurs : on déplie la liste. Aucun : dossier du launcher. */
function cliquerLanceur(id) {
  const l = etat.lanceurs.find((x) => x.id === id);
  if (!l) return;
  if (l.jeux.length === 1) return naviguer(etat.actif, l.jeux[0].dossier);
  if (l.jeux.length > 1) {
    config.jeuxDeplies[id] = !config.jeuxDeplies[id];
    sauver();
    return renderBarre();
  }
  naviguer(etat.actif, l.dossier);
}

async function chargerJeux() {
  try {
    etat.lanceurs = await api.jeux();
  } catch {
    etat.lanceurs = [];
  }
  if (!etat.lanceurs.length) return;
  renderBarre();
  const exes = etat.lanceurs.flatMap((l) => [l.executable, ...l.jeux.map((j) => j.executable)]).filter(Boolean);
  if (!exes.length) return;
  try {
    etat.iconesApps = await api.iconesApplications(exes);
    renderBarre();
  } catch {
    /* pas d'icône : la lettre de couleur reste affichée */
  }
}

function menuLanceur(id, x, y) {
  const l = etat.lanceurs.find((x) => x.id === id);
  if (!l) return;
  let html = `<div class="menu-titre">${esc(l.nom)}</div>`;
  if (l.executable) html += item('lancer-lanceur', 'ouvrir', `Lancer ${l.nom}`);
  html += item('dossier-lanceur', 'dossier', `Dossier de ${l.nom}`);
  html += item('onglet-lanceur', 'onglet', 'Ouvrir le dossier dans un nouvel onglet');
  ouvrirMenu(x, y, html, { type: 'lanceur', dossier: l.dossier, executable: l.executable });
}

function menuJeu(chemin, x, y) {
  let html = `<div class="menu-titre">${esc(nomDe(chemin))}</div>`;
  html += item('dossier-lanceur', 'dossier', 'Ouvrir le dossier');
  html += item('onglet-lanceur', 'onglet', 'Ouvrir dans un nouvel onglet');
  html += item('copier-chemin-jeu', 'lien', 'Copier le chemin');
  ouvrirMenu(x, y, html, { type: 'jeu', dossier: chemin });
}

function compteTag(id) {
  return Object.values(config.tags).filter((l) => l.includes(id)).length;
}

function renderBarre() {
  const e = espace();
  const epingles = e.epingles
    .map((ep) =>
      ep.dossier
        ? `<button class="epingle" data-action="aller" data-chemin="${esc(ep.chemin)}" data-depot="${esc(ep.chemin)}" data-epingle="${esc(ep.chemin)}" title="${esc(ep.chemin)}">${icDossier(couleurDossier(ep.chemin), 22, 18)}<span>${esc(nomAffiche(ep.chemin))}</span></button>`
        : `<button class="epingle" data-action="ouvrir-fichier" data-chemin="${esc(ep.chemin)}" data-epingle="${esc(ep.chemin)}" title="${esc(ep.chemin)}">${badgeFichier(nomDe(ep.chemin), extensionDe(ep.chemin))}<span>${esc(nomDe(ep.chemin))}</span></button>`
    )
    .join('');
  const base = dossiersDeBase()
    .map((f) => `<button class="epingle base" data-action="aller" data-chemin="${esc(f.chemin)}" data-depot="${esc(f.chemin)}" title="${esc(f.chemin)}">${ic(f.icone, 19, `style="color:${f.couleur}"`)}<span>${f.libelle}</span></button>`)
    .join('');
  const jeux = htmlJeux();
  const tags = listeTags()
    .map((t) => {
      const on = etat.filtreTag === t.id;
      return `<button class="tag-btn${on ? ' on' : ''}" data-action="filtre-tag" data-id="${esc(t.id)}" data-tag="${esc(t.id)}" aria-pressed="${on}" style="--c:${t.couleur}"><span class="point"></span><span class="tag-nom">${esc(t.libelle)}</span><span class="compte">${compteTag(t.id) || ''}</span></button>`;
    })
    .join('');
  const espaces = config.espaces
    .map((s) => {
      const c = PALETTE[s.couleur] || PALETTE.violet;
      const on = s.id === e.id;
      return `<button class="pastille-espace${on ? ' on' : ''}" data-action="espace" data-id="${esc(s.id)}" aria-pressed="${on}" aria-label="Espace ${esc(s.nom)}" title="${esc(s.nom)}" style="--c:${c.couleur};--e:${c.encre}">${ic(s.icone, 18)}</button>`;
    })
    .join('');

  $('#barre-contenu').innerHTML = `
    <section aria-label="Espace ${esc(e.nom)}">
      <div class="espace-tete">
        <span class="espace-pastille">${ic(e.icone, 17)}</span>
        <h2>${esc(e.nom)}</h2>
        <button class="icone-btn" data-action="modifier-espace" aria-label="Modifier l'espace" title="Modifier l'espace">${ic('points', 18)}</button>
      </div>
      ${epingles}
      <button class="epingle discret" data-action="epingler">${ic('plus', 18)}<span>Épingler le dossier ouvert</span></button>
    </section>
    <section aria-label="Dossiers">
      <div class="titre-bloc"><span>Dossiers</span></div>
      ${base}
    </section>
    ${jeux}
    <section aria-label="Tags">
      <div class="titre-bloc"><span>Tags</span>
        <span class="titre-actions">${etat.filtreTag ? '<button class="lien-btn" data-action="filtre-tag" data-id="">Tout afficher</button>' : ''}
        <button class="icone-btn mini" data-action="nouveau-tag" aria-label="Nouveau tag" title="Nouveau tag">${ic('plus', 16)}</button></span>
      </div>
      ${tags}
    </section>
`;
  // Toujours visibles en bas, même quand la liste du haut défile
  const sombre = config.theme === 'sombre';
  const boutonTheme = `<button class="icone-btn mini" data-action="theme" aria-label="${sombre ? 'Passer en mode clair' : 'Passer en mode sombre'}"
    title="${sombre ? 'Mode clair' : 'Mode sombre'}">${ic(sombre ? 'soleil' : 'lune', 15)}</button>`;
  const m = etat.miseAJour;
  // Bouton de mise à jour (si disponible), puis la ligne version + mode sombre + vérification
  const boutonMaj = m
    ? `<button class="bouton-maj${etat.majAnimee ? '' : ' apparait'}" data-action="mettre-a-jour" ${etat.installationEnCours ? 'disabled' : ''} title="${esc(m.notes || '')}">
        ${ic('telecharger', 17)}<span>${etat.installationEnCours ? 'Installation…' : `Mettre à jour vers ${esc(m.version)}`}</span></button>`
    : '';
  const boutonVerif = m
    ? ''
    : `<button class="icone-btn mini verif-maj${etat.verificationEnCours ? ' tourne' : ''}${etat.majAJour ? ' ok' : ''}" data-action="verifier-maj"
        aria-label="Rechercher une mise à jour" title="Rechercher une mise à jour">${ic(etat.majAJour ? 'coche' : 'actualiser', 15)}</button>`;
  const blocVersion = `${boutonMaj}<div class="ligne-version"><button class="version" data-action="nouveautes" title="Voir les nouveautés">${etat.version ? `Ariane ${esc(etat.version)}` : 'Ariane'}</button>${boutonTheme}${boutonVerif}</div>`;
  $('#barre-pied').innerHTML = `
    ${blocVersion}
    <button class="epingle" data-action="diviser">${ic('diviser', 18)}<span>${config.deuxPanneaux ? 'Un seul panneau' : 'Deux panneaux'}</span></button>
    <div class="espaces" role="group" aria-label="Espaces">${espaces}
      <button class="pastille-espace ajout" data-action="nouvel-espace" aria-label="Nouvel espace" title="Nouvel espace">${ic('plus', 18)}</button>
    </div>`;
}

// ———————————————————————————— Sélection ————————————————————————————

function cliquerLigne(i, idx, ev) {
  const p = panneau(i);
  const e = p.visibles[idx];
  if (!e) return;
  if (ev.shiftKey && p.ancre) {
    const a = p.visibles.findIndex((x) => x.chemin === p.ancre);
    const [debut, fin] = a < idx ? [a, idx] : [idx, a];
    if (!(ev.ctrlKey || ev.metaKey)) p.selection.clear();
    for (let k = Math.max(0, debut); k <= fin; k++) p.selection.add(p.visibles[k].chemin);
  } else if (ev.ctrlKey || ev.metaKey) {
    if (p.selection.has(e.chemin)) p.selection.delete(e.chemin);
    else p.selection.add(e.chemin);
    p.ancre = e.chemin;
  } else {
    p.selection = new Set([e.chemin]);
    p.ancre = e.chemin;
  }
  majSelection(i);
}

/** Met à jour la surbrillance sans recréer les lignes (sinon le double-clic est perdu). */
function majSelection(i) {
  const sec = document.querySelector(`.panneau[data-i="${i}"]`);
  if (!sec) return;
  const p = panneau(i);
  sec.querySelectorAll('.ligne').forEach((ligne) => {
    const e = p.visibles[+ligne.dataset.idx];
    const on = !!e && p.selection.has(e.chemin);
    ligne.classList.toggle('sel', on);
    ligne.setAttribute('aria-selected', String(on));
  });
  const pied = sec.querySelector('.pied');
  if (pied) pied.outerHTML = htmlPied(i);
}

function deplacerSelection(i, delta, etendre) {
  const p = panneau(i);
  const v = p.visibles;
  if (!v.length) return;
  const actuel = v.findIndex((x) => x.chemin === p.ancre);
  const n = actuel < 0 ? (delta > 0 ? 0 : v.length - 1) : Math.min(v.length - 1, Math.max(0, actuel + delta));
  if (etendre) p.selection.add(v[n].chemin);
  else p.selection = new Set([v[n].chemin]);
  p.ancre = v[n].chemin;
  p.reveler = v[n].chemin;
  renderPanneau(i);
}

function selectionnes(i = etat.actif) {
  return [...panneau(i).selection];
}

function entreesSelectionnees(i = etat.actif) {
  const p = panneau(i);
  return (p.contenu?.entrees ?? []).filter((e) => p.selection.has(e.chemin));
}

// ———————————————————————————— Actions sur les fichiers ————————————————————————————

function ouvrirEntree(i, e, { nouvelOngletDossier = false } = {}) {
  if (e.est_dossier) {
    if (nouvelOngletDossier) nouvelOnglet(i, e.chemin);
    else naviguer(i, e.chemin);
  } else {
    essayer(api.ouvrir(e.chemin));
  }
}

function ouvrirSelection(i) {
  const liste = entreesSelectionnees(i);
  const dossiers = liste.filter((e) => e.est_dossier);
  // Un seul dossier : on y va. Plusieurs : chacun dans un onglet.
  if (dossiers.length === 1 && liste.length === 1) return ouvrirEntree(i, dossiers[0]);
  dossiers.forEach((d) => nouvelOnglet(i, d.chemin));
  liste.filter((e) => !e.est_dossier).forEach((f) => essayer(api.ouvrir(f.chemin)));
}

function mettreDansPressePapiers(i, mode) {
  const chemins = selectionnes(i);
  if (!chemins.length) return;
  etat.pressePapiers = { mode, chemins };
  const verbe = mode === 'couper' ? 'coupé' : 'copié';
  notifier(`${libelleElements(chemins)} ${verbe}${chemins.length > 1 ? 's' : ''}. Collez avec Ctrl+V.`);
  renderTous();
}

async function collerIci(i, destination = dossierCourant(i)) {
  const pp = etat.pressePapiers;
  if (!pp || !destination) return;
  if (pp.mode === 'couper') etat.pressePapiers = null; // un élément coupé ne se colle qu'une fois
  // Collage annulé depuis la fenêtre de conflit : on peut recoller
  if (!(await collerVers(pp.chemins, destination, pp.mode))) etat.pressePapiers = pp;
}

/** Copie ou déplace des éléments. Le glisser-déposer passe aussi par ici (mode « couper »).
 *  Renvoie false si l'utilisateur a annulé depuis la fenêtre de conflit. */
async function collerVers(chemins, destination, mode) {
  if (!chemins.length) return true;
  let r, choix;
  try {
    choix = await choisirConflits(chemins, destination, mode);
    if (!choix) return false;
    r = await api.coller(chemins, destination, mode, choix);
  } catch (e) {
    erreur(e);
    rafraichirTout();
    return true;
  }
  if (mode === 'couper') deplacerReglages(r.operations);
  if (r.erreurs.length) notifier(r.erreurs.join('\n'), { erreur: true });
  const ignores = Object.values(choix).filter((c) => c === 'ignorer').length;
  if (!r.operations.length && ignores) notifier(`${pluriel(ignores, 'élément ignoré', 'éléments ignorés')} : rien n'a été collé.`);
  if (r.operations.length) {
    const cibles = r.operations.map((o) => o.cible);
    // Les éléments collés sont sélectionnés là où ils arrivent
    etat.panneaux.forEach((p) => {
      if (p.contenu?.chemin === destination) {
        p.selection = new Set(cibles);
        p.ancre = cibles[0];
        p.reveler = cibles[0];
      }
    });
    const verbe = mode === 'couper' ? 'déplacé' : 'copié';
    const accord = r.operations.length > 1 ? 's' : '';
    const annuler = async () => {
      await essayer(api.annulerCollage(r.operations, mode));
      if (mode === 'couper') deplacerReglages(r.operations.map((o) => ({ source: o.cible, cible: o.source })));
      rafraichirTout();
    };
    etat.annulation = annuler;
    const remplaces = r.operations.filter((o) => choix[o.source] === 'remplacer').length;
    const details = [remplaces && pluriel(remplaces, 'remplacé', 'remplacés'), ignores && pluriel(ignores, 'ignoré', 'ignorés')].filter(Boolean);
    notifier(`${libelleElements(r.operations.map((o) => o.source))} ${verbe}${accord} vers « ${nomAffiche(destination)} »${details.length ? ` (${details.join(', ')})` : ''}`, { annuler });
  }
  rafraichirTout();
  return true;
}

/** Demande quoi faire de chaque nom déjà pris dans `destination`.
 *  Renvoie { source: 'remplacer' | 'garder' | 'ignorer' }, ou null si le collage est annulé. */
async function choisirConflits(chemins, destination, mode) {
  const conflits = await api.conflits(chemins, destination);
  const choix = {};
  let pourTous = null;
  for (const [n, c] of conflits.entries()) {
    const reponse = pourTous ? { choix: pourTous } : await demanderConflit(c, n, conflits.length, mode, destination);
    if (!reponse) return null;
    if (reponse.tous) pourTous = reponse.choix;
    choix[c.source] = reponse.choix;
  }
  return choix;
}

/** Les tags, couleurs et épingles suivent les éléments déplacés ou renommés. */
function deplacerReglages(operations) {
  if (!operations.length) return;
  const corriger = (c) => {
    for (const o of operations) {
      if (c === o.source) return o.cible;
      if (estSous(c, o.source)) return o.cible + c.slice(o.source.length);
    }
    return c;
  };
  // Un élément qui en remplace un autre apporte ses réglages ; sans réglages, il garde ceux de l'ancien
  const corrigerCles = (objet) => {
    const restes = {};
    const deplaces = {};
    for (const [k, v] of Object.entries(objet)) {
      const c = corriger(k);
      (c === k ? restes : deplaces)[c] = v;
    }
    return { ...restes, ...deplaces };
  };
  config.tags = corrigerCles(config.tags);
  config.couleursDossiers = corrigerCles(config.couleursDossiers);
  for (const e of config.espaces) e.epingles = e.epingles.map((ep) => ({ ...ep, chemin: corriger(ep.chemin) }));
  for (const p of etat.panneaux)
    for (const o of p.onglets) {
      o.chemin = corriger(o.chemin);
      o.histo = o.histo.map(corriger);
    }
  sauver();
  renderBarre();
}

async function mettreALaCorbeille(i) {
  const chemins = selectionnes(i);
  if (!chemins.length) return;
  try {
    await api.corbeille(chemins);
    notifier(`${libelleElements(chemins)} mis à la corbeille`);
  } catch (e) {
    erreur(e);
  }
  rafraichirTout();
}

function commencerRenommage(i) {
  const p = panneau(i);
  if (p.selection.size !== 1) return;
  p.renommage = selectionnes(i)[0];
  p.reveler = p.renommage;
  renderPanneau(i);
}

async function terminerRenommage(i, champ, annuler = false) {
  const p = panneau(i);
  const chemin = p.renommage;
  if (!chemin) return;
  p.renommage = null;
  const nouveau = champ.value.trim();
  if (annuler || !nouveau || nouveau === nomDe(chemin)) {
    renderPanneau(i);
    return;
  }
  try {
    const cible = await api.renommer(chemin, nouveau);
    deplacerReglages([{ source: chemin, cible }]);
    etat.panneaux.forEach((q) => {
      if (q.selection.has(chemin)) {
        q.selection = new Set([cible]);
        q.ancre = cible;
      }
    });
  } catch (e) {
    erreur(e);
  }
  rafraichirTout();
}

async function creer(i, type, nom) {
  const dossier = dossierCourant(i);
  if (!dossier) return;
  const chemin = await essayer(type === 'dossier' ? api.creerDossier(dossier, nom) : api.creerFichier(dossier, nom));
  if (!chemin) return;
  const p = panneau(i);
  p.selection = new Set([chemin]);
  p.ancre = chemin;
  p.renommage = chemin; // on propose tout de suite de le nommer
  p.reveler = chemin;
  rafraichirTout();
}

function basculerCaches() {
  config.afficherCaches = !config.afficherCaches;
  sauver();
  renderTous();
}

function basculerTag(chemins, id) {
  const tousLOnt = chemins.every((c) => (config.tags[c] || []).includes(id));
  for (const c of chemins) {
    const tags = new Set(config.tags[c] || []);
    if (tousLOnt) tags.delete(id);
    else tags.add(id);
    if (tags.size) config.tags[c] = [...tags];
    else delete config.tags[c];
  }
  sauver();
  renderBarre();
  renderTous();
}

function colorerDossiers(chemins, couleur) {
  for (const c of chemins) {
    if (couleur) config.couleursDossiers[c] = couleur;
    else delete config.couleursDossiers[c];
  }
  sauver();
  renderBarre();
  renderTous();
}

/** Épingle un ou plusieurs éléments (dossiers ou fichiers) dans l'espace actif. */
function epingler(elements) {
  const liste = (Array.isArray(elements) ? elements : [elements]).filter((x) => x?.chemin);
  if (!liste.length) return;
  const e = espace();
  for (const x of liste) {
    if (!e.epingles.some((ep) => ep.chemin === x.chemin)) e.epingles.push({ chemin: x.chemin, dossier: !!x.dossier });
  }
  sauver();
  renderBarre();
  const nom = liste.length === 1 ? `« ${nomAffiche(liste[0].chemin)} »` : `${liste.length} éléments`;
  notifier(`${nom} épinglé${liste.length > 1 ? 's' : ''} dans ${e.nom}`);
}

// ——— Actions selon le type ———

/** Sélectionne des éléments créés, avec une notification annulable (mise à la corbeille). */
function montrerCrees(i, crees, message) {
  const p = panneau(i);
  p.selection = new Set(crees);
  p.ancre = crees[0];
  p.reveler = crees[0];
  const annuler = async () => {
    await essayer(api.corbeille(crees));
    rafraichirTout();
  };
  etat.annulation = annuler;
  notifier(message, { annuler });
  rafraichirTout();
}

async function extraire(i, e, mode) {
  notifier(`Extraction de « ${e.nom} »…`);
  const crees = await essayer(api.extraireArchive(e.chemin, mode));
  if (!crees) return;
  const ou = mode === 'dossier' ? `dans « ${nomDe(crees[0])} »` : 'ici';
  montrerCrees(i, crees, `« ${e.nom} » extrait ${ou}`);
}

async function compresserSelection(i, chemins) {
  const dossier = dossierCourant(i);
  if (!dossier || !chemins.length) return;
  notifier('Compression en cours…');
  const zip = await essayer(api.compresser(chemins, dossier));
  if (zip) montrerCrees(i, [zip], `« ${nomDe(zip)} » créé`);
}

async function calculerTaille(e) {
  notifier(`Calcul de la taille de « ${e.nom} »…`);
  const t = await essayer(api.tailleDossier(e.chemin));
  if (!t) return;
  notifier(`« ${e.nom} » : ${taille(t.octets)}, ${pluriel(t.fichiers, 'fichier', 'fichiers')} et ${pluriel(t.dossiers, 'dossier', 'dossiers')}`);
}

async function copierContenu(e) {
  const texte = await essayer(api.lireTexte(e.chemin));
  if (texte != null) copierTexte(texte, `Contenu de « ${e.nom} » copié`);
}

async function ouvrirDepot(i) {
  const adresse = await api.adresseDepot(dossierCourant(i)).catch(() => null);
  if (adresse) essayer(api.ouvrir(adresse));
  else notifier('Ce dépôt n’a pas d’adresse en ligne (remote « origin »).', { erreur: true });
}

async function copierTexte(texte, message = 'Chemin copié') {
  try {
    await navigator.clipboard.writeText(texte);
  } catch {
    const zone = document.createElement('textarea');
    zone.value = texte;
    document.body.appendChild(zone);
    zone.select();
    document.execCommand('copy');
    zone.remove();
  }
  notifier(message);
}

function viderRecherche() {
  etat.recherche = '';
  const champ = $('#champ-recherche');
  if (champ) champ.value = '';
}

// ———————————————————————————— Recherche dans tous les dossiers ————————————————————————————

const rechercheGlobale = { jeton: 0, minuterie: null, resultats: [], index: 0, ouvert: false, complet: true, requete: '', enCours: false };

function estChemin(texte) {
  return /^(\/|~|[a-zA-Z]:[\\/])/.test(texte);
}

function planifierRecherche(texte) {
  clearTimeout(rechercheGlobale.minuterie);
  const q = texte.trim();
  if (q.length < 2 || estChemin(q)) {
    rechercheGlobale.jeton++;
    rechercheGlobale.resultats = [];
    return fermerSuggestions();
  }
  rechercheGlobale.minuterie = setTimeout(() => lancerRecherche(q), 180);
}

/** Où chercher : l'accueil d'abord, puis les dossiers ouverts, les épingles et les jeux
 * (ils sont souvent hors du dossier personnel, comme C:\\Program Files (x86)\\World of Warcraft). */
function racinesRecherche() {
  const liste = [standards.accueil];
  for (let i = 0; i < nbPanneaux(); i++) if (dossierCourant(i)) liste.push(dossierCourant(i));
  for (const e of config.espaces) for (const ep of e.epingles) if (ep.dossier) liste.push(ep.chemin);
  for (const l of etat.lanceurs) {
    liste.push(l.dossier);
    for (const j of l.jeux) liste.push(j.dossier);
  }
  return [...new Set(liste)];
}

async function lancerRecherche(q) {
  const jeton = ++rechercheGlobale.jeton;
  rechercheGlobale.requete = q;
  rechercheGlobale.enCours = true;
  rechercheGlobale.ouvert = true;
  renderSuggestions();
  try {
    const r = await api.chercher(racinesRecherche(), q);
    if (jeton !== rechercheGlobale.jeton) return;
    rechercheGlobale.resultats = r.resultats;
    rechercheGlobale.complet = r.complet;
    rechercheGlobale.index = 0;
  } catch (e) {
    if (jeton !== rechercheGlobale.jeton) return;
    rechercheGlobale.resultats = [];
    erreur(e);
  }
  rechercheGlobale.enCours = false;
  if (document.activeElement === $('#champ-recherche')) renderSuggestions();
}

/** « ~/work/copernic » plutôt que « /home/stephane/work/copernic ». */
function cheminCourt(chemin) {
  const a = standards.accueil;
  if (chemin === a) return '~';
  return estSous(chemin, a) ? `~${SEP}${chemin.slice(a.length).replace(/^[\\/]+/, '')}` : chemin;
}

/** Met en évidence la partie du nom qui correspond à la recherche. */
function surlignerNom(nom, q) {
  const i = normaliser(nom).indexOf(normaliser(q));
  if (i < 0 || normaliser(nom).length !== nom.length) return esc(nom);
  return `${esc(nom.slice(0, i))}<mark>${esc(nom.slice(i, i + q.length))}</mark>${esc(nom.slice(i + q.length))}`;
}

function renderSuggestions() {
  const liste = $('#suggestions');
  const champ = $('#champ-recherche');
  const r = rechercheGlobale;
  liste.hidden = !r.ouvert;
  champ.setAttribute('aria-expanded', String(r.ouvert));
  if (!r.ouvert) return;
  const items = r.resultats
    .map((t, k) => {
      const icone = t.est_dossier
        ? icDossierSpecial(couleurDossier(t.chemin), dossierSpecial(t.chemin), 26, 22)
        : (() => {
            return icFichier(typeFichier(t.extension || (t.nom.startsWith('.') ? t.nom.slice(1).toLowerCase() : '')));
          })();
      return `<div class="sugg${k === r.index ? ' on' : ''}" role="option" aria-selected="${k === r.index}" data-k="${k}">
        ${icone}
        <span class="sugg-textes"><span class="sugg-nom">${surlignerNom(t.nom, r.requete)}</span><span class="sugg-chemin">${esc(cheminCourt(t.parent))}</span></span>
      </div>`;
    })
    .join('');
  let info = '';
  if (r.enCours && !r.resultats.length) info = 'Recherche en cours…';
  else if (!r.enCours && !r.resultats.length) info = `Aucun fichier ni dossier ne contient « ${esc(r.requete)} » dans le dossier personnel, les dossiers ouverts, les épingles et les jeux.`;
  else if (!r.complet) info = 'Recherche interrompue au bout de 1,5 s : précisez le nom si l’élément n’apparaît pas.';
  liste.innerHTML = `${items}${info ? `<div class="sugg-info">${info}</div>` : ''}`;
  liste.querySelector('.sugg.on')?.scrollIntoView({ block: 'nearest' });
}

function fermerSuggestions() {
  rechercheGlobale.ouvert = false;
  renderSuggestions();
}

/** Ouvre le dossier d'un fichier dans le panneau actif et sélectionne le fichier. */
async function montrerDansDossier(chemin) {
  const i = etat.actif;
  if (!(await naviguer(i, parentDe(chemin)))) return;
  const p = panneau(i);
  p.selection = new Set([chemin]);
  p.ancre = chemin;
  p.reveler = chemin;
  renderPanneau(i);
}

/** Dossier : on y va. Fichier : on ouvre son dossier et on le sélectionne. */
async function ouvrirSuggestion(k, { onglet = false } = {}) {
  const t = rechercheGlobale.resultats[k];
  if (!t) return;
  fermerSuggestions();
  rechercheGlobale.resultats = [];
  viderRecherche();
  $('#champ-recherche').blur();
  const i = etat.actif;
  const dossier = t.est_dossier ? t.chemin : t.parent;
  if (onglet) nouvelOnglet(i, dossier);
  else if (!(await naviguer(i, dossier))) return;
  else renderTous();
  if (!t.est_dossier) {
    const p = panneau(i);
    p.selection = new Set([t.chemin]);
    p.ancre = t.chemin;
    p.reveler = t.chemin;
    renderPanneau(i);
  }
}

// ———————————————————————————— Menus contextuels ————————————————————————————

const SEPARATEUR = '<div class="menu-sep" role="separator"></div>';

function item(action, icone, libelle, o = {}) {
  return `<button type="button" role="menuitem" class="menu-item${o.danger ? ' danger' : ''}" data-action="${action}" ${o.arg != null ? `data-arg="${esc(o.arg)}"` : ''} ${o.desactive ? 'disabled' : ''}>
    ${ic(icone, 16)}<span class="menu-libelle">${esc(libelle)}</span>${o.coche ? `<span class="menu-coche">${ic('coche', 16)}</span>` : ''}${o.raccourci ? `<kbd>${o.raccourci}</kbd>` : ''}
  </button>`;
}

function ligneCouleurs(action, libelle) {
  return `<div class="menu-ligne empile"><span>${libelle}</span>
    ${COULEURS_DOSSIER.map((c) => `<button type="button" class="choix-carre" style="--c:${c}" data-action="${action}" data-arg="${c}" aria-label="Couleur ${c}"></button>`).join('')}
    <button type="button" class="choix-carre aucun" data-action="${action}" data-arg="" aria-label="Couleur par défaut" title="Couleur par défaut">${ic('x', 12)}</button>
  </div>`;
}

function ouvrirMenu(x, y, html, contexte) {
  etat.menu = contexte;
  const calque = $('#calque-menu');
  calque.innerHTML = `<div class="menu" role="menu">${html}</div>`;
  const menu = calque.firstElementChild;
  const r = menu.getBoundingClientRect();
  const gauche = Math.max(8, Math.min(x, innerWidth - r.width - 8));
  const haut = y + r.height > innerHeight - 8 ? Math.max(8, innerHeight - r.height - 8) : y;
  menu.style.left = `${gauche}px`;
  menu.style.top = `${haut}px`;
}

function fermerMenu() {
  etat.menu = null;
  $('#calque-menu').innerHTML = '';
}

const EXT_TEXTE = new Set(('txt md markdown json jsonc js mjs cjs ts tsx jsx css scss less html htm xml svg yml yaml toml ini cfg conf env csv tsv log '
  + 'py rs go java kt c cpp h hpp cs php rb lua sh bash zsh fish ps1 bat cmd sql vue svelte gitignore editorconfig lock').split(' '));
const EXT_EXECUTABLE = new Set(['exe', 'msi', 'bat', 'cmd']);

function extension(e) {
  return e.extension || (e.nom.startsWith('.') ? e.nom.slice(1).toLowerCase() : '');
}
function estArchive(e) {
  const n = e.nom.toLowerCase();
  return !e.est_dossier && (n.endsWith('.zip') || n.endsWith('.tar') || n.endsWith('.tar.gz') || n.endsWith('.tgz'));
}
function nomSansArchive(nom) {
  return nom.replace(/\.(tar\.gz|tgz|tar|zip)$/i, '');
}
const sousWindows = () => SEP === '\\';

function menuElements(i, x, y) {
  const liste = entreesSelectionnees(i);
  if (!liste.length) return;
  const un = liste.length === 1 ? liste[0] : null;
  const dossiers = liste.filter((e) => e.est_dossier);
  const tousDossiers = dossiers.length === liste.length;
  const chemins = liste.map((e) => e.chemin);
  const tags = listeTags()
    .map((t) => {
      const on = chemins.every((c) => (config.tags[c] || []).includes(t.id));
      return `<button type="button" class="choix-rond${on ? ' on' : ''}" style="--c:${t.couleur}" data-action="tag" data-arg="${esc(t.id)}" aria-pressed="${on}" aria-label="Tag ${esc(t.libelle)}" title="${esc(t.libelle)}"></button>`;
    })
    .join('');

  let html = `<div class="menu-titre">${esc(un ? un.nom : `${liste.length} éléments`)}</div>`;
  html += item('ouvrir', 'ouvrir', 'Ouvrir', { raccourci: 'Entrée' });
  if (un?.est_dossier) html += item('ouvrir-onglet', 'onglet', 'Ouvrir dans un nouvel onglet');
  // Actions propres au type de l'élément
  if (un && estArchive(un)) {
    html += item('extraire-dossier', 'dossier', `Extraire dans « ${nomSansArchive(un.nom)} »`);
    html += item('extraire-ici', 'telecharger', 'Extraire ici');
  }
  if (un && sousWindows() && EXT_EXECUTABLE.has(extension(un))) html += item('executer-admin', 'alerte', 'Exécuter en tant qu’administrateur');
  if (un && !un.est_dossier && EXT_TEXTE.has(extension(un))) html += item('copier-contenu', 'document', 'Copier le contenu');
  if (un?.est_dossier) html += item('calculer-taille', 'colonnes', 'Calculer la taille');
  html += SEPARATEUR;
  html += item('compresser', 'dossierPlus', liste.length > 1 ? `Compresser les ${liste.length} éléments en .zip` : 'Compresser en .zip');
  html += item('dupliquer', 'copier', 'Dupliquer');
  html += item('couper', 'couper', 'Couper', { raccourci: 'Ctrl+X' });
  html += item('copier', 'copier', 'Copier', { raccourci: 'Ctrl+C' });
  if (un?.est_dossier && etat.pressePapiers) html += item('coller-dans', 'coller', 'Coller dans ce dossier');
  html += SEPARATEUR;
  if (un?.est_dossier) html += item('terminal', 'terminal', 'Ouvrir un terminal ici');
  if (un) html += item('vscode', 'code', 'Ouvrir dans VS Code');
  html += item('copier-chemin', 'lien', liste.length > 1 ? 'Copier les chemins' : 'Copier le chemin');
  html += item('epingler-element', 'epingle', `Épingler dans ${espace().nom}`);
  html += SEPARATEUR;
  html += `<div class="menu-ligne"><span>Tag</span>${tags}<button type="button" class="choix-rond ajout" data-action="nouveau-tag-sel" aria-label="Nouveau tag" title="Nouveau tag">${ic('plus', 12)}</button></div>`;
  if (tousDossiers) html += ligneCouleurs('couleur-elements', 'Couleur');
  html += SEPARATEUR;
  html += item('renommer', 'crayon', 'Renommer', { raccourci: 'F2', desactive: !un });
  html += item('corbeille', 'corbeille', 'Mettre à la corbeille', { danger: true, raccourci: 'Suppr' });
  ouvrirMenu(x, y, html, { type: 'elements', panneau: i, entrees: liste });
}

function menuFond(i, x, y) {
  const dossier = dossierCourant(i);
  if (!dossier) return;
  const puces = [
    ['txt', '#F1F1F4', '#3F3F46'],
    ['md', '#CCFBF1', '#115E59'],
    ['json', '#E0E7FF', '#3730A3'],
    ['html', '#FFE4E6', '#9F1239'],
    ['js', '#FEF3C7', '#92400E'],
  ]
    .map(([ext, fond, texte]) => `<button type="button" class="puce-fichier" style="background:${fond};color:${texte}" data-action="nouveau-fichier" data-arg="${ext}">.${ext}</button>`)
    .join('');
  let html = `<div class="menu-titre">Dans « ${esc(nomAffiche(dossier))} »</div>`;
  html += item('nouveau-dossier', 'dossierPlus', 'Nouveau dossier', { raccourci: 'Ctrl+Maj+N' });
  html += `<div class="menu-ligne">${ic('fichierPlus', 16)}<span>Nouveau fichier</span><div class="puces-fichier">${puces}</div></div>`;
  html += item('coller', 'coller', 'Coller', { raccourci: 'Ctrl+V', desactive: !etat.pressePapiers });
  html += SEPARATEUR;
  html += item('caches', 'oeil', 'Fichiers cachés', { coche: config.afficherCaches, raccourci: 'Ctrl+H' });
  html += item('colonnes', 'colonnes', 'Colonnes affichées…');
  html += item('groupes-date', 'document', 'Regrouper par date', { coche: estGroupeParDate(i) });
  html += item('actualiser', 'actualiser', 'Actualiser', { raccourci: 'F5' });
  html += SEPARATEUR;
  html += item('terminal-ici', 'terminal', 'Ouvrir un terminal ici');
  html += item('vscode-ici', 'code', 'Ouvrir dans VS Code');
  html += item('epingler-ici', 'epingle', `Épingler dans ${espace().nom}`);
  if (panneau(i).git) html += item('ouvrir-depot', 'branche', 'Ouvrir le dépôt dans le navigateur');
  html += SEPARATEUR;
  html += ligneCouleurs('couleur-ici', 'Couleur du dossier');
  ouvrirMenu(x, y, html, { type: 'fond', panneau: i, dossier });
}

function menuEpingle(chemin, x, y) {
  const ep = espace().epingles.find((x) => x.chemin === chemin);
  if (!ep) return;
  let html = `<div class="menu-titre">${esc(nomAffiche(chemin))}</div>`;
  html += item('ouvrir-epingle', 'ouvrir', 'Ouvrir');
  if (ep.dossier) html += item('onglet-epingle', 'onglet', 'Ouvrir dans un nouvel onglet');
  else html += item('montrer-epingle', 'dossier', 'Afficher dans son dossier');
  html += SEPARATEUR;
  html += item('retirer-epingle', 'x', `Retirer de ${espace().nom}`);
  ouvrirMenu(x, y, html, { type: 'epingle', chemin, dossier: ep.dossier });
}

function menuTag(id, x, y) {
  const t = trouverTag(id);
  if (!t) return;
  let html = `<div class="menu-titre">${esc(t.libelle)}</div>`;
  html += item('modifier-tag', 'crayon', 'Modifier le tag…');
  html += item('supprimer-tag', 'corbeille', 'Supprimer le tag', { danger: true });
  ouvrirMenu(x, y, html, { type: 'tag', tag: id });
}

function executerMenu(action, arg) {
  const ctx = etat.menu;
  if (!ctx) return;
  fermerMenu();
  const i = ctx.panneau ?? etat.actif;
  const un = ctx.entrees?.length === 1 ? ctx.entrees[0] : null;
  const chemins = ctx.entrees?.map((e) => e.chemin) ?? [];
  switch (action) {
    // Éléments sélectionnés
    case 'ouvrir': return ouvrirSelection(i);
    case 'ouvrir-onglet': return un && nouvelOnglet(i, un.chemin);
    case 'couper': return mettreDansPressePapiers(i, 'couper');
    case 'copier': return mettreDansPressePapiers(i, 'copier');
    case 'coller-dans': return un && collerIci(i, un.chemin);
    case 'terminal': return un && essayer(api.terminal(un.chemin));
    case 'vscode': return un && essayer(api.vscode(un.chemin));
    case 'copier-chemin': return copierTexte(chemins.join('\n'));
    case 'epingler-element': return epingler(ctx.entrees.map((e) => ({ chemin: e.chemin, dossier: e.est_dossier })));
    case 'nouveau-tag-sel': return ouvrirModaleTag(null, chemins);
    case 'tag': return basculerTag(chemins, arg);
    case 'couleur-elements': return colorerDossiers(chemins, arg);
    case 'renommer': return commencerRenommage(i);
    case 'extraire-dossier': return un && extraire(i, un, 'dossier');
    case 'extraire-ici': return un && extraire(i, un, 'ici');
    case 'executer-admin': return un && essayer(api.executerAdmin(un.chemin));
    case 'copier-contenu': return un && copierContenu(un);
    case 'calculer-taille': return un && calculerTaille(un);
    case 'compresser': return compresserSelection(i, chemins);
    case 'dupliquer': return collerVers(chemins, dossierCourant(i), 'copier');
    case 'ouvrir-depot': return ouvrirDepot(i);
    case 'corbeille': return mettreALaCorbeille(i);
    // Fond du dossier
    case 'nouveau-dossier': return creer(i, 'dossier', 'Nouveau dossier');
    case 'nouveau-fichier': return creer(i, 'fichier', `Nouveau fichier.${arg}`);
    case 'coller': return collerIci(i);
    case 'caches': return basculerCaches();
    case 'colonnes':
      etat.colonnesOuvert = i;
      return renderPanneau(i);
    case 'actualiser': return rafraichirTout();
    case 'groupes-date':
      config.groupesDate[ctx.dossier] = !estGroupeParDate(i);
      sauver();
      return renderPanneau(i);
    case 'terminal-ici': return essayer(api.terminal(ctx.dossier));
    case 'vscode-ici': return essayer(api.vscode(ctx.dossier));
    case 'epingler-ici': return epingler({ chemin: ctx.dossier, dossier: true });
    case 'couleur-ici': return colorerDossiers([ctx.dossier], arg);
    // Épingles
    case 'ouvrir-epingle': return ctx.dossier ? naviguer(etat.actif, ctx.chemin) : essayer(api.ouvrir(ctx.chemin));
    case 'onglet-epingle': return nouvelOnglet(etat.actif, ctx.chemin);
    case 'montrer-epingle': return montrerDansDossier(ctx.chemin);
    case 'retirer-epingle': {
      const e = espace();
      e.epingles = e.epingles.filter((ep) => ep.chemin !== ctx.chemin);
      sauver();
      return renderBarre();
    }
    // Jeux
    case 'lancer-lanceur': return essayer(api.ouvrir(ctx.executable));
    case 'dossier-lanceur': return naviguer(etat.actif, ctx.dossier);
    case 'onglet-lanceur': return nouvelOnglet(etat.actif, ctx.dossier);
    case 'copier-chemin-jeu': return copierTexte(ctx.dossier);
    // Tags
    case 'modifier-tag': return ouvrirModaleTag(ctx.tag);
    case 'supprimer-tag': return supprimerTag(ctx.tag);
  }
}

// ———————————————————————————— Mises à jour ————————————————————————————

async function verifierMiseAJour({ manuel = false } = {}) {
  if (etat.verificationEnCours) return;
  etat.verificationEnCours = true;
  etat.majAJour = false;
  renderBarre();
  const debut = performance.now();
  let echec = null;
  try {
    etat.version = await api.version();
    etat.miseAJour = await api.verifierMiseAJour();
  } catch (e) {
    echec = e; // hors ligne, ou mises à jour pas encore configurées
  }
  // L'icône tourne au moins un instant : une réponse trop rapide passerait inaperçue
  await new Promise((ok) => setTimeout(ok, Math.max(0, 600 - (performance.now() - debut))));
  etat.verificationEnCours = false;

  if (etat.miseAJour) {
    renderBarre();
    if (!etat.majAnimee) {
      etat.majAnimee = true;
      celebrer($('.bouton-maj'));
      notifier(`La version ${etat.miseAJour.version} est disponible.`, { annuler: installerMiseAJour, libelle: 'Mettre à jour' });
    }
    return;
  }
  if (manuel && echec) erreur(`Impossible de vérifier les mises à jour : ${echec}`);
  else if (manuel) {
    etat.majAJour = true;
    notifier(`Ariane est à jour (version ${etat.version}).`);
    setTimeout(() => {
      etat.majAJour = false;
      renderBarre();
    }, 2500);
  }
  renderBarre();
}

/** Petite gerbe de points colorés qui s'envolent du bouton. */
function celebrer(element) {
  if (!element || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const r = element.getBoundingClientRect();
  const couleurs = ['#F97316', '#2563EB', '#6D28D9', '#16A34A', '#FBBF24', '#DB2777'];
  for (let k = 0; k < 18; k++) {
    const p = document.createElement('span');
    p.className = 'particule';
    const angle = (Math.PI * 2 * k) / 18 + Math.random() * 0.4;
    const distance = 40 + Math.random() * 50;
    p.style.left = `${r.left + r.width / 2}px`;
    p.style.top = `${r.top + r.height / 2}px`;
    p.style.background = couleurs[k % couleurs.length];
    p.style.setProperty('--dx', `${Math.cos(angle) * distance}px`);
    p.style.setProperty('--dy', `${Math.sin(angle) * distance - 20}px`);
    document.body.appendChild(p);
    setTimeout(() => p.remove(), 900);
  }
}

async function installerMiseAJour() {
  if (etat.installationEnCours || !etat.miseAJour) return;
  etat.installationEnCours = true;
  renderBarre();
  notifier(`Téléchargement de la version ${etat.miseAJour.version}… Ariane redémarrera toute seule.`);
  try {
    await api.installerMiseAJour();
  } catch (e) {
    etat.installationEnCours = false;
    renderBarre();
    erreur(e);
  }
}

// ———————————————————————————— Nouveautés de la version ————————————————————————————

/** -1, 0 ou 1 : « 0.8.10 » est bien plus récent que « 0.8.9 ». */
function comparerVersions(a, b) {
  const pa = String(a).split('.').map(Number);
  const pb = String(b).split('.').map(Number);
  for (let k = 0; k < Math.max(pa.length, pb.length); k++) {
    const d = (pa[k] || 0) - (pb[k] || 0);
    if (d) return Math.sign(d);
  }
  return 0;
}

async function chargerNouveautes() {
  try {
    const liste = await (await fetch('nouveautes.json')).json();
    return liste.filter((n) => n.version !== 'prochaine').sort((a, b) => comparerVersions(b.version, a.version));
  } catch {
    return [];
  }
}

/** Après une mise à jour, montre une seule fois ce qui a changé depuis la version précédente. */
async function nouveautesApresMiseAJour() {
  const actuelle = etat.version;
  const vue = config.derniereVersionVue ?? (etat.reglagesExistants ? '0.0.0' : null);
  if (config.derniereVersionVue !== actuelle) {
    config.derniereVersionVue = actuelle;
    sauver();
  }
  if (!vue || comparerVersions(actuelle, vue) <= 0) return; // première installation, ou rien de neuf
  const notes = (await chargerNouveautes()).filter((n) => comparerVersions(n.version, vue) > 0 && comparerVersions(n.version, actuelle) <= 0);
  if (notes.length) ouvrirNouveautes(notes, true);
}

/** Fenêtre « Quoi de neuf ». Sans liste : tout l'historique. */
async function ouvrirNouveautes(notes = null, apresMiseAJour = false) {
  const liste = notes ?? (await chargerNouveautes());
  const date = (d) => (d ? new Date(`${d}T12:00:00`).toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' }) : '');
  const blocs = liste.length
    ? liste
        .map(
          (n) => `<section class="note-version">
            <h3>Version ${esc(n.version)}${n.date ? `<span>${date(n.date)}</span>` : ''}</h3>
            <ul>${n.points.map((p) => `<li>${esc(p)}</li>`).join('')}</ul>
          </section>`
        )
        .join('')
    : '<p class="vide">Pas encore de notes de version.</p>';
  $('#modale').innerHTML = `<div class="voile">
    <div class="modale nouveautes" role="dialog" aria-modal="true" aria-labelledby="titre-nouveautes">
      <h2 id="titre-nouveautes">${apresMiseAJour ? `Ariane est passée en version ${esc(etat.version)}` : 'Nouveautés d’Ariane'}</h2>
      <div class="notes-defilement">${blocs}</div>
      <div class="actions"><span class="pousse"></span><button type="button" class="bouton principal" data-r="fermer">${apresMiseAJour ? 'Super !' : 'Fermer'}</button></div>
    </div></div>`;
  const voile = $('#modale .voile');
  voile.addEventListener('mousedown', (ev) => {
    if (ev.target === voile) fermerModale();
  });
  $('#modale [data-r="fermer"]').addEventListener('click', fermerModale);
  $('#modale [data-r="fermer"]').focus();
  if (apresMiseAJour) celebrer($('#titre-nouveautes'));
}

// ———————————————————————————— Notifications ————————————————————————————

function notifier(texte, { erreur: estErreur = false, annuler = null, libelle = 'Annuler' } = {}) {
  const zone = $('#notifications');
  const el = document.createElement('div');
  el.className = `notif${estErreur ? ' erreur' : ''}`;
  el.setAttribute('role', estErreur ? 'alert' : 'status');
  el.innerHTML = `${ic(estErreur ? 'alerte' : 'coche', 18)}<span class="notif-texte"></span>${annuler ? `<button class="notif-btn">${esc(libelle)}</button>` : ''}<button class="notif-fermer" aria-label="Fermer">${ic('x', 14)}</button>`;
  el.querySelector('.notif-texte').textContent = texte;
  const retirer = () => el.remove();
  el.querySelector('.notif-fermer').addEventListener('click', retirer);
  el.querySelector('.notif-btn')?.addEventListener('click', () => {
    retirer();
    if (etat.annulation === annuler) etat.annulation = null;
    annuler();
  });
  zone.appendChild(el);
  while (zone.children.length > 3) zone.firstElementChild.remove();
  setTimeout(retirer, estErreur ? 9000 : annuler ? 7000 : 3500);
}

// ———————————————————————————— Fenêtre « espace » ————————————————————————————

function ouvrirModaleEspace(id = null) {
  const e = id ? config.espaces.find((x) => x.id === id) : null;
  const couleur = e?.couleur || 'bleu';
  const icone = e?.icone || 'etoile';
  const couleurs = Object.entries(PALETTE)
    .map(([cle, c]) => `<label title="${c.nom}"><input type="radio" name="couleur" value="${cle}" ${cle === couleur ? 'checked' : ''} aria-label="${c.nom}"><span class="rond" style="--c:${c.couleur}"></span></label>`)
    .join('');
  const icones = ICONES_ESPACE.map((n) => `<label><input type="radio" name="icone" value="${n}" ${n === icone ? 'checked' : ''} aria-label="Icône ${n}"><span class="carre">${ic(n, 20)}</span></label>`).join('');
  const supprimable = e && config.espaces.length > 1;
  $('#modale').innerHTML = `<div class="voile">
    <form class="modale" role="dialog" aria-modal="true" aria-labelledby="titre-modale">
      <h2 id="titre-modale">${e ? "Modifier l'espace" : 'Nouvel espace'}</h2>
      <label for="nom-espace">Nom<input id="nom-espace" type="text" name="nom" maxlength="24" required value="${esc(e?.nom || '')}" placeholder="Travail, Photos, Jeux…"></label>
      <fieldset><legend>Couleur</legend><div class="choix">${couleurs}</div></fieldset>
      <fieldset><legend>Icône</legend><div class="choix">${icones}</div></fieldset>
      <div class="actions">
        ${supprimable ? '<button type="button" class="bouton danger" data-r="supprimer">Supprimer</button>' : ''}
        <span class="pousse"></span>
        <button type="button" class="bouton" data-r="annuler">Annuler</button>
        <button type="submit" class="bouton principal">${e ? 'Enregistrer' : "Créer l'espace"}</button>
      </div>
    </form></div>`;
  const form = $('#modale form');
  const champ = $('#nom-espace');
  champ.focus();
  champ.select();
  form.addEventListener('submit', (ev) => {
    ev.preventDefault();
    const d = new FormData(form);
    const nom = String(d.get('nom')).trim();
    if (!nom) return;
    if (e) {
      Object.assign(e, { nom, couleur: d.get('couleur'), icone: d.get('icone') });
      appliquerTheme();
    } else {
      const nouvel = { id: `e${Date.now().toString(36)}`, nom, couleur: d.get('couleur'), icone: d.get('icone'), epingles: [], onglets: [] };
      config.espaces.push(nouvel);
      fermerModale();
      changerEspace(nouvel.id);
      return;
    }
    fermerModale();
    renderBarre();
    sauver();
  });
  form.addEventListener('click', (ev) => {
    const r = ev.target.closest('[data-r]')?.dataset.r;
    if (r === 'annuler') fermerModale();
    if (r === 'supprimer') {
      const bouton = ev.target.closest('[data-r]');
      if (!bouton.dataset.confirme) {
        bouton.dataset.confirme = '1';
        bouton.textContent = 'Confirmer la suppression';
        return;
      }
      config.espaces = config.espaces.filter((x) => x.id !== e.id);
      fermerModale();
      if (config.espaceActif === e.id) {
        config.espaceActif = '';
        changerEspace(config.espaces[0].id, { memoriser: false });
      } else renderBarre();
      sauver();
    }
  });
  $('#modale .voile').addEventListener('mousedown', (ev) => {
    if (ev.target.classList.contains('voile')) fermerModale();
  });
}

/** Créer (id null) ou modifier un tag. `appliquerA` : chemins à étiqueter tout de suite. */
function ouvrirModaleTag(id = null, appliquerA = []) {
  const existant = id ? config.listeTags.find((t) => t.id === id) : null;
  const utilisees = new Set(config.listeTags.map((t) => t.couleur));
  const couleurDefaut = existant?.couleur || Object.keys(COULEURS_TAG).find((c) => !utilisees.has(c)) || 'violet';
  const couleurs = Object.entries(COULEURS_TAG)
    .map(([cle, c]) => `<label title="${c.nom}"><input type="radio" name="couleur" value="${cle}" ${cle === couleurDefaut ? 'checked' : ''} aria-label="${c.nom}"><span class="rond" style="--c:${c.couleur}"></span></label>`)
    .join('');
  $('#modale').innerHTML = `<div class="voile">
    <form class="modale" role="dialog" aria-modal="true" aria-labelledby="titre-modale">
      <h2 id="titre-modale">${existant ? 'Modifier le tag' : 'Nouveau tag'}</h2>
      <label for="nom-tag">Nom<input id="nom-tag" type="text" name="nom" maxlength="24" required value="${esc(existant?.libelle || '')}" placeholder="Client, Facture, À trier…"></label>
      <fieldset><legend>Couleur</legend><div class="choix">${couleurs}</div></fieldset>
      <div class="actions">
        ${existant ? '<button type="button" class="bouton danger" data-r="supprimer">Supprimer</button>' : ''}
        <span class="pousse"></span>
        <button type="button" class="bouton" data-r="annuler">Annuler</button>
        <button type="submit" class="bouton principal">${existant ? 'Enregistrer' : 'Créer le tag'}</button>
      </div>
    </form></div>`;
  const form = $('#modale form');
  const champ = $('#nom-tag');
  champ.focus();
  champ.select();
  form.addEventListener('submit', (ev) => {
    ev.preventDefault();
    const d = new FormData(form);
    const libelle = String(d.get('nom')).trim();
    if (!libelle) return;
    if (existant) {
      Object.assign(existant, { libelle, couleur: d.get('couleur') });
    } else {
      const nouveau = { id: `t${Date.now().toString(36)}`, libelle, couleur: d.get('couleur') };
      config.listeTags.push(nouveau);
      for (const c of appliquerA) config.tags[c] = [...new Set([...(config.tags[c] || []), nouveau.id])];
    }
    fermerModale();
    sauver();
    renderBarre();
    renderTous();
  });
  form.addEventListener('click', (ev) => {
    const r = ev.target.closest('[data-r]')?.dataset.r;
    if (r === 'annuler') fermerModale();
    if (r === 'supprimer') {
      fermerModale();
      supprimerTag(existant.id);
    }
  });
  $('#modale .voile').addEventListener('mousedown', (ev) => {
    if (ev.target.classList.contains('voile')) fermerModale();
  });
}

/** Supprime un tag et le retire de tous les éléments (annulable). */
function supprimerTag(id) {
  const t = config.listeTags.find((x) => x.id === id);
  if (!t) return;
  const avant = { liste: config.listeTags.map((x) => ({ ...x })), tags: JSON.parse(JSON.stringify(config.tags)) };
  config.listeTags = config.listeTags.filter((x) => x.id !== id);
  for (const [chemin, liste] of Object.entries(config.tags)) {
    const reste = liste.filter((x) => x !== id);
    if (reste.length) config.tags[chemin] = reste;
    else delete config.tags[chemin];
  }
  if (etat.filtreTag === id) etat.filtreTag = null;
  sauver();
  renderBarre();
  renderTous();
  const annuler = () => {
    config.listeTags = avant.liste;
    config.tags = avant.tags;
    sauver();
    renderBarre();
    renderTous();
  };
  etat.annulation = annuler;
  notifier(`Tag « ${t.libelle} » supprimé`, { annuler });
}

// ———————————————————————————— Fenêtre « conflit de noms » ————————————————————————————

/** Le nom de `c.source` est déjà pris : remplacer, garder les deux ou ignorer ?
 *  Résout { choix, tous }, ou null si le collage est annulé (Échap). */
function demanderConflit(c, rang, total, mode, destination) {
  const restants = total - rang - 1;
  const date = (ms) => (ms == null ? '—' : new Date(ms).toLocaleString('fr-FR', { dateStyle: 'medium', timeStyle: 'short' }));
  const poids = (a) => (a.est_dossier ? (a.nb_elements >= 10000 ? '10 000+ éléments' : pluriel(a.nb_elements ?? 0, 'élément', 'éléments')) : taille(a.taille));
  const { nouveau: n, existant: x } = c;
  // Moins de 2 s d'écart : les disques FAT arrondissent les dates
  const memeDate = n.modifie != null && x.modifie != null && Math.abs(n.modifie - x.modifie) < 2000;
  const memePoids = n.est_dossier === x.est_dossier && (n.est_dossier ? n.nb_elements === x.nb_elements : n.taille === x.taille);
  const plusRecent = memeDate ? null : (n.modifie ?? 0) > (x.modifie ?? 0) ? 'nouveau' : 'existant';
  const plusGros = memePoids || n.est_dossier !== x.est_dossier ? null : (n.taille ?? n.nb_elements ?? 0) > (x.taille ?? x.nb_elements ?? 0) ? 'nouveau' : 'existant';
  const carte = (qui, a, chemin, titre) => {
    const e = { ...a, chemin, nom: c.nom, extension: a.est_dossier ? '' : extensionDe(c.nom) };
    return `<div class="conflit-carte">
      <div class="conflit-titre">${titre}</div>
      <div class="conflit-element"><span class="conflit-icone">${icone(e)}</span><span class="conflit-nom" title="${esc(chemin)}">${esc(c.nom)}</span></div>
      <dl>
        <dt>Modifié</dt><dd>${date(a.modifie)}${plusRecent === qui ? ' <span class="conflit-badge">plus récent</span>' : ''}</dd>
        <dt>${a.est_dossier ? 'Contenu' : 'Taille'}</dt><dd>${poids(a)}${plusGros === qui ? ` <span class="conflit-badge">${a.est_dossier ? 'plus rempli' : 'plus lourd'}</span>` : ''}</dd>
      </dl>
    </div>`;
  };
  const occupe = `${destination}${destination.endsWith(SEP) ? '' : SEP}${c.nom}`;
  const verbe = mode === 'couper' ? 'déplacé' : 'copié';
  return new Promise((resoudre) => {
    $('#modale').innerHTML = `<div class="voile">
      <div class="modale conflit" role="dialog" aria-modal="true" aria-labelledby="titre-conflit" aria-describedby="detail-conflit">
        <h2 id="titre-conflit">« ${esc(c.nom)} » existe déjà</h2>
        <p id="detail-conflit" class="conflit-detail">dans « ${esc(nomAffiche(destination))} »${total > 1 ? ` · conflit ${rang + 1} sur ${total}` : ''}</p>
        <div class="conflit-comparaison">
          ${carte('existant', x, occupe, 'Déjà là')}
          ${carte('nouveau', n, c.source, `Nouveau, ${verbe}`)}
        </div>
        ${memeDate && memePoids ? '<p class="conflit-note">Même date et même taille : c’est sans doute le même fichier.</p>' : ''}
        ${x.est_dossier ? '<p class="conflit-note">Remplacer met le dossier existant et tout son contenu à la corbeille.</p>' : ''}
        ${restants ? `<label class="conflit-tous"><input type="checkbox"> Faire de même pour ${restants > 1 ? `les ${restants} conflits suivants` : 'le conflit suivant'}</label>` : ''}
        <div class="actions">
          <button type="button" class="bouton" data-r="ignorer">Ignorer</button>
          <span class="pousse"></span>
          <button type="button" class="bouton" data-r="remplacer">Remplacer</button>
          <button type="button" class="bouton principal" data-r="garder">Garder les deux</button>
        </div>
        <p class="conflit-astuce">Échap pour annuler tout le collage</p>
      </div></div>`;
    quandFermeModale = () => resoudre(null);
    $('#modale .modale').addEventListener('click', (ev) => {
      const choix = ev.target.closest('[data-r]')?.dataset.r;
      if (!choix) return;
      const tous = !!$('#modale .conflit-tous input')?.checked;
      quandFermeModale = null;
      fermerModale();
      resoudre({ choix, tous });
    });
    // Pas de fermeture en cliquant à côté : il faut choisir (ou Échap pour tout annuler)
    $('#modale [data-r="garder"]').focus();
  });
}

/** Appelé quand la fenêtre se ferme sans réponse (Échap, clic à côté). */
let quandFermeModale = null;

function fermerModale() {
  $('#modale').innerHTML = '';
  const rappel = quandFermeModale;
  quandFermeModale = null;
  rappel?.();
}

// ———————————————————————————— Glisser-déposer ————————————————————————————
// Déposer = couper / coller : l'élément est déplacé, sur une autre ligne, un autre panneau,
// un dossier, une épingle ou une tuile de la barre latérale.

function cibleDepot(ev) {
  const g = etat.glisse;
  const el = ev.target.closest?.('[data-depot]');
  if (!g || !el) return null;
  const cible = el.dataset.depot;
  if (cible === g.dossier) return null; // déjà dedans
  if (g.chemins.some((c) => estSous(cible, c))) return null; // dans lui-même
  return el;
}

function surligner(el) {
  if (etat.survolEl === el) return;
  const ancien = etat.survolEl;
  if (ancien) {
    ancien.classList.remove('depot-survol');
    if (ancien.classList.contains('zone-liste')) {
      const sec = ancien.closest('.panneau');
      sec?.classList.remove('depot-survol');
      sec?.querySelector('.indice-depot')?.remove();
    }
  }
  etat.survolEl = el;
  if (!el) return;
  el.classList.add('depot-survol');
  if (el.classList.contains('zone-liste')) {
    const sec = el.closest('.panneau');
    sec.classList.add('depot-survol');
    const indice = document.createElement('div');
    indice.className = 'indice-depot';
    indice.innerHTML = `${ic('telecharger', 18)}<span></span>`;
    indice.querySelector('span').textContent = `Déposer pour déplacer dans « ${nomAffiche(el.dataset.depot)} »`;
    sec.appendChild(indice);
  }
}

function finirGlisse() {
  surligner(null);
  etat.glisse = null;
  document.querySelectorAll('.ligne.glisse').forEach((l) => l.classList.remove('glisse'));
  const differes = [...etat.renduDiffere];
  etat.renduDiffere.clear();
  differes.forEach(renderPanneau);
}

function brancherGlisserDeposer() {
  document.addEventListener('dragstart', (ev) => {
    const ligne = ev.target.closest?.('.ligne');
    if (!ligne) return;
    if (etat.lasso) finirLasso();
    const sec = ligne.closest('.panneau');
    const i = +sec.dataset.i;
    const p = panneau(i);
    const e = p.visibles[+ligne.dataset.idx];
    if (!e || !p.contenu) return ev.preventDefault();
    if (!p.selection.has(e.chemin)) {
      // Mise à jour de la sélection sans tout redessiner (sinon le glissement s'interrompt)
      p.selection = new Set([e.chemin]);
      p.ancre = e.chemin;
      sec.querySelectorAll('.ligne.sel').forEach((l) => l.classList.remove('sel'));
      ligne.classList.add('sel');
    }
    activerPanneau(i);
    const chemins = [...p.selection];
    etat.glisse = { panneau: i, dossier: p.contenu.chemin, chemins };
    ev.dataTransfer.effectAllowed = 'move';
    ev.dataTransfer.setData('text/plain', chemins.join('\n'));
    if (chemins.length > 1) {
      const fantome = document.createElement('div');
      fantome.className = 'fantome';
      fantome.textContent = `${chemins.length} éléments`;
      document.body.appendChild(fantome);
      ev.dataTransfer.setDragImage(fantome, 16, 16);
      setTimeout(() => fantome.remove(), 0);
    }
    requestAnimationFrame(() => sec.querySelectorAll('.ligne.sel').forEach((l) => l.classList.add('glisse')));
  });

  const survol = (ev) => {
    if (!etat.glisse) return;
    const el = cibleDepot(ev);
    surligner(el);
    if (el) {
      ev.preventDefault();
      ev.dataTransfer.dropEffect = 'move';
    }
  };
  document.addEventListener('dragenter', survol);
  document.addEventListener('dragover', survol);

  document.addEventListener('drop', (ev) => {
    const el = cibleDepot(ev);
    const g = etat.glisse;
    if (!el || !g) return finirGlisse();
    ev.preventDefault();
    const destination = el.dataset.depot;
    finirGlisse();
    collerVers(g.chemins, destination, 'couper');
  });
  document.addEventListener('dragend', finirGlisse);
}

// ———————————————————————————— Sélection par zone ————————————————————————————
// Comme l'Explorateur Windows : on trace un rectangle depuis un endroit libre (hors icône et nom)
// et tout ce qu'il touche est sélectionné. Avec Ctrl, on ajoute à la sélection existante.

function debuterLasso(ev, i) {
  const def = ev.target.closest('.defilement');
  if (!def) return;
  const r = def.getBoundingClientRect();
  etat.lasso = {
    panneau: i,
    def,
    depart: { x: ev.clientX - r.left + def.scrollLeft, y: ev.clientY - r.top + def.scrollTop },
    souris: { x: ev.clientX, y: ev.clientY },
    base: ev.ctrlKey || ev.metaKey ? new Set(panneau(i).selection) : new Set(),
    actif: false,
    cadre: null,
    minuterie: null,
  };
}

function suivreLasso() {
  const l = etat.lasso;
  if (!l) return;
  const def = l.def;
  const r = def.getBoundingClientRect();
  const x = l.souris.x - r.left + def.scrollLeft;
  const y = l.souris.y - r.top + def.scrollTop;
  if (!l.actif) {
    if (Math.abs(x - l.depart.x) < 5 && Math.abs(y - l.depart.y) < 5) return;
    l.actif = true;
    l.cadre = document.createElement('div');
    l.cadre.className = 'lasso';
    def.appendChild(l.cadre);
    // Défilement automatique quand on approche du haut ou du bas de la liste
    l.minuterie = setInterval(() => {
      const b = def.getBoundingClientRect();
      if (l.souris.y < b.top + 30) def.scrollTop -= 16;
      else if (l.souris.y > b.bottom - 30) def.scrollTop += 16;
      else return;
      suivreLasso();
    }, 16);
  }
  const gauche = Math.min(x, l.depart.x);
  const haut = Math.min(y, l.depart.y);
  const droite = Math.max(x, l.depart.x);
  const bas = Math.max(y, l.depart.y);
  Object.assign(l.cadre.style, { left: `${gauche}px`, top: `${haut}px`, width: `${droite - gauche}px`, height: `${bas - haut}px` });

  const p = panneau(l.panneau);
  const selection = new Set(l.base);
  def.querySelectorAll('.ligne').forEach((ligne) => {
    const touche = ligne.offsetTop < bas && ligne.offsetTop + ligne.offsetHeight > haut && ligne.offsetLeft < droite && ligne.offsetLeft + ligne.offsetWidth > gauche;
    const e = p.visibles[+ligne.dataset.idx];
    if (touche && e) selection.add(e.chemin);
  });
  p.selection = selection;
  majSelection(l.panneau);
}

function finirLasso() {
  const l = etat.lasso;
  if (!l) return;
  clearInterval(l.minuterie);
  l.cadre?.remove();
  etat.lasso = null;
  if (l.actif) {
    const p = panneau(l.panneau);
    p.ancre = p.visibles.find((e) => p.selection.has(e.chemin))?.chemin ?? null;
  }
  const differes = [...etat.renduDiffere];
  etat.renduDiffere.clear();
  differes.forEach(renderPanneau);
}

// ———————————————————————————— Événements ————————————————————————————

function actionPanneau(i, action, d) {
  const p = panneau(i);
  switch (action) {
    case 'onglet': return changerOnglet(i, +d.k);
    case 'fermer-onglet': return fermerOnglet(i, +d.k);
    case 'nouvel-onglet': return nouvelOnglet(i, dossierCourant(i) || standards.accueil);
    case 'retour': return historique(i, -1);
    case 'avance': return historique(i, 1);
    case 'haut': return remonter(i);
    case 'aller': return naviguer(i, d.chemin);
    case 'editer-chemin': return editerChemin(i);
    case 'caches': return basculerCaches();
    case 'colonnes':
      etat.colonnesOuvert = etat.colonnesOuvert === i ? null : i;
      return renderPanneau(i);
    case 'colonne':
      config.colonnes[d.col] = !config.colonnes[d.col];
      sauver();
      return renderTous();
    case 'groupe':
      p.replies ??= new Set();
      if (p.replies.has(d.libelle)) p.replies.delete(d.libelle);
      else p.replies.add(d.libelle);
      return renderPanneau(i);
    case 'trier':
      if (estGroupeParDate(i)) {
        config.groupesDate[dossierCourant(i)] = false;
        sauver();
        notifier('Regroupement par date désactivé pour ce dossier (clic droit dans le vide pour le remettre).');
      }
      if (p.tri.col === d.col) p.tri.sens *= -1;
      else p.tri = { col: d.col, sens: d.col === 'nom' ? 1 : -1 };
      return renderPanneau(i);
    case 'terminal-ici': return dossierCourant(i) && essayer(api.terminal(dossierCourant(i)));
    case 'vscode-ici': return dossierCourant(i) && essayer(api.vscode(dossierCourant(i)));
  }
}

function actionBarre(action, d) {
  const i = etat.actif;
  switch (action) {
    case 'aller': return naviguer(i, d.chemin);
    case 'espace': return changerEspace(d.id);
    case 'nouvel-espace': return ouvrirModaleEspace();
    case 'modifier-espace': return ouvrirModaleEspace(espace().id);
    case 'epingler': return dossierCourant(i) && epingler({ chemin: dossierCourant(i), dossier: true });
    case 'ouvrir-fichier': return essayer(api.ouvrir(d.chemin));
    case 'nouveau-tag': return ouvrirModaleTag();
    case 'lanceur': return cliquerLanceur(d.id);
    case 'mettre-a-jour': return installerMiseAJour();
    case 'verifier-maj': return verifierMiseAJour({ manuel: true });
    case 'theme': return basculerModeSombre();
    case 'nouveautes': return ouvrirNouveautes();
    case 'filtre-tag':
      etat.filtreTag = d.id && etat.filtreTag !== d.id ? d.id : null;
      renderBarre();
      return renderTous();
    case 'diviser':
      config.deuxPanneaux = !config.deuxPanneaux;
      if (!config.deuxPanneaux) etat.actif = 0;
      sauver();
      renderBarre();
      renderStructure();
      if (config.deuxPanneaux && !panneau(1).contenu) charger(1);
      return;
  }
}

function brancherEvenements() {
  const zone = $('#panneaux');

  zone.addEventListener('mousedown', (ev) => {
    const sec = ev.target.closest('.panneau');
    if (!sec) return;
    if (ev.button === 3 || ev.button === 4) ev.preventDefault();
    activerPanneau(+sec.dataset.i);
    // Zone de sélection : depuis la liste, mais pas depuis une icône, un nom ou l'en-tête
    const libre = ev.button === 0 && !ev.shiftKey && ev.target.closest('.defilement') && !ev.target.closest('.poignee, .entete, input, button');
    if (libre) debuterLasso(ev, +sec.dataset.i);
  });
  document.addEventListener('mousemove', (ev) => {
    if (!etat.lasso) return;
    etat.lasso.souris = { x: ev.clientX, y: ev.clientY };
    suivreLasso();
  });
  document.addEventListener('mouseup', () => {
    if (!etat.lasso) return;
    // Le « click » qui suit une zone tracée ne doit rien changer à la sélection
    if (etat.lasso.actif) {
      etat.lassoTermine = true;
      setTimeout(() => (etat.lassoTermine = false), 0);
    }
    finirLasso();
  });

  zone.addEventListener('click', (ev) => {
    const sec = ev.target.closest('.panneau');
    if (!sec) return;
    const i = +sec.dataset.i;
    const bouton = ev.target.closest('[data-action]');
    if (bouton) return actionPanneau(i, bouton.dataset.action, bouton.dataset);
    if (ev.target.classList.contains('fil')) return editerChemin(i);
    if (etat.colonnesOuvert === i && !ev.target.closest('.popover')) {
      etat.colonnesOuvert = null;
      renderPanneau(i);
    }
    const ligne = ev.target.closest('.ligne');
    if (ligne && etat.lassoTermine) {
      etat.lassoTermine = false;
      return;
    }
    if (ligne) {
      if (ev.target.closest('.renommage')) return;
      const e = panneau(i).visibles[+ligne.dataset.idx];
      const maintenant = performance.now();
      const sansTouche = !ev.ctrlKey && !ev.metaKey && !ev.shiftKey;
      if (e && sansTouche && dernierClic.chemin === e.chemin && maintenant - dernierClic.temps < DELAI_DOUBLE_CLIC) {
        dernierClic = { chemin: null, temps: 0 };
        ouvrirEntree(i, e);
        return;
      }
      dernierClic = { chemin: e?.chemin ?? null, temps: maintenant };
      cliquerLigne(i, +ligne.dataset.idx, ev);
      return;
    }
    if (ev.target.closest('.zone-liste')) {
      const p = panneau(i);
      if (etat.lassoTermine) {
        etat.lassoTermine = false; // fin d'une sélection par zone : on garde la sélection
        return;
      }
      if (p.selection.size) {
        p.selection.clear();
        renderPanneau(i);
      }
    }
  });


  // Clic molette sur un dossier : nouvel onglet
  zone.addEventListener('auxclick', (ev) => {
    if (ev.button !== 1) return;
    const ligne = ev.target.closest('.ligne');
    if (!ligne) return;
    ev.preventDefault();
    const i = +ligne.closest('.panneau').dataset.i;
    const e = panneau(i).visibles[+ligne.dataset.idx];
    if (e?.est_dossier) nouvelOnglet(i, e.chemin);
  });

  // Boutons « précédent / suivant » de la souris
  zone.addEventListener('mouseup', (ev) => {
    if (ev.button !== 3 && ev.button !== 4) return;
    ev.preventDefault();
    const sec = ev.target.closest('.panneau');
    if (sec) historique(+sec.dataset.i, ev.button === 3 ? -1 : 1);
  });

  zone.addEventListener('contextmenu', (ev) => {
    ev.preventDefault();
    const sec = ev.target.closest('.panneau');
    if (!sec) return;
    const i = +sec.dataset.i;
    activerPanneau(i);
    const p = panneau(i);
    const ligne = ev.target.closest('.ligne');
    if (ligne) {
      const e = p.visibles[+ligne.dataset.idx];
      if (!e) return;
      if (!p.selection.has(e.chemin)) {
        p.selection = new Set([e.chemin]);
        p.ancre = e.chemin;
        renderPanneau(i);
      }
      menuElements(i, ev.clientX, ev.clientY);
    } else if (ev.target.closest('.zone-liste')) {
      if (p.selection.size) {
        p.selection.clear();
        renderPanneau(i);
      }
      menuFond(i, ev.clientX, ev.clientY);
    }
  });

  // Champ de renommage
  zone.addEventListener('keydown', (ev) => {
    if (ev.target.classList.contains('champ-chemin')) {
      const i = +ev.target.closest('.panneau').dataset.i;
      if (ev.key === 'Enter') {
        ev.preventDefault();
        validerChemin(i, ev.target);
      } else if (ev.key === 'Escape') {
        ev.preventDefault();
        ev.stopPropagation();
        fermerEditionChemin(i);
      }
      return;
    }
    if (!ev.target.classList.contains('renommage')) return;
    const i = +ev.target.closest('.panneau').dataset.i;
    if (ev.key === 'Enter') {
      ev.preventDefault();
      terminerRenommage(i, ev.target);
    } else if (ev.key === 'Escape') {
      ev.preventDefault();
      ev.stopPropagation();
      terminerRenommage(i, ev.target, true);
    }
  });
  zone.addEventListener('focusout', (ev) => {
    if (ev.target.classList.contains('champ-chemin')) {
      const sec = ev.target.closest('.panneau');
      // Laisse le temps à un éventuel Entrée de partir avant de refermer
      if (sec) setTimeout(() => fermerEditionChemin(+sec.dataset.i), 0);
      return;
    }
    if (!ev.target.classList.contains('renommage')) return;
    const sec = ev.target.closest('.panneau');
    if (sec) terminerRenommage(+sec.dataset.i, ev.target);
  });

  // Barre latérale
  const barre = $('#barre');
  barre.addEventListener('click', (ev) => {
    const bouton = ev.target.closest('[data-action]');
    if (bouton) actionBarre(bouton.dataset.action, bouton.dataset);
  });
  barre.addEventListener('contextmenu', (ev) => {
    if (ev.target.closest('input')) return;
    ev.preventDefault();
    const epingle = ev.target.closest('[data-epingle]');
    if (epingle) return menuEpingle(epingle.dataset.epingle, ev.clientX, ev.clientY);
    const tag = ev.target.closest('[data-tag]');
    if (tag) return menuTag(tag.dataset.tag, ev.clientX, ev.clientY);
    const lanceur = ev.target.closest('[data-lanceur]');
    if (lanceur) return menuLanceur(lanceur.dataset.lanceur, ev.clientX, ev.clientY);
    const jeu = ev.target.closest('[data-jeu]');
    if (jeu) return menuJeu(jeu.dataset.jeu, ev.clientX, ev.clientY);
  });

  // Recherche : filtre le panneau actif, propose des résultats dans tout le dossier
  // personnel, ou va à un chemin tapé (/…, ~/…, C:\…)
  const champ = $('#champ-recherche');
  champ.addEventListener('input', () => {
    const avant = etat.recherchePanneau;
    // Un chemin tapé (/…, ~/…, C:\…) ne filtre pas le dossier ouvert
    etat.recherche = estChemin(champ.value.trim()) ? '' : champ.value;
    etat.recherchePanneau = etat.actif;
    if (avant !== etat.actif) renderPanneau(avant);
    renderPanneau(etat.actif);
    planifierRecherche(champ.value);
  });
  champ.addEventListener('keydown', async (ev) => {
    const ouvert = rechercheGlobale.ouvert && rechercheGlobale.resultats.length > 0;
    if (ev.key === 'ArrowDown' || ev.key === 'ArrowUp') {
      if (!ouvert) return;
      ev.preventDefault();
      const n = rechercheGlobale.resultats.length;
      rechercheGlobale.index = (rechercheGlobale.index + (ev.key === 'ArrowDown' ? 1 : -1) + n) % n;
      renderSuggestions();
    } else if (ev.key === 'Escape') {
      if (rechercheGlobale.ouvert) return fermerSuggestions();
      viderRecherche();
      renderTous();
      champ.blur();
    } else if (ev.key === 'Enter') {
      ev.preventDefault();
      const v = champ.value.trim();
      if (estChemin(v)) {
        if (await naviguer(etat.actif, v)) champ.blur();
      } else if (ouvert) {
        ouvrirSuggestion(rechercheGlobale.index);
      }
    }
  });
  champ.addEventListener('focus', () => {
    if (rechercheGlobale.resultats.length && champ.value.trim().length >= 2) {
      rechercheGlobale.ouvert = true;
      renderSuggestions();
    }
  });
  const liste = $('#suggestions');
  // mousedown + preventDefault : le champ garde le focus, le clic n'est pas perdu
  liste.addEventListener('mousedown', (ev) => {
    const item = ev.target.closest('[data-k]');
    if (!item) return;
    ev.preventDefault();
    if (ev.button === 0) ouvrirSuggestion(+item.dataset.k);
    if (ev.button === 1) ouvrirSuggestion(+item.dataset.k, { onglet: true });
  });
  document.addEventListener('mousedown', (ev) => {
    if (rechercheGlobale.ouvert && !ev.target.closest('.recherche')) fermerSuggestions();
  });

  // Menus
  const calque = $('#calque-menu');
  calque.addEventListener('click', (ev) => {
    const bouton = ev.target.closest('[data-action]');
    if (bouton && !bouton.disabled) executerMenu(bouton.dataset.action, bouton.dataset.arg);
  });
  calque.addEventListener('contextmenu', (ev) => ev.preventDefault());
  document.addEventListener('mousedown', (ev) => {
    if (etat.menu && !ev.target.closest('.menu')) fermerMenu();
  });
  window.addEventListener('blur', fermerMenu);
  window.addEventListener('resize', fermerMenu);

  // Retour dans la fenêtre : on actualise (des fichiers ont pu changer ailleurs)
  window.addEventListener('focus', () => {
    if (!etat.glisse) rafraichirTout();
  });

  document.addEventListener('keydown', clavier);
  brancherGlisserDeposer();
}

function clavier(ev) {
  const ctrl = ev.ctrlKey || ev.metaKey;
  const touche = ev.key.length === 1 ? ev.key.toLowerCase() : ev.key;

  if (touche === 'Escape') {
    if (etat.menu) return fermerMenu();
    if ($('#modale').innerHTML) return fermerModale();
    if (etat.colonnesOuvert != null) {
      const i = etat.colonnesOuvert;
      etat.colonnesOuvert = null;
      return renderPanneau(i);
    }
  }
  if (ev.target.matches('input, textarea, select') || $('#modale').innerHTML) return;

  const i = etat.actif;
  const p = panneau(i);
  const raccourcis = {
    c: () => ctrl && mettreDansPressePapiers(i, 'copier'),
    x: () => ctrl && mettreDansPressePapiers(i, 'couper'),
    v: () => ctrl && collerIci(i),
    a: () => {
      if (!ctrl) return false;
      p.selection = new Set(p.visibles.map((e) => e.chemin));
      renderPanneau(i);
    },
    z: () => {
      if (!ctrl || !etat.annulation) return false;
      const annuler = etat.annulation;
      etat.annulation = null;
      annuler();
    },
    h: () => ctrl && basculerCaches(),
    t: () => ctrl && nouvelOnglet(i, dossierCourant(i) || standards.accueil),
    w: () => ctrl && fermerOnglet(i, p.ongletActif),
    l: () => ctrl && editerChemin(i),
    f: () => ctrl && $('#champ-recherche').focus(),
    n: () => ctrl && ev.shiftKey && creer(i, 'dossier', 'Nouveau dossier'),
    Tab: () => ctrl && changerOnglet(i, (p.ongletActif + (ev.shiftKey ? -1 : 1) + p.onglets.length) % p.onglets.length),
    F2: () => commencerRenommage(i),
    F5: () => rafraichirTout(),
    Delete: () => mettreALaCorbeille(i),
    Enter: () => ouvrirSelection(i),
    Backspace: () => historique(i, -1),
    ArrowLeft: () => ev.altKey && historique(i, -1),
    ArrowRight: () => ev.altKey && historique(i, 1),
    ArrowUp: () => (ev.altKey ? remonter(i) : deplacerSelection(i, -1, ev.shiftKey)),
    ArrowDown: () => (ev.altKey ? undefined : deplacerSelection(i, 1, ev.shiftKey)),
  };
  const action = raccourcis[touche];
  if (!action) return;
  const resultat = action();
  if (resultat !== false) ev.preventDefault();
}

// ———————————————————————————— Démarrage ————————————————————————————

async function demarrer() {
  standards = await api.standards();
  SEP = standards.separateur;
  try {
    const brut = await api.lireConfig();
    config = brut ? completer(JSON.parse(brut)) : configParDefaut();
    etat.reglagesExistants = !!brut;
  } catch {
    config = configParDefaut();
  }
  appliquerTheme();
  construirePanneaux(espace());

  // Dossier passé en argument (ariane ~/Projets) : ouvert dans un onglet
  const initial = await api.cheminInitial().catch(() => null);
  if (initial) {
    const p = panneau(0);
    p.onglets.push({ chemin: initial, histo: [initial], pos: 0 });
    p.ongletActif = p.onglets.length - 1;
  }

  renderStructure();
  renderBarre();
  brancherEvenements();
  for (let i = 0; i < nbPanneaux(); i++) charger(i);
  chargerJeux();
  api.version().then((v) => {
    etat.version = v;
    renderBarre();
    nouveautesApresMiseAJour();
  }).catch(() => {});
  setTimeout(() => verifierMiseAJour(), 3000); // sans ralentir l'ouverture
}

// Toute erreur inattendue s'affiche, au lieu d'échouer en silence
window.addEventListener('error', (ev) => notifier(`Erreur interne : ${ev.message}`, { erreur: true }));
window.addEventListener('unhandledrejection', (ev) => notifier(`Erreur interne : ${ev.reason?.message ?? ev.reason}`, { erreur: true }));

demarrer().catch((e) => {
  document.body.innerHTML = `<p style="padding:24px;font-family:sans-serif">Démarrage impossible : ${esc(String(e))}</p>`;
});
