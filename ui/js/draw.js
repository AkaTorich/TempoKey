// Рисунки панели трека с дисперсией: каждая линия кладётся трижды — розовая влево, голубая вправо
// (сложение света), сердцевина поверх.
import { PITCH } from './music.js';

const PINK = 'rgba(255, 42, 109, .9)';
const CYAN = 'rgba(5, 217, 232, .9)';
const YELLOW = '#fcee0a';

function prep(cv) {
  const dpr = window.devicePixelRatio || 1;
  const w = cv.clientWidth;
  const h = cv.clientHeight;
  if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) {
    cv.width = Math.round(w * dpr);
    cv.height = Math.round(h * dpr);
  }
  const ctx = cv.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  return { ctx, w, h };
}

// path(ctx) строит путь; d — расхождение, core — цвет сердцевины.
function dispersed(ctx, path, d, width, core) {
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  ctx.lineWidth = width;
  ctx.lineJoin = 'round';
  ctx.translate(-d, 0);
  ctx.strokeStyle = PINK;
  path(ctx);
  ctx.stroke();
  ctx.translate(2 * d, 0);
  ctx.strokeStyle = CYAN;
  path(ctx);
  ctx.stroke();
  ctx.restore();
  ctx.save();
  ctx.lineWidth = width * 0.85;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = core;
  path(ctx);
  ctx.stroke();
  ctx.restore();
}

function gridLines(ctx, w, h) {
  ctx.save();
  ctx.strokeStyle = 'rgba(5, 217, 232, .08)';
  ctx.lineWidth = 1;
  for (let x = 0.5; x < w; x += 24) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke(); }
  for (let y = 0.5; y < h; y += 16) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke(); }
  ctx.restore();
}

export function clear(cv) { prep(cv); }

// Сетка долей поверх самого насыщенного отрезка огибающей атак.
export function drawGrid(cv, tempo, choice) {
  const { ctx, w, h } = prep(cv);
  gridLines(ctx, w, h);
  const ex = tempo.excerpt;
  if (!ex || !ex.length || !choice) return;
  const dur = ex.length / tempo.frame_rate;
  const start = tempo.excerpt_start;
  const base = h - 8;
  const top = 14;
  const X = (i) => (i / (ex.length - 1)) * w;
  const Y = (v) => base - v * (base - top);

  // доли
  const period = 60 / choice.bpm;
  let k = Math.ceil((start - choice.offset) / period);
  ctx.save();
  for (let t = choice.offset + k * period; t < start + dur; t += period, k++) {
    const x = ((t - start) / dur) * w;
    const bar = ((k % 4) + 4) % 4 === 0;
    ctx.fillStyle = bar ? 'rgba(252, 238, 10, .16)' : 'rgba(5, 217, 232, .08)';
    ctx.fillRect(x - 1, top - 4, 2, base - top + 4);
    ctx.fillStyle = bar ? YELLOW : 'rgba(5, 217, 232, .8)';
    ctx.beginPath();
    ctx.moveTo(x - 4, 2);
    ctx.lineTo(x + 4, 2);
    ctx.lineTo(x, 8);
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();

  // огибающая: заливка и контур с дисперсией
  const path = (c) => {
    c.beginPath();
    c.moveTo(0, base);
    for (let i = 0; i < ex.length; i++) c.lineTo(X(i), Y(ex[i]));
    c.lineTo(w, base);
  };
  const grad = ctx.createLinearGradient(0, top, 0, base);
  grad.addColorStop(0, 'rgba(255, 42, 109, .45)');
  grad.addColorStop(1, 'rgba(43, 69, 255, .05)');
  ctx.fillStyle = grad;
  path(ctx);
  ctx.fill();
  dispersed(ctx, path, 1.2, 1.3, '#fff6a8');
}

// Обзор трека: волна (минимум/максимум) с дисперсией и рамка отрезка сетки.
export function drawWave(cv, report) {
  const { ctx, w, h } = prep(cv);
  const wf = report.waveform;
  if (!wf || !wf.length) return;
  const mid = h / 2;
  const cols = Math.max(1, Math.floor(w));
  const bars = (c) => {
    c.beginPath();
    for (let x = 0; x < cols; x++) {
      const p = wf[Math.min(wf.length - 1, Math.floor((x / cols) * wf.length))];
      c.moveTo(x + 0.5, mid - p[1] * (mid - 3));
      c.lineTo(x + 0.5, mid - p[0] * (mid - 3));
    }
  };
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  ctx.lineWidth = 1;
  ctx.translate(-1.5, 0);
  ctx.strokeStyle = 'rgba(255, 42, 109, .55)';
  bars(ctx);
  ctx.stroke();
  ctx.translate(3, 0);
  ctx.strokeStyle = 'rgba(5, 217, 232, .55)';
  bars(ctx);
  ctx.stroke();
  ctx.restore();
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, '#6f86ff');
  g.addColorStop(0.5, '#dff6ff');
  g.addColorStop(1, '#6f86ff');
  ctx.strokeStyle = g;
  ctx.lineWidth = 1;
  bars(ctx);
  ctx.stroke();

  const t = report.tempo;
  if (t.excerpt && t.excerpt.length) {
    const x0 = (t.excerpt_start / report.duration) * w;
    const x1 = ((t.excerpt_start + t.excerpt.length / t.frame_rate) / report.duration) * w;
    ctx.strokeStyle = YELLOW;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(x0 + 5, 1); ctx.lineTo(x0, 1); ctx.lineTo(x0, h - 1); ctx.lineTo(x0 + 5, h - 1);
    ctx.moveTo(x1 - 5, 1); ctx.lineTo(x1, 1); ctx.lineTo(x1, h - 1); ctx.lineTo(x1 - 5, h - 1);
    ctx.stroke();
    ctx.fillStyle = 'rgba(252, 238, 10, .08)';
    ctx.fillRect(x0, 0, x1 - x0, h);
  }
}

// Колесо высот по кругу квинт: длина сектора — доля ступени; тоника жёлтая, ступени лада голубые.
const FIFTHS = [0, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5];
const MAJOR_SCALE = [0, 2, 4, 5, 7, 9, 11];
const MINOR_SCALE = [0, 2, 3, 5, 7, 8, 10];

export function drawWheel(cv, chroma, key, label) {
  const { ctx, w, h } = prep(cv);
  const cx = w / 2;
  const cy = h / 2;
  const R = Math.min(w, h) / 2 - 16;
  const r0 = R * 0.36;
  const scale = new Set((key.minor ? MINOR_SCALE : MAJOR_SCALE).map((s) => (s + key.tonic) % 12));
  const step = (Math.PI * 2) / 12;

  ctx.strokeStyle = 'rgba(5, 217, 232, .18)';
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.stroke();
  ctx.beginPath(); ctx.arc(cx, cy, r0, 0, Math.PI * 2); ctx.stroke();

  FIFTHS.forEach((pc, i) => {
    const a0 = -Math.PI / 2 + i * step - step / 2 + 0.05;
    const a1 = a0 + step - 0.1;
    const r = r0 + (R - r0) * Math.max(0.04, chroma[pc]);
    const wedge = (c) => {
      c.beginPath();
      c.arc(cx, cy, r, a0, a1);
      c.arc(cx, cy, r0 + 2, a1, a0, true);
      c.closePath();
    };
    const tonic = pc === key.tonic;
    const inScale = scale.has(pc);
    ctx.fillStyle = tonic ? 'rgba(252, 238, 10, .85)' : inScale ? 'rgba(5, 217, 232, .45)' : 'rgba(118, 133, 176, .18)';
    wedge(ctx);
    ctx.fill();
    if (tonic || inScale) dispersed(ctx, wedge, 1.2, 1, tonic ? YELLOW : 'rgba(223, 246, 255, .7)');

    const am = (a0 + a1) / 2;
    ctx.fillStyle = tonic ? YELLOW : inScale ? '#dff6ff' : 'rgba(118, 133, 176, .6)';
    ctx.font = `600 ${tonic ? 11 : 10}px TempoKey, Bahnschrift, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(PITCH[pc], cx + Math.cos(am) * (R + 9), cy + Math.sin(am) * (R + 9));
  });

  ctx.font = `700 ${label.length > 3 ? 17 : 21}px TempoKey, Bahnschrift, sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.globalCompositeOperation = 'lighter';
  ctx.fillStyle = PINK; ctx.fillText(label, cx - 1.6, cy);
  ctx.fillStyle = CYAN; ctx.fillText(label, cx + 1.6, cy);
  ctx.globalCompositeOperation = 'source-over';
  ctx.fillStyle = '#fff';
  ctx.fillText(label, cx, cy);
}

// Местный темп по окнам: отклонение от выбранного темпа в ударах в минуту. Возвращает наибольшее отклонение.
export function drawDrift(cv, tempo, bpm) {
  const { ctx, w, h } = prep(cv);
  gridLines(ctx, w, h);
  const d = tempo.drift || [];
  const vals = d.map((r) => (r == null ? null : (r - 1) * bpm));
  const known = vals.filter((v) => v != null);
  const mid = h / 2;
  ctx.strokeStyle = 'rgba(252, 238, 10, .35)';
  ctx.setLineDash([4, 4]);
  ctx.beginPath(); ctx.moveTo(0, mid + 0.5); ctx.lineTo(w, mid + 0.5); ctx.stroke();
  ctx.setLineDash([]);
  if (known.length < 2) return null;
  const peak = Math.max(...known.map(Math.abs));
  const span = Math.max(0.25, peak * 1.3);
  const X = (i) => 6 + (i / (vals.length - 1)) * (w - 12);
  const Y = (v) => mid - (v / span) * (mid - 8);
  const path = (c) => {
    c.beginPath();
    let pen = false;
    vals.forEach((v, i) => {
      if (v == null) { pen = false; return; }
      if (pen) c.lineTo(X(i), Y(v)); else c.moveTo(X(i), Y(v));
      pen = true;
    });
  };
  dispersed(ctx, path, 1.4, 2, YELLOW);
  ctx.fillStyle = YELLOW;
  vals.forEach((v, i) => { if (v != null) ctx.fillRect(X(i) - 1.5, Y(v) - 1.5, 3, 3); });
  return peak;
}
