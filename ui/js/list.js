// Список треков: строки своей формы, сортировка по столбцам, выбор мышью и стрелками, Delete — убрать.
import { store, select, remove } from './tracks.js';
import { view, saveView, trackTempo, trackKey } from './view.js';
import { keyLabel, keyOrder, formatBpm, formatTime, formatCents } from './music.js';

const STATUS_ORDER = { running: 0, queued: 1, done: 2, stopped: 3, error: 4 };

let listEl;
let headEl;
const rows = new Map();

export function initList() {
  listEl = document.getElementById('trackList');
  headEl = document.getElementById('listHead');

  headEl.addEventListener('click', (e) => {
    const h = e.target.closest('[data-sort]');
    if (!h) return;
    const col = h.dataset.sort;
    view.sort = { col, desc: view.sort.col === col ? !view.sort.desc : false };
    saveView();
    renderAll();
  });

  listEl.addEventListener('mousedown', (e) => {
    const r = e.target.closest('.row');
    if (r) select(+r.dataset.id);
  });

  listEl.addEventListener('keydown', (e) => {
    const order = sorted();
    const i = order.findIndex((t) => t.id === store.selectedId);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = order[Math.max(0, Math.min(order.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next) select(next.id);
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      const next = e.key === 'Home' ? order[0] : order[order.length - 1];
      if (next) select(next.id);
    } else if (e.key === 'Delete' && i >= 0) {
      remove(order[i].id);
    }
  });
}

function sortValue(t, col) {
  switch (col) {
    case 'name': return t.name.toLowerCase();
    case 'bpm': { const b = trackTempo(t); return b ? b.bpm : Infinity; }
    case 'key': { const k = trackKey(t); return k ? keyOrder(k.tonic, k.minor) : Infinity; }
    case 'tune': return t.report ? t.report.key.tuning_cents : Infinity;
    case 'time': return t.report ? t.report.duration : Infinity;
    case 'status': return STATUS_ORDER[t.status];
    default: return t.id;
  }
}

function sorted() {
  const { col, desc } = view.sort;
  const list = store.tracks.slice();
  list.sort((a, b) => {
    const x = sortValue(a, col);
    const y = sortValue(b, col);
    const c = x < y ? -1 : x > y ? 1 : a.id - b.id;
    return desc ? -c : c;
  });
  return list;
}

function statusHtml(t) {
  switch (t.status) {
    case 'running': {
      const p = Math.round(t.progress * 100);
      return `<div class="rowprog"><div class="pbar"><i style="width:${p}%"></i></div><b>${p}%</b></div>`;
    }
    case 'queued': return 'QUEUED';
    case 'done': return 'READY';
    case 'stopped': return 'STOPPED';
    default: return 'ERROR';
  }
}

function fill(row, t, index) {
  const tempo = trackTempo(t);
  const key = trackKey(t);
  row.className = 'row' + (t.id === store.selectedId ? ' sel' : '');
  row.innerHTML =
    `<span class="c-num">${String(index + 1).padStart(2, '0')}</span>` +
    `<span class="c-name"><b></b><em>${t.ext}</em></span>` +
    `<span class="c-bpm${tempo && tempo.folded ? ' c-dim' : ''}">${tempo ? formatBpm(tempo.bpm, view.decimals) : '—'}</span>` +
    `<span class="c-key">${key ? keyLabel(key.tonic, key.minor, view.notation) : '—'}</span>` +
    `<span class="c-tune">${t.report ? formatCents(t.report.key.tuning_cents) : ''}</span>` +
    `<span class="c-time">${t.report ? formatTime(t.report.duration) : ''}</span>` +
    `<span class="c-stat ${t.status}">${statusHtml(t)}</span>`;
  row.querySelector('b').textContent = t.name;
  row.title = t.error ? `${t.path}\n${t.error}` : t.path;
}

export function renderAll() {
  headEl.querySelectorAll('[data-sort]').forEach((h) => {
    h.classList.toggle('sorted', h.dataset.sort === view.sort.col);
    h.classList.toggle('desc', h.dataset.sort === view.sort.col && view.sort.desc);
  });
  const frag = document.createDocumentFragment();
  rows.clear();
  sorted().forEach((t, i) => {
    const row = document.createElement('div');
    row.dataset.id = t.id;
    row.setAttribute('role', 'option');
    fill(row, t, i);
    rows.set(t.id, { row, index: i });
    frag.appendChild(row);
  });
  listEl.replaceChildren(frag);
  document.getElementById('trackCount').textContent = store.tracks.length;
  document.body.classList.toggle('no-tracks', store.tracks.length === 0);
}

// Обновление одной строки (ход разбора) без перестройки списка.
export function renderOne(t) {
  const r = rows.get(t.id);
  if (!r) return;
  if (t.status === 'running') {
    const bar = r.row.querySelector('.pbar i');
    if (bar) {
      const p = Math.round(t.progress * 100);
      bar.style.width = p + '%';
      r.row.querySelector('.rowprog b').textContent = p + '%';
      return;
    }
  }
  fill(r.row, t, r.index);
}

export function markSelection() {
  rows.forEach(({ row }, id) => row.classList.toggle('sel', id === store.selectedId));
  const r = rows.get(store.selectedId);
  if (r) r.row.scrollIntoView({ block: 'nearest' });
}

// Строки, которые надо будет пересортировать: если сортировка по столбцу, который изменился у готового трека.
export function sortDependsOnResult() {
  return ['bpm', 'key', 'tune', 'time', 'status'].includes(view.sort.col);
}
