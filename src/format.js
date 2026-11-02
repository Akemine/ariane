// Mise en forme : tailles, dates, types de fichiers.

export function echapper(texte) {
  return String(texte ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

/** Minuscules et sans accents, pour la recherche. */
export function normaliser(texte) {
  return String(texte).normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
}

export function pluriel(n, singulier, plurielForme) {
  return `${n} ${n > 1 ? plurielForme : singulier}`;
}

export function taille(octets) {
  if (octets == null) return '—';
  const unites = ['o', 'Ko', 'Mo', 'Go', 'To'];
  let valeur = octets;
  let u = 0;
  while (valeur >= 1024 && u < unites.length - 1) {
    valeur /= 1024;
    u++;
  }
  const affiche = u === 0 ? String(valeur) : valeur < 10 ? valeur.toFixed(1) : String(Math.round(valeur));
  return `${affiche.replace('.', ',')} ${unites[u]}`;
}

export function elements(n) {
  if (n == null) return '—';
  if (n === 0) return 'vide';
  if (n >= 10000) return '10 000+ élém.';
  return `${n} élém.`;
}

const MOIS = ['janv.', 'févr.', 'mars', 'avr.', 'mai', 'juin', 'juil.', 'août', 'sept.', 'oct.', 'nov.', 'déc.'];
const heure = (d) => `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;

export function dateRelative(ms) {
  if (ms == null) return '—';
  const d = new Date(ms);
  const maintenant = new Date();
  const secondes = (maintenant - d) / 1000;
  if (secondes < 60 && secondes > -60) return "à l'instant";
  if (secondes < 3600 && secondes > 0) return `il y a ${Math.floor(secondes / 60)} min`;
  const debutJour = new Date(maintenant.getFullYear(), maintenant.getMonth(), maintenant.getDate());
  if (d >= debutJour) return secondes < 6 * 3600 ? `il y a ${Math.floor(secondes / 3600)} h` : `auj. ${heure(d)}`;
  const hier = new Date(debutJour);
  hier.setDate(hier.getDate() - 1);
  if (d >= hier) return `hier ${heure(d)}`;
  const jours = Math.floor((debutJour - d) / 86400000) + 1;
  if (jours < 7) return `il y a ${jours} j`;
  const base = `${d.getDate()} ${MOIS[d.getMonth()]}`;
  return d.getFullYear() === maintenant.getFullYear() ? base : `${base} ${d.getFullYear()}`;
}

export function dateComplete(ms) {
  if (ms == null) return '';
  return new Date(ms).toLocaleString('fr-FR', { dateStyle: 'full', timeStyle: 'short' });
}

// Familles de fichiers : chaque famille a sa couleur, pour se repérer d'un coup d'œil.
const FAMILLES = [
  { ext: 'js ts jsx tsx mjs cjs py rs go java c cpp h hpp cs php rb sh bash zsh fish lua kt swift dart vue svelte', fond: '#FEF3C7', texte: '#92400E' },
  { ext: 'html htm css scss sass less', fond: '#FFE4E6', texte: '#9F1239' },
  { ext: 'json yaml yml toml xml csv tsv ini conf cfg env lock sql', fond: '#E0E7FF', texte: '#3730A3' },
  { ext: 'md txt rtf log', fond: '#CCFBF1', texte: '#115E59' },
  { ext: 'pdf', fond: '#FEE2E2', texte: '#991B1B' },
  { ext: 'doc docx odt', fond: '#DBEAFE', texte: '#1E40AF' },
  { ext: 'xls xlsx ods', fond: '#DCFCE7', texte: '#166534' },
  { ext: 'ppt pptx odp key', fond: '#FFEDD5', texte: '#9A3412' },
  { ext: 'png jpg jpeg gif webp svg bmp ico heic heif avif tif tiff raw', fond: '#FCE7F3', texte: '#9D174D' },
  { ext: 'mp4 mkv webm avi mov m4v wmv', fond: '#EDE9FE', texte: '#5B21B6' },
  { ext: 'mp3 flac ogg wav m4a opus aac', fond: '#E0F2FE', texte: '#075985' },
  { ext: 'zip tar gz tgz xz 7z rar bz2 zst', fond: '#EDE7DF', texte: '#57452F' },
  { ext: 'iso img rpm deb appimage flatpakref exe msi dmg', fond: '#E7E5E4', texte: '#44403C' },
];
const PAR_EXTENSION = new Map();
for (const f of FAMILLES) for (const e of f.ext.split(' ')) PAR_EXTENSION.set(e, f);

export function typeFichier(extension) {
  const famille = PAR_EXTENSION.get(extension) || { fond: '#F1F1F4', texte: '#3F3F46' };
  const libelle = extension ? extension.toUpperCase().slice(0, 4) : 'FIC';
  return { libelle, fond: famille.fond, texte: famille.texte };
}
