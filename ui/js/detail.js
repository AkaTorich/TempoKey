// Панель выбранного трека: крупные BPM и тональность, кандидаты (щелчок — выбрать вручную), рисунки, сведения.
import { selected, changed } from './tracks.js';
import { view, trackTempo, trackKey } from './view.js';
import { keyLabel, keyLong, camelot, openKey, relation, formatTime, formatCents } from './music.js';
import { drawGrid, drawWave, drawWheel, drawDrift } from './draw.js';

const $ = (id) => document.getElementById(id);
let shownId = null;
let shownBpm = '';
let shownKey = '';

function setDisp(el, text) {
  el.textContent = text;
  el.dataset.text = text;
}

function burst(el) {
  el.classList.remove('burst');
  void el.offsetWidth;
  el.classList.add('burst');
}

function sweep() {
  const p = $('prism');
  p.classList.remove('go');
  void p.offsetWidth;
  p.classList.add('go');
}

export function initDetail() {
  $('dBpmList').addEventListener('click', (e) => {
    const o = e.target.closest('.opt');
    const t = selected();
    if (!o || !t) return;
    const i = +o.dataset.i;
    const auto = trackTempoAuto(t);
    t.bpmPick = auto && auto.index === i ? null : i;
    changed(t);
  });
  $('dKeyList').addEventListener('click', (e) => {
    const o = e.target.closest('.opt');
    const t = selected();
    if (!o || !t) return;
    const k = +o.dataset.k;
    const auto = t.report.key.tonic + (t.report.key.minor ? 12 : 0);
    t.keyPick = k === auto ? null : k;
    changed(t);
  });
  let pending = false;
  new ResizeObserver(() => {
    if (pending) return;
    pending = true;
    requestAnimationFrame(() => { pending = false; renderDetail(false); });
  }).observe($('detail'));
}

function trackTempoAuto(t) {
  const saved = t.bpmPick;
  t.bpmPick = null;
  const r = trackTempo(t);
  t.bpmPick = saved;
  return r;
}

export function renderDetail(animate) {
  const t = selected();
  const panel = $('detail');
  panel.classList.toggle('has-track', !!t);
  if (!t) { shownId = null; return; }
  const fresh = t.id !== shownId;
  shownId = t.id;
  $('dName').textContent = t.name;
  $('dPath').textContent = t.path;

  panel.classList.toggle('pending', !t.report);
  if (!t.report) {
    renderProcess(t);
    shownBpm = shownKey = '';
    if (fresh && animate) sweep();
    return;
  }

  const r = t.report;
  const tempo = trackTempo(t);
  const key = trackKey(t);

  // BPM
  // целая и дробная части из одной строки — без двойного округления
  const [bpmText, fracDigits] = tempo.bpm.toFixed(2).split('.');
  const frac = '.' + fracDigits;
  setDisp($('dBpm'), bpmText);
  $('dBpmFrac').textContent = frac;
  if (animate && (fresh || shownBpm !== bpmText + frac)) burst($('dBpm'));
  shownBpm = bpmText + frac;
  const conf = tempo.manual ? 1 : tempo.confidence;
  $('dBpmConf').style.width = Math.round(conf * 100) + '%';
  $('dBpmSub').innerHTML = tempo.manual ? 'MANUAL PICK'
    : tempo.folded ? `OUT OF RANGE · <b>${tempo.bpm.toFixed(2)}</b>`
      : `CONFIDENCE <b>${Math.round(conf * 100)}%</b>`;

  // тональность
  const kText = keyLabel(key.tonic, key.minor, view.notation);
  setDisp($('dKey'), kText);
  if (animate && (fresh || shownKey !== kText)) burst($('dKey'));
  shownKey = kText;
  const kConf = key.manual ? 1 : r.key.confidence;
  $('dKeyConf').style.width = Math.round(kConf * 100) + '%';
  $('dKeySub').innerHTML = `<b>${keyLong(key.tonic, key.minor)}</b> · ${camelot(key.tonic, key.minor)} · ${openKey(key.tonic, key.minor)}`;

  // кандидаты темпа
  const top = r.tempo.candidates.slice(0, 6);
  const maxSal = Math.max(...top.map((c) => c.salience), 1e-9);
  $('dBpmList').innerHTML = top.map((c, i) => {
    const on = i === tempo.index && !tempo.folded;
    const out = c.bpm < view.bpmMin - 0.005 || c.bpm > view.bpmMax + 0.005;
    return `<div class="opt${on ? ' on' : ''}" data-i="${i}" role="option"${out ? ' style="opacity:.55"' : ''}>` +
      `<span class="val">${c.bpm.toFixed(2)}</span>` +
      `<span class="meter"><i style="width:${Math.round((c.salience / maxSal) * 100)}%"></i></span>` +
      `<span class="rel">${on ? '' : relation(c.bpm, tempo.bpm)}</span></div>`;
  }).join('');

  // кандидаты тональности
  const order = r.key.scores.map((s, k) => [s, k]).sort((a, b) => b[0] - a[0]).slice(0, 6);
  const lo = Math.min(...r.key.scores);
  const hi = Math.max(...r.key.scores);
  const alt = view.notation === 'camelot' ? 'std' : 'camelot';
  $('dKeyList').innerHTML = order.map(([s, k]) => {
    const tonic = k % 12;
    const minor = k >= 12;
    return `<div class="opt${k === key.index ? ' on' : ''}" data-k="${k}" role="option">` +
      `<span class="val">${keyLabel(tonic, minor, view.notation)}</span>` +
      `<span class="meter"><i style="width:${Math.round(((s - lo) / (hi - lo || 1)) * 100)}%"></i></span>` +
      `<span class="rel alt">${keyLabel(tonic, minor, alt)}</span></div>`;
  }).join('');

  // рисунки
  $('dGridAt').textContent = r.tempo.excerpt.length ? '@ ' + formatTime(r.tempo.excerpt_start) : '';
  drawGrid($('cvGrid'), r.tempo, tempo);
  drawWave($('cvWave'), r);
  drawWheel($('cvWheel'), r.key.chroma, key, keyLabel(key.tonic, key.minor, view.notation));
  const peak = drawDrift($('cvDrift'), r.tempo, tempo.bpm);
  $('dDrift').textContent = peak == null ? '—' : '±' + peak.toFixed(2);

  const ch = r.channels === 1 ? 'MONO' : r.channels === 2 ? 'STEREO' : r.channels + ' CH';
  $('dMeta').innerHTML =
    `<dt>TUNING A4</dt><dd class="hl">${r.key.reference_a.toFixed(1)} Hz</dd>` +
    `<dt>OFFSET</dt><dd>${formatCents(r.key.tuning_cents)}</dd>` +
    `<dt>FIRST BEAT</dt><dd>${tempo.offset.toFixed(3)} s</dd>` +
    `<dt>LENGTH</dt><dd>${formatTime(r.duration)}</dd>` +
    `<dt>FORMAT</dt><dd>${t.ext} · ${(r.sample_rate / 1000).toFixed(1)} kHz · ${ch}</dd>`;

  if (fresh && animate) sweep();
}

// Ход разбора: стадии по долям, которые шлёт Rust (чтение 0–0,33, темп 0,33–0,6, тональность 0,6–1).
const STAGE_START = [0, 0.33, 0.6];

function renderProcess(t) {
  const p = t.status === 'running' ? t.progress : 0;
  const stage = p >= STAGE_START[2] ? 2 : p >= STAGE_START[1] ? 1 : 0;
  document.querySelectorAll('#detail .stage').forEach((el, i) => {
    el.classList.toggle('done', t.status === 'running' && i < stage);
    el.classList.toggle('active', t.status === 'running' && i === stage);
    el.classList.toggle('failed', t.status === 'error' && i === 0);
  });
  const bar = $('dProg');
  bar.style.setProperty('--p', Math.round(p * 100));
  bar.querySelector('.pct').textContent = Math.round(p * 100) + '%';
  bar.classList.toggle('active', t.status === 'running');
  bar.classList.toggle('error', t.status === 'error');
  const msg = $('dProcMsg');
  msg.classList.toggle('err', t.status === 'error');
  msg.textContent = t.status === 'running' ? ['DECODING AUDIO', 'MEASURING TEMPO', 'DETECTING KEY'][stage]
    : t.status === 'queued' ? 'IN QUEUE'
      : t.status === 'stopped' ? 'STOPPED'
        : (t.error || 'ERROR').toUpperCase();
}
