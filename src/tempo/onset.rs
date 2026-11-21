// Огибающая атак методом SuperFlux (Böck, Widmer 2013): логарифмический банк полос по 12 на октаву,
// положительный прирост энергии к кадру μ шагов назад, где у прошлого кадра берётся максимум из трёх
// соседних полос — так вибрато и глиссандо не дают ложных атак.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

use rayon::prelude::*;

use crate::analysis::{Failure, Reporter};
use crate::dsp::{smooth_size, spectrum::Plan, window::hann};

pub struct Envelope {
    pub values: Vec<f32>,
    /// Кадров в секунду (точно: частота дискретизации / шаг).
    pub frame_rate: f64,
}

const FRAME_SECONDS: f64 = 0.046;
const FRAMES_PER_SECOND: f64 = 200.0;
const BANDS_PER_OCTAVE: f64 = 12.0;
const F_LOW: f64 = 30.0;
const F_HIGH: f64 = 16000.0;
/// Усиление перед логарифмом: log(1 + G·E).
const LOG_GAIN: f32 = 1000.0;
/// Окно вычитания местного среднего, секунды.
const MEAN_WINDOW_S: f64 = 1.0;
const CHUNK: usize = 2048;

pub fn superflux(x: &[f32], sr: f64, rep: &dyn Reporter, base: f64, span: f64) -> Result<Envelope, Failure> {
    let n = smooth_size((sr * FRAME_SECONDS).round() as usize, 2);
    let hop = ((sr / FRAMES_PER_SECOND).round() as usize).max(1);
    let fr = sr / hop as f64;
    let frames = x.len() / hop + 1;
    let plan = Plan::new(n, hann(n));
    let scale = 2.0 / plan.window.iter().sum::<f32>();
    let bank = Filterbank::new(n, sr);
    let nb = bank.len();
    // сдвиг сравнения: на сколько кадров окно уходит за половину своей высоты
    let mu = (((n as f64 / 4.0) / hop as f64).round() as usize).max(1);

    let chunks = frames.div_ceil(CHUNK);
    let done = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let parts: Vec<Vec<f32>> = (0..chunks)
        .into_par_iter()
        .map_init(
            || (plan.workspace(), vec![0f32; n / 2 + 1]),
            |(ws, mags), c| {
                if stop.load(Relaxed) {
                    return Vec::new();
                }
                let f0 = c * CHUNK;
                let f1 = ((c + 1) * CHUNK).min(frames);
                let start = f0.saturating_sub(mu);
                let mut bands = vec![0f32; (f1 - start) * nb];
                for (row, t) in (start..f1).enumerate() {
                    ws.forward(x, t * hop);
                    ws.magnitudes(mags);
                    bank.apply(mags, scale, &mut bands[row * nb..(row + 1) * nb]);
                }
                let mut flux = vec![0f32; f1 - f0];
                for t in f0.max(mu)..f1 {
                    let cur = &bands[(t - start) * nb..(t - start + 1) * nb];
                    let prev = &bands[(t - mu - start) * nb..(t - mu - start + 1) * nb];
                    let mut s = 0f32;
                    for b in 0..nb {
                        let lo = b.saturating_sub(1);
                        let hi = (b + 1).min(nb - 1);
                        let mx = prev[lo..=hi].iter().cloned().fold(f32::MIN, f32::max);
                        let d = cur[b] - mx;
                        if d > 0.0 {
                            s += d;
                        }
                    }
                    flux[t - f0] = s;
                }
                let k = done.fetch_add(1, Relaxed) + 1;
                rep.progress(base + span * 0.95 * k as f64 / chunks as f64);
                if rep.cancelled() {
                    stop.store(true, Relaxed);
                }
                flux
            },
        )
        .collect();
    if stop.load(Relaxed) {
        return Err(Failure::Cancelled);
    }

    let mut env = parts.concat();
    env.truncate(frames);
    postprocess(&mut env, fr);
    Ok(Envelope { values: env, frame_rate: fr })
}

/// Вычитание местного среднего (уходят медленные перепады громкости), отсечение отрицательного,
/// приведение к единичному разбросу.
fn postprocess(env: &mut [f32], fr: f64) {
    let n = env.len();
    if n == 0 {
        return;
    }
    let half = ((fr * MEAN_WINDOW_S * 0.5).round() as usize).max(1);
    let mut prefix = vec![0f64; n + 1];
    for i in 0..n {
        prefix[i + 1] = prefix[i] + env[i] as f64;
    }
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let a = i.saturating_sub(half);
            let b = (i + half + 1).min(n);
            let mean = (prefix[b] - prefix[a]) / (b - a) as f64;
            (env[i] as f64 - mean).max(0.0) as f32
        })
        .collect();
    let mean = out.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let var = out.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    let k = if var > 1e-18 { (1.0 / var.sqrt()) as f32 } else { 1.0 };
    for (e, o) in env.iter_mut().zip(out) {
        *e = o * k;
    }
}

/// Треугольные полосы на логарифмической шкале; у низких частот, где бинов меньше, чем полос,
/// совпадающие центры сливаются (как в madmom).
struct Filterbank {
    filters: Vec<(usize, Vec<f32>)>,
}

impl Filterbank {
    fn new(n: usize, sr: f64) -> Self {
        let bin_hz = sr / n as f64;
        let f_hi = F_HIGH.min(sr * 0.5 * 0.95);
        let ratio = 2f64.powf(1.0 / BANDS_PER_OCTAVE);
        let mut centers: Vec<usize> = Vec::new();
        let mut f = F_LOW;
        while f <= f_hi {
            let b = (f / bin_hz).round() as usize;
            if centers.last() != Some(&b) {
                centers.push(b);
            }
            f *= ratio;
        }
        let mut filters = Vec::new();
        for j in 1..centers.len().saturating_sub(1) {
            let (l, c, r) = (centers[j - 1], centers[j], centers[j + 1]);
            let mut w: Vec<f32> = (l + 1..r)
                .map(|k| if k <= c { (k - l) as f32 / (c - l) as f32 } else { (r - k) as f32 / (r - c) as f32 })
                .collect();
            let s: f32 = w.iter().sum();
            w.iter_mut().for_each(|v| *v /= s);
            filters.push((l + 1, w));
        }
        Filterbank { filters }
    }

    fn len(&self) -> usize {
        self.filters.len()
    }

    fn apply(&self, mags: &[f32], scale: f32, out: &mut [f32]) {
        for ((start, w), o) in self.filters.iter().zip(out.iter_mut()) {
            let e: f32 = mags[*start..*start + w.len()].iter().zip(w).map(|(m, k)| m * k).sum();
            *o = (LOG_GAIN * scale * e).ln_1p();
        }
    }
}
