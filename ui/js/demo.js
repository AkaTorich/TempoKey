// Проверка окна без файлов (tools/check/shots.ps1 вызывает App.demo()): треки с правдоподобными отчётами
// во всех состояниях — готовые, идёт разбор, в очереди, ошибка.
import { inject } from './tracks.js';

const NAMES = [
  ['Chrome Rain', 128, 9, true], ['Neon Requiem', 145, 6, true], ['Synthetic Dawn', 87.5, 2, false],
  ['Pulse Engine', 174, 4, true], ['Glass Circuit', 124.37, 10, true], ['Data Ghost', 140, 7, true],
  ['Midnight Protocol', 122, 0, false], ['Voltage Bloom', 136.5, 1, true], ['Hex Runner', 150, 11, true],
  ['Signal Decay', 95, 5, false], ['Static Saints', 132, 3, true], ['Orbital Fade', 118, 8, false],
];

const MAJOR = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MINOR = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

function rng(seed) {
  let s = Math.imul(seed, 2654435761) >>> 0;
  return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 4294967296);
}

function report(name, bpm, tonic, minor, seed) {
  const r = rng(seed);
  const duration = 240 + r() * 180;
  const fr = 200;
  const offset = r() * (60 / bpm);
  const exLen = 8 * fr;
  const exStart = 60 + r() * 60;
  const excerpt = Array.from({ length: exLen }, (_, i) => {
    const t = exStart + i / fr;
    const ph = ((t - offset) / (60 / bpm)) % 1;
    const half = ((t - offset) / (30 / bpm)) % 1;
    return Math.max(0, Math.exp(-ph * 22) + 0.35 * Math.exp(-half * 30) * (half > 0.05 ? 1 : 0) + r() * 0.08 - 0.04);
  });
  const peak = Math.max(...excerpt);
  const prof = minor ? MINOR : MAJOR;
  const chroma = Array.from({ length: 12 }, (_, pc) => Math.max(0.05, prof[(pc - tonic + 12) % 12] / 6.4 - 0.25 + r() * 0.12));
  const cmax = Math.max(...chroma);
  const scores = Array.from({ length: 24 }, (_, k) => {
    const t = k % 12;
    const m = k >= 12;
    if (t === tonic && m === minor) return 0.86;
    if (minor && !m && t === (tonic + 3) % 12) return 0.79;
    if (!minor && m && t === (tonic + 9) % 12) return 0.78;
    return 0.6 - r() * 0.9;
  });
  const waveform = Array.from({ length: 720 }, (_, i) => {
    const x = i / 720;
    const env = (x < 0.08 ? x / 0.08 : 1) * (x > 0.45 && x < 0.55 ? 0.35 : 1) * (x > 0.94 ? (1 - x) / 0.06 : 1);
    const a = env * (0.55 + r() * 0.45);
    return [-a, a * (0.9 + r() * 0.1)];
  });
  const drift = Array.from({ length: Math.floor(duration / 20) - 1 }, (_, i) =>
    i > 5 && i < 8 ? null : 1 + (r() - 0.5) * 0.0004);
  return {
    path: '', sample_rate: r() > 0.5 ? 44100 : 48000, channels: 2, duration,
    tempo: {
      candidates: [
        { bpm, salience: 1, offset },
        { bpm: bpm / 2, salience: 0.55 + r() * 0.2, offset },
        { bpm: bpm * 2, salience: 0.3 + r() * 0.1, offset },
        { bpm: bpm * 1.5, salience: 0.08, offset },
        { bpm: bpm / 4, salience: 0.06, offset },
      ],
      envelope: [], excerpt: excerpt.map((v) => v / peak), excerpt_start: exStart, frame_rate: fr,
      drift, drift_step: 20,
    },
    key: {
      tonic, minor, confidence: r() > 0.3 ? 1 : 0.75, scores,
      chroma: chroma.map((v) => v / cmax),
      tuning_cents: (r() - 0.5) * 6, reference_a: 440 * Math.pow(2, ((r() - 0.5) * 6) / 1200),
    },
    waveform, elapsed_ms: 800,
  };
}

export function demo() {
  const folder = 'D:\\Music\\Synthwave\\2026';
  const ext = ['FLAC', 'WAV', 'MP3', 'M4A'];
  const tracks = NAMES.map(([name, bpm, tonic, minor], i) => {
    const e = ext[i % ext.length];
    const t = { path: `${folder}\\${name}.${e.toLowerCase()}`, status: 'done', progress: 1 };
    if (i === 9) return { ...t, status: 'running', progress: 0.46 };
    if (i === 10) return { ...t, status: 'queued', progress: 0 };
    if (i === 11) return { ...t, status: 'error', error: 'unsupported codec: ALAC in CAF' };
    return { ...t, report: report(name, bpm, tonic, minor, 1000 + i) };
  });
  inject(tracks);
}
