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

// Familles de fichiers : chaque famille a son dessin et sa couleur, pour se repérer d'un coup d'œil.
const FAMILLES = [
  { famille: 'pdf', ext: 'pdf', vive: '#DC2626', fond: '#FEF2F2' },
  { famille: 'image', ext: 'png jpg jpeg gif webp svg bmp ico heic heif avif tif tiff raw psd', vive: '#DB2777', fond: '#FDF2F8' },
  { famille: 'video', ext: 'mp4 mkv webm avi mov m4v wmv flv', vive: '#7C3AED', fond: '#F5F3FF' },
  { famille: 'audio', ext: 'mp3 flac ogg wav m4a opus aac wma', vive: '#0284C7', fond: '#F0F9FF' },
  { famille: 'archive', ext: 'zip tar gz tgz xz 7z rar bz2 zst', vive: '#B45309', fond: '#FFFBEB' },
  { famille: 'disque', ext: 'iso img dmg vhd vhdx', vive: '#475569', fond: '#F1F5F9' },
  { famille: 'executable', ext: 'exe msi appimage rpm deb flatpakref apk', vive: '#1D4ED8', fond: '#EFF6FF' },
  { famille: 'doc', ext: 'doc docx odt rtf pages', vive: '#2563EB', fond: '#EFF6FF' },
  { famille: 'tableur', ext: 'xls xlsx ods numbers', vive: '#16A34A', fond: '#F0FDF4' },
  { famille: 'presentation', ext: 'ppt pptx odp key', vive: '#EA580C', fond: '#FFF7ED' },
  { famille: 'web', ext: 'html htm url webloc', vive: '#E11D48', fond: '#FFF1F2' },
  { famille: 'code', ext: 'js ts jsx tsx mjs cjs py rs go java c cpp h hpp cs php rb lua kt swift dart vue svelte css scss sass less sql', vive: '#D97706', fond: '#FFFBEB' },
  { famille: 'script', ext: 'sh bash zsh fish ps1 bat cmd', vive: '#334155', fond: '#F1F5F9' },
  { famille: 'donnees', ext: 'json jsonc yaml yml toml xml csv tsv', vive: '#4F46E5', fond: '#EEF2FF' },
  { famille: 'config', ext: 'ini conf cfg env lock gitignore editorconfig properties', vive: '#6B7280', fond: '#F3F4F6' },
  { famille: 'texte', ext: 'txt md markdown log nfo', vive: '#0D9488', fond: '#F0FDFA' },
  { famille: 'police', ext: 'ttf otf woff woff2', vive: '#9333EA', fond: '#FAF5FF' },
];
const PAR_EXTENSION = new Map();
for (const f of FAMILLES) for (const e of f.ext.split(' ')) PAR_EXTENSION.set(e, f);
const DEFAUT = { famille: 'defaut', vive: '#9CA3AF', fond: '#F9FAFB' };

export function typeFichier(extension) {
  const f = PAR_EXTENSION.get(extension) || DEFAUT;
  return { famille: f.famille, vive: f.vive, fond: f.fond, libelle: extension ? extension.toUpperCase().slice(0, 4) : '' };
}
