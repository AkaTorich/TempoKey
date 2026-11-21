// Музыкальные обозначения и выбор темпа из кандидатов.

const MAJOR = ['C', 'D♭', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];
const MINOR = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'B♭', 'B'];
export const PITCH = ['C', 'C♯', 'D', 'E♭', 'E', 'F', 'F♯', 'G', 'A♭', 'A', 'B♭', 'B'];

export const NOTATIONS = [
  { value: 'std', label: 'STANDARD · Am', short: 'STANDARD' },
  { value: 'camelot', label: 'CAMELOT · 8A', short: 'CAMELOT' },
  { value: 'open', label: 'OPEN KEY · 1m', short: 'OPEN KEY' },
];

export const DECIMALS = [
  { value: 0, label: '128' },
  { value: 1, label: '128.0' },
  { value: 2, label: '128.00' },
];

export function keyShort(tonic, minor) {
  return minor ? MINOR[tonic] + 'm' : MAJOR[tonic];
}

export function keyLong(tonic, minor) {
  return (minor ? MINOR[tonic] : MAJOR[tonic]) + (minor ? ' minor' : ' major');
}

// Номер на круге Камелота: до мажор = 8B, ля минор = 8A; шаг по квинтам — шаг по кругу.
function camelotNumber(tonic, minor) {
  const major = minor ? (tonic + 3) % 12 : tonic;
  return ((major * 7 + 7) % 12) + 1;
}

export function camelot(tonic, minor) {
  return camelotNumber(tonic, minor) + (minor ? 'A' : 'B');
}

export function openKey(tonic, minor) {
  return (((camelotNumber(tonic, minor) - 8 + 12) % 12) + 1) + (minor ? 'm' : 'd');
}

export function keyLabel(tonic, minor, notation) {
  if (notation === 'camelot') return camelot(tonic, minor);
  if (notation === 'open') return openKey(tonic, minor);
  return keyShort(tonic, minor);
}

// Ключ сортировки по кругу Камелота: 1A, 1B, 2A …
export function keyOrder(tonic, minor) {
  return camelotNumber(tonic, minor) * 2 + (minor ? 0 : 1);
}

const round2 = (v) => Math.round(v * 100) / 100;

// Темп из кандидатов в диапазоне [lo, hi]: выраженность × мягкое предпочтение середины диапазона
// (логнормальное: центр — среднее геометрическое границ, ширина — половина диапазона в октавах).
// Нет никого в диапазоне — сильнейший кандидат переносится в диапазон октавами.
export function chooseTempo(cands, lo, hi) {
  if (!cands || !cands.length) return null;
  const center = Math.sqrt(lo * hi);
  const sigma = Math.max(Math.log2(hi / lo) / 2, 0.1);
  const inside = [];
  cands.forEach((k, i) => {
    const b = round2(k.bpm);
    if (b < lo || b > hi) return;
    const z = Math.log2(k.bpm / center) / sigma;
    inside.push({ index: i, bpm: k.bpm, offset: k.offset, score: k.salience * Math.exp(-0.5 * z * z) });
  });
  if (inside.length) {
    inside.sort((a, b) => b.score - a.score);
    const conf = inside.length > 1 ? 1 - inside[1].score / inside[0].score : 1;
    return { index: inside[0].index, bpm: inside[0].bpm, offset: inside[0].offset, confidence: conf, folded: false };
  }
  let b = cands[0].bpm;
  while (b < lo) b *= 2;
  while (b > hi) b /= 2;
  if (b < lo) b = cands[0].bpm;
  return { index: 0, bpm: b, offset: cands[0].offset, confidence: 0, folded: true };
}

// Отношение кандидата к выбранному темпу: ×2, ÷2, ×3/2 …
export function relation(bpm, ref) {
  const r = bpm / ref;
  const known = [[2, '×2'], [0.5, '÷2'], [4, '×4'], [0.25, '÷4'], [1.5, '×3/2'], [2 / 3, '×2/3'], [3, '×3'], [1 / 3, '÷3'], [0.75, '×3/4'], [4 / 3, '×4/3']];
  for (const [v, label] of known) if (Math.abs(r / v - 1) < 0.01) return label;
  return '';
}

export function formatBpm(bpm, decimals) {
  return bpm.toFixed(decimals);
}

export function formatTime(sec) {
  if (!isFinite(sec)) return '';
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, '0');
  return h ? `${h}:${String(m).padStart(2, '0')}:${r}` : `${m}:${r}`;
}

export function formatCents(c) {
  const v = Math.round(c * 10) / 10;
  return (v > 0 ? '+' : v < 0 ? '−' : '±') + Math.abs(v).toFixed(1) + ' ct';
}
