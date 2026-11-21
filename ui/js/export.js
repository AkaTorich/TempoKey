// Выгрузка списка в CSV (запятая, точка в числах, UTF-8 с BOM — пишет Rust).
import { store } from './tracks.js';
import { trackTempo, trackKey } from './view.js';
import { keyShort, camelot, openKey, formatTime } from './music.js';
import { invoke, pickSavePath } from './tauri.js';

const q = (v) => {
  const s = String(v ?? '');
  return /[",\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s;
};

export async function exportCsv() {
  const done = store.tracks.filter((t) => t.report);
  if (!done.length) return;
  const path = await pickSavePath('TempoKey.csv');
  if (!path) return;
  const lines = [['File', 'Folder', 'BPM', 'Key', 'Camelot', 'Open Key', 'Tuning A4 (Hz)', 'Tuning (cents)', 'Length', 'Format'].join(',')];
  for (const t of done) {
    const tempo = trackTempo(t);
    const key = trackKey(t);
    const r = t.report;
    lines.push([
      t.name + (t.ext ? '.' + t.ext.toLowerCase() : ''),
      t.dir,
      tempo ? tempo.bpm.toFixed(2) : '',
      keyShort(key.tonic, key.minor).replace('♯', '#').replace('♭', 'b'),
      camelot(key.tonic, key.minor),
      openKey(key.tonic, key.minor),
      r.key.reference_a.toFixed(2),
      r.key.tuning_cents.toFixed(1),
      formatTime(r.duration),
      t.ext,
    ].map(q).join(','));
  }
  await invoke('write_text_file', { path, text: lines.join('\r\n') + '\r\n' });
}
