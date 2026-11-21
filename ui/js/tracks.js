// Треки окна и связь с очередью разбора (события track-progress / track-done).
import { invoke, listen } from './tauri.js';

export const store = {
  tracks: [],
  byId: new Map(),
  selectedId: null,
  nextId: 1,
};

const listeners = new Set();
export function onChange(fn) { listeners.add(fn); }
function emit(kind, track) { listeners.forEach((fn) => fn(kind, track)); }

function makeTrack(path) {
  const parts = path.split(/[\\/]/);
  const file = parts.pop();
  const dot = file.lastIndexOf('.');
  return {
    id: store.nextId++,
    path,
    name: dot > 0 ? file.slice(0, dot) : file,
    ext: dot > 0 ? file.slice(dot + 1).toUpperCase() : '',
    dir: parts.join('\\'),
    status: 'queued',      // queued | running | done | error | stopped
    progress: 0,
    report: null,
    error: null,
    bpmPick: null,         // номер кандидата, выбранного вручную
    keyPick: null,         // тональность 0…23, выбранная вручную
  };
}

// Файлы и папки → новые треки в очередь (повторы пропускаются).
export async function addPaths(paths) {
  if (!paths || !paths.length) return 0;
  const files = (await invoke('scan_paths', { paths })) || [];
  const have = new Set(store.tracks.map((t) => t.path.toLowerCase()));
  const fresh = [];
  for (const p of files) {
    const key = p.toLowerCase();
    if (have.has(key)) continue;
    have.add(key);
    const t = makeTrack(p);
    store.tracks.push(t);
    store.byId.set(t.id, t);
    fresh.push(t);
  }
  if (!fresh.length) return 0;
  emit('add');
  if (store.selectedId == null) select(fresh[0].id);
  await invoke('enqueue_tracks', { tracks: fresh.map((t) => ({ id: t.id, path: t.path })) });
  return fresh.length;
}

export function select(id) {
  if (store.selectedId === id) return;
  store.selectedId = id;
  emit('select', store.byId.get(id));
}

export function selected() {
  return store.byId.get(store.selectedId) || null;
}

export function isBusy() {
  return store.tracks.some((t) => t.status === 'queued' || t.status === 'running');
}

export async function stopAll() {
  await invoke('cancel_queue');
  for (const t of store.tracks) {
    if (t.status === 'queued' || t.status === 'running') {
      t.status = 'stopped';
      t.progress = 0;
    }
  }
  emit('stop');
}

export async function resume() {
  const again = store.tracks.filter((t) => t.status === 'stopped');
  if (!again.length) return;
  again.forEach((t) => { t.status = 'queued'; t.error = null; });
  emit('stop');
  await invoke('enqueue_tracks', { tracks: again.map((t) => ({ id: t.id, path: t.path })) });
}

export async function clearAll() {
  await invoke('cancel_queue');
  store.tracks = [];
  store.byId.clear();
  store.selectedId = null;
  emit('clear');
}

export function remove(id) {
  const i = store.tracks.findIndex((t) => t.id === id);
  if (i < 0) return;
  store.tracks.splice(i, 1);
  store.byId.delete(id);
  if (store.selectedId === id) {
    const next = store.tracks[Math.min(i, store.tracks.length - 1)];
    store.selectedId = next ? next.id : null;
  }
  emit('remove');
  emit('select', selected());
}

export function changed(track) { emit('pick', track); }

export function initEvents() {
  listen('track-progress', (e) => {
    const t = store.byId.get(e.payload.id);
    if (!t || t.status === 'done' || t.status === 'stopped') return;
    t.status = 'running';
    t.progress = e.payload.progress;
    emit('progress', t);
  });
  listen('track-done', (e) => {
    const p = e.payload;
    const t = store.byId.get(p.id);
    if (!t) return;
    if (p.report) {
      t.report = p.report;
      t.status = 'done';
      t.progress = 1;
    } else if (p.error === 'analysis cancelled') {
      t.status = 'stopped';
    } else {
      t.status = 'error';
      t.error = p.error;
    }
    emit('done', t);
  });
}

// Для проверки окна без файлов: готовые треки с правдоподобными отчётами.
export function inject(tracks) {
  for (const t of tracks) {
    const n = makeTrack(t.path);
    Object.assign(n, t);
    store.tracks.push(n);
    store.byId.set(n.id, n);
  }
  emit('add');
  if (store.selectedId == null && store.tracks.length) select(store.tracks[0].id);
}
