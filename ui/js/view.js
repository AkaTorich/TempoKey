// Настройки показа (диапазон BPM, обозначение тональности, знаки после точки) и итоговые значения трека.
import { chooseTempo } from './music.js';

const KEY = 'tempokey.view';

export const view = {
  bpmMin: 70,
  bpmMax: 180,
  notation: 'std',
  decimals: 2,
  sort: { col: 'num', desc: false },
};

export function loadView() {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
    Object.assign(view, saved);
  } catch { /* настроек ещё нет */ }
}

export function saveView() {
  try { localStorage.setItem(KEY, JSON.stringify(view)); } catch { /* хранилище недоступно */ }
}

// Темп трека: выбранный вручную кандидат или выбор по диапазону.
export function trackTempo(t) {
  const r = t.report;
  if (!r || !r.tempo.candidates.length) return null;
  if (t.bpmPick != null && r.tempo.candidates[t.bpmPick]) {
    const c = r.tempo.candidates[t.bpmPick];
    return { index: t.bpmPick, bpm: c.bpm, offset: c.offset, confidence: null, manual: true };
  }
  return chooseTempo(r.tempo.candidates, view.bpmMin, view.bpmMax);
}

// Тональность трека: 0…23 (мажор, затем минор).
export function trackKey(t) {
  const r = t.report;
  if (!r) return null;
  const k = t.keyPick != null ? t.keyPick : r.key.tonic + (r.key.minor ? 12 : 0);
  return { index: k, tonic: k % 12, minor: k >= 12, manual: t.keyPick != null };
}
