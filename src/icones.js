// Icônes au trait (SVG), dessinées avec currentColor pour hériter de la couleur du texte.

const TRACES = {
  retour: '<path d="M15 18l-6-6 6-6"/>',
  avance: '<path d="M9 18l6-6-6-6"/>',
  haut: '<path d="M12 19V5"/><path d="M5 12l7-7 7 7"/>',
  chercher: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
  oeil: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  colonnes: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
  terminal: '<path d="M4 17l6-5-6-5"/><path d="M12 19h8"/>',
  code: '<path d="M16 18l6-6-6-6"/><path d="M8 6l-6 6 6 6"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  x: '<path d="M18 6L6 18M6 6l12 12"/>',
  maison: '<path d="M3 11l9-7 9 7"/><path d="M5 10v10h14V10"/>',
  telecharger: '<path d="M12 4v11"/><path d="M7 10l5 5 5-5"/><path d="M5 20h14"/>',
  ecran: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>',
  document: '<path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z"/><path d="M14 3v5h5"/><path d="M9 13h6M9 17h6"/>',
  // Balai : manche, ligature, brosse évasée, et une étincelle
  nuage: '<path d="M7 19h10.5a4.5 4.5 0 0 0 .4-8.98A6 6 0 0 0 6.3 9.5 4.75 4.75 0 0 0 7 19z"/>',
  reglages: '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>',
  balai: '<path d="M12 2v8"/><path d="M8 13v-1.5A1.5 1.5 0 0 1 9.5 10h5a1.5 1.5 0 0 1 1.5 1.5V13"/><path d="M7 13h10l2 8H5z"/><path d="M10 21l.4-4M14 21l-.4-4"/><path d="M19 2.5c.2 1.1.9 1.8 2 2-1.1.2-1.8.9-2 2-.2-1.1-.9-1.8-2-2 1.1-.2 1.8-.9 2-2z"/>',
  corbeille: '<path d="M4 7h16"/><path d="M10 11v6M14 11v6"/><path d="M6 7l1 13h10l1-13"/><path d="M9 7V4h6v3"/>',
  ouvrir: '<path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5"/>',
  copier: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  couper: '<circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M20 4L8.1 15.9M14.5 14.5L20 20M8.1 8.1L12 12"/>',
  coller: '<rect x="5" y="5" width="14" height="16" rx="2"/><rect x="9" y="3" width="6" height="4" rx="1"/>',
  crayon: '<path d="M4 20h4L19 9l-4-4L4 16z"/>',
  branche: '<circle cx="6" cy="5" r="2.5"/><circle cx="6" cy="19" r="2.5"/><circle cx="18" cy="7" r="2.5"/><path d="M6 7.5v9"/><path d="M18 9.5c0 4.5-6 4-11.5 7"/>',
  coche: '<path d="M5 12l5 5 9-10"/>',
  alerte: '<circle cx="12" cy="12" r="9"/><path d="M12 8v5M12 16.5h.01"/>',
  points: '<circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/>',
  dossierPlus: '<path d="M3 6a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M12 10.5v6M9 13.5h6"/>',
  fichierPlus: '<path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z"/><path d="M14 3v5h5"/><path d="M12 11v6M9 14h6"/>',
  epingle: '<path d="M12 17v5"/><path d="M9 3h6l-1 6 3 3v2H7v-2l3-3z"/>',
  diviser: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M12 4v16"/>',
  actualiser: '<path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 4v7h-7"/>',
  onglet: '<rect x="3" y="6" width="18" height="14" rx="2"/><path d="M3 10h18"/><path d="M8 6V4h8v2"/>',
  lien: '<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
  manette: '<rect x="2" y="7" width="20" height="11" rx="5"/><path d="M7 11v4M5 13h4"/><path d="M15.5 12h.01M18 14.5h.01"/>',
  etoile: '<path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z"/>',
  livre: '<path d="M4 4h6a3 3 0 0 1 3 3v13a2 2 0 0 0-2-2H4z"/><path d="M20 4h-6a3 3 0 0 0-3 3v13a2 2 0 0 1 2-2h7z"/>',
  valise: '<rect x="3" y="7" width="18" height="13" rx="2"/><path d="M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/><path d="M3 13h18"/>',
  musique: '<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>',
  image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M21 16l-5-5-9 9"/>',
  video: '<rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10.5l5-3v9l-5-3z"/>',
  soleil: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  lune: '<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z"/>',
  dossier: '<path d="M3 6a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  usb: '<rect x="7" y="9" width="10" height="12" rx="2"/><path d="M9 9V3h6v6"/><path d="M11 5.5v1M13 5.5v1"/>',
  disqueExterne: '<rect x="3" y="6" width="18" height="12" rx="3"/><path d="M7 14h5"/><path d="M16.5 14h.01"/>',
  carte: '<path d="M8 3h7l4 4v13a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6z"/><path d="M10 6.5v3M13 6.5v3"/>',
  cd: '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2.5"/>',
  ejecter: '<path d="M12 5l7 8H5z"/><path d="M5 18.5h14"/>',
};

/** Icône au trait. `attributs` permet d'ajouter un style (couleur…). */
export function ic(nom, taille = 18, attributs = '') {
  return `<svg width="${taille}" height="${taille}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" ${attributs}>${TRACES[nom] || ''}</svg>`;
}

/** Dossier plein, coloré. */
export function icDossier(couleur, largeur = 24, hauteur = 20) {
  return `<svg class="ic-dossier" width="${largeur}" height="${hauteur}" viewBox="0 0 24 20" aria-hidden="true" style="color:${couleur}"><path fill="currentColor" d="M2 4a2 2 0 0 1 2-2h5l2 2.5h9a2 2 0 0 1 2 2V16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z"/><path d="M2 7.5h20" stroke="rgba(255,255,255,.28)" stroke-width="1.4"/></svg>`;
}

export const ICONES_DISPONIBLES = Object.keys(TRACES);

// ——— Icônes de fichiers : une page à coin plié, colorée selon la famille, avec un dessin ———
// Repère : viewBox 24 × 28, le dessin occupe le bas de la page (autour de x 12, y 18).
const DESSINS = {
  texte: '<path d="M7.5 13h9M7.5 16.5h9M7.5 20h5.5" fill="none" stroke="C" stroke-width="1.7" stroke-linecap="round"/>',
  doc: '<path d="M7.5 12.5h9M7.5 15.5h9M7.5 18.5h9M7.5 21.5h6" fill="none" stroke="C" stroke-width="1.6" stroke-linecap="round"/>',
  pdf: '<path d="M7.5 11.5h9M7.5 14.5h6" fill="none" stroke="C" stroke-width="1.6" stroke-linecap="round"/>'
    + '<rect x="2.5" y="17.5" width="15" height="7" rx="1.6" fill="C"/>'
    + '<text x="10" y="22.9" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="5.6" font-weight="800" fill="#fff">PDF</text>',
  image: '<circle cx="9.3" cy="13.3" r="1.9" fill="C"/><path d="M5.5 23l4.2-5.2 2.8 3 2.4-2.6 3.6 4.8z" fill="C"/>',
  video: '<rect x="5.5" y="12" width="13" height="10.5" rx="2.2" fill="C"/><path d="M10.6 14.6v5.3l4.4-2.65z" fill="#fff"/>',
  audio: '<path d="M11.2 21V12.6l5.6-1.4v7.6" fill="none" stroke="C" stroke-width="1.7" stroke-linejoin="round"/>'
    + '<circle cx="9.4" cy="21" r="2.2" fill="C"/><circle cx="15" cy="18.8" r="2.2" fill="C"/>',
  archive: '<path d="M12 2.5v1.6M12 5.6v1.6M12 8.7v1.6M12 11.8v1.6" stroke="C" stroke-width="2.6"/>'
    + '<rect x="9.6" y="14.6" width="4.8" height="6.4" rx="1.3" fill="C"/><rect x="11.1" y="17.6" width="1.8" height="2" rx=".5" fill="#fff"/>',
  disque: '<circle cx="12" cy="17.5" r="5.4" fill="C"/><circle cx="12" cy="17.5" r="1.6" fill="#fff"/>',
  executable: '<rect x="5.8" y="12" width="12.4" height="10.5" rx="1.8" fill="C"/><path d="M5.8 15h12.4" stroke="#fff" stroke-width="1.2"/>'
    + '<circle cx="8" cy="13.5" r=".6" fill="#fff"/><circle cx="9.8" cy="13.5" r=".6" fill="#fff"/>',
  tableur: '<rect x="6" y="12" width="12" height="10" rx="1.3" fill="none" stroke="C" stroke-width="1.5"/><path d="M6 15.4h12M6 18.7h12M10.2 12v10" stroke="C" stroke-width="1.3"/>',
  presentation: '<rect x="5.8" y="11.8" width="12.4" height="8.4" rx="1.3" fill="C"/><path d="M8.6 17.6v-2M11 17.6v-3.4M13.4 17.6v-1.4" stroke="#fff" stroke-width="1.3" stroke-linecap="round"/><path d="M12 20.2v2.6M9.5 22.8h5" stroke="C" stroke-width="1.4" stroke-linecap="round"/>',
  web: '<circle cx="12" cy="17.5" r="5.2" fill="none" stroke="C" stroke-width="1.5"/><path d="M6.8 17.5h10.4M12 12.3c2.6 2.8 2.6 7.6 0 10.4M12 12.3c-2.6 2.8-2.6 7.6 0 10.4" fill="none" stroke="C" stroke-width="1.3"/>',
  code: '<path d="M9.6 13.4l-3.4 4.1 3.4 4.1M14.4 13.4l3.4 4.1-3.4 4.1" fill="none" stroke="C" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>',
  script: '<rect x="5.6" y="12" width="12.8" height="10.5" rx="1.8" fill="C"/><path d="M8.2 15l2.4 2.2-2.4 2.2M12 19.6h3.6" fill="none" stroke="#fff" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>',
  donnees: '<path d="M10.2 12.6c-1.6 0-1.8.9-1.8 2.1s-.3 2.3-1.7 2.3c1.4 0 1.7 1 1.7 2.3s.2 2.1 1.8 2.1M13.8 12.6c1.6 0 1.8.9 1.8 2.1s.3 2.3 1.7 2.3c-1.4 0-1.7 1-1.7 2.3s-.2 2.1-1.8 2.1" fill="none" stroke="C" stroke-width="1.6" stroke-linecap="round"/>',
  config: '<path d="M7 13.5h10M7 17.5h10M7 21.5h10" stroke="C" stroke-width="1.4" stroke-linecap="round"/><circle cx="14" cy="13.5" r="1.7" fill="C"/><circle cx="9.5" cy="17.5" r="1.7" fill="C"/><circle cx="13" cy="21.5" r="1.7" fill="C"/>',
  police: '<text x="12" y="22" text-anchor="middle" font-family="Georgia, serif" font-size="10" font-weight="700" fill="C">Aa</text>',
  defaut: '',
};

/** Icône d'un fichier selon sa famille (voir typeFichier dans format.js). */
export function icFichier(type, largeur = 25, hauteur = 29) {
  const c = type.vive;
  const dessin = (DESSINS[type.famille] ?? '').replaceAll('"C"', `"${c}"`);
  return `<svg class="ic-fichier" width="${largeur}" height="${hauteur}" viewBox="0 0 24 28" aria-hidden="true">
    <path d="M5.5 1.5h9L20.5 7.5v17.5a1.6 1.6 0 0 1-1.6 1.6H5.5a1.6 1.6 0 0 1-1.6-1.6V3.1a1.6 1.6 0 0 1 1.6-1.6z" fill="${type.fond}" stroke="${c}" stroke-width="1.3"/>
    <path d="M14.5 1.5v4.4a1.6 1.6 0 0 0 1.6 1.6h4.4z" fill="${c}" opacity=".35"/>${dessin}</svg>`;
}

// Petits dessins blancs posés sur les dossiers spéciaux (Téléchargements, Images…), repère 24 × 20
const SUR_DOSSIER = {
  telechargements: '<path d="M12 8.5v6M9.4 12.2l2.6 2.6 2.6-2.6" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>',
  documents: '<path d="M9 9.5h6M9 12h6M9 14.5h4" stroke="#fff" stroke-width="1.5" stroke-linecap="round"/>',
  images: '<circle cx="10" cy="10" r="1.3" fill="#fff"/><path d="M7.5 15.5l3-3.6 2 2 1.6-1.7 2.4 3.3z" fill="#fff"/>',
  musique: '<path d="M11.2 14.6V8.8l4-1v5" fill="none" stroke="#fff" stroke-width="1.4"/><circle cx="9.9" cy="14.6" r="1.5" fill="#fff"/><circle cx="13.9" cy="12.9" r="1.5" fill="#fff"/>',
  videos: '<path d="M10.4 8.8v5.6l4.6-2.8z" fill="#fff"/>',
  bureau: '<rect x="8" y="8.6" width="8" height="5.4" rx=".8" fill="none" stroke="#fff" stroke-width="1.4"/><path d="M10.5 15.6h3" stroke="#fff" stroke-width="1.4" stroke-linecap="round"/>',
  accueil: '<path d="M8.4 11.6L12 8.6l3.6 3v3.6H8.4z" fill="#fff"/>',
};

/** Dossier coloré, avec un dessin s'il s'agit d'un dossier spécial. */
export function icDossierSpecial(couleur, special, largeur = 24, hauteur = 20) {
  const base = icDossier(couleur, largeur, hauteur);
  const dessin = SUR_DOSSIER[special];
  return dessin ? base.replace('</svg>', `${dessin}</svg>`) : base;
}
