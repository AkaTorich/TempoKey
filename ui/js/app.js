// Сборка окна: кнопки окна и панели, поля диапазона, списки выбора, бросание файлов, строка состояния.
import { appWindow, inTauri, invoke, listen, pickFiles, pickFolders } from './tauri.js';
import { initGuard } from './guard.js';
import { initShape } from './shape.js';
import { makeSelect } from './select.js';
import { NOTATIONS, DECIMALS } from './music.js';
import { view, loadView, saveView } from './view.js';
import { store, onChange, addPaths, stopAll, resume, clearAll, isBusy, initEvents, selected } from './tracks.js';
import { initList, renderAll, renderOne, markSelection, sortDependsOnResult } from './list.js';
import { initDetail, renderDetail } from './detail.js';
import { exportCsv } from './export.js';
import { demo } from './demo.js';

const $ = (id) => document.getElementById(id);

initGuard();
loadView();
const shape = initShape();
initList();
initDetail();

// ── кнопки окна ──
$('btnMin').addEventListener('click', () => inTauri && appWindow.minimize());
$('btnMax').addEventListener('click', () => inTauri && appWindow.toggleMaximize());
$('btnClose').addEventListener('click', () => inTauri && appWindow.close());

// ── панель ──
$('btnFiles').addEventListener('click', async () => addPaths(await pickFiles()));
$('btnFolder').addEventListener('click', async () => addPaths(await pickFolders()));
$('btnStop').addEventListener('click', () => (isBusy() ? stopAll() : resume()));
$('btnClear').addEventListener('click', () => clearAll());
$('btnExport').addEventListener('click', () => exportCsv());

// ── диапазон BPM ──
const fMin = $('bpmMin');
const fMax = $('bpmMax');
fMin.value = view.bpmMin;
fMax.value = view.bpmMax;

function applyRange() {
  const lo = Number(fMin.value);
  const hi = Number(fMax.value);
  const ok = Number.isFinite(lo) && Number.isFinite(hi) && lo >= 20 && hi <= 400 && hi > lo;
  fMin.parentElement.classList.toggle('bad', !ok);
  fMax.parentElement.classList.toggle('bad', !ok);
  if (!ok || (lo === view.bpmMin && hi === view.bpmMax)) return;
  view.bpmMin = lo;
  view.bpmMax = hi;
  saveView();
  renderAll();
  renderDetail(true);
}
for (const f of [fMin, fMax]) {
  f.addEventListener('input', applyRange);
  f.addEventListener('wheel', (e) => {
    e.preventDefault();
    f.value = Number(f.value || 0) + (e.deltaY < 0 ? 1 : -1);
    applyRange();
  }, { passive: false });
  f.addEventListener('keydown', (e) => { if (e.key === 'Enter') f.blur(); });
}

// ── списки выбора ──
makeSelect($('selNotation'), NOTATIONS, view.notation, (v) => {
  view.notation = v;
  saveView();
  renderAll();
  renderDetail(true);
});
makeSelect($('selDecimals'), DECIMALS, view.decimals, (v) => {
  view.decimals = v;
  saveView();
  renderAll();
});

// ── бросание файлов и папок на окно ──
listen('tauri://drag-enter', () => document.body.classList.add('dragging'));
listen('tauri://drag-leave', () => document.body.classList.remove('dragging'));
listen('tauri://drag-drop', (e) => {
  document.body.classList.remove('dragging');
  addPaths(e.payload.paths || []);
});

// ── строка состояния и доступность кнопок ──
function updateChrome() {
  const total = store.tracks.length;
  const busy = isBusy();
  const stopped = store.tracks.some((t) => t.status === 'stopped');
  const finished = store.tracks.filter((t) => t.status === 'done' || t.status === 'error' || t.status === 'stopped').length;
  const running = store.tracks.find((t) => t.status === 'running');
  const errors = store.tracks.filter((t) => t.status === 'error').length;

  const stop = $('btnStop');
  stop.disabled = !busy && !stopped;
  stop.classList.toggle('p', busy || !stopped);
  stop.classList.toggle('c', !busy && stopped);
  stop.querySelector('span').textContent = busy || !stopped ? 'STOP' : 'RESUME';
  $('btnClear').disabled = total === 0;
  $('btnExport').disabled = !store.tracks.some((t) => t.report);

  const part = total ? (finished + (running ? running.progress : 0)) / total : 0;
  const prog = $('sbProg');
  prog.style.setProperty('--p', Math.round(part * 100));
  $('sbPct').textContent = Math.round(part * 100) + '%';
  prog.classList.toggle('active', busy);
  const err = errors ? ` · <em>${errors} ERROR${errors > 1 ? 'S' : ''}</em>` : '';
  const now = running ? ` · <i></i>` : '';
  $('sbText').innerHTML = !total ? 'IDLE'
    : busy ? `ANALYZING <b>${Math.min(finished + 1, total)}</b> / ${total}${now}${err}`
      : `DONE · <b>${total}</b> TRACK${total > 1 ? 'S' : ''}${err}`;
  if (running) $('sbText').querySelector('i').textContent = running.name;
}

onChange((kind, t) => {
  switch (kind) {
    case 'add':
    case 'clear':
    case 'remove':
    case 'stop':
      renderAll();
      renderDetail(kind !== 'stop');
      break;
    case 'progress':
      renderOne(t);
      if (t === selected()) renderDetail(false);
      updateChrome();
      return;
    case 'done':
      if (sortDependsOnResult()) renderAll(); else renderOne(t);
      if (t === selected()) renderDetail(true);
      break;
    case 'select':
      markSelection();
      renderDetail(true);
      break;
    case 'pick':
      renderOne(t);
      renderDetail(true);
      break;
  }
  updateChrome();
});

initEvents();

// Сборка окна шагами для заставки (окно пока скрыто: только обещания, без таймеров — их скрытое окно тормозит).
async function boot() {
  const step = (progress, label) => (inTauri ? invoke('boot_step', { progress, label }).catch(() => {}) : Promise.resolve());
  await step(0.12, 'INITIALIZING CORE');
  await step(0.3, 'LOADING FONT');
  try {
    await Promise.all([document.fonts.load('400 16px TempoKey'), document.fonts.load('700 16px TempoKey')]);
  } catch { /* останется запасной шрифт */ }
  await document.fonts.ready;
  await step(0.52, 'BUILDING INTERFACE');
  renderAll();
  updateChrome();
  await step(0.72, 'DRAWING PANELS');
  renderDetail(false);
  await step(0.88, 'SHAPING WINDOW');
  await shape.ready;
  await step(1, 'READY');
  $('trackList').focus();
}
boot();

// для проверки окна через порт отладки WebView2 (tools/check)
window.App = { demo, addPaths, store, view };
