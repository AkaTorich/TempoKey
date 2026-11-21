// Профиль высот трека:
// длинные кадры 0,7 с (разрешение ~1,4 Гц — различимы полутона баса), окно Блэкмана — Харриса,
// выравнивание спектра делением на местную огибающую ±1/6 октавы, спектральные пики с параболическим
// уточнением частоты → ноты кадра (notes.rs: основные тоны без обертонов) → общий строй трека по
// уточнённым основным тонам → вклад каждой ноты в 12 ступеней окном cos² шириной 4/3 полутона.
use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

use rayon::prelude::*;

use super::notes::{self, Note};
use crate::analysis::{Failure, Reporter};
use crate::dsp::{smooth_size, spectrum::Plan, window::blackman_harris};

const FRAME_SECONDS: f64 = 0.7;
const F_MIN: f64 = 40.0;
const F_MAX: f64 = 5000.0;
const MAX_PEAKS: usize = 60;
/// Пик должен быть выше местной огибающей во столько раз.
const PEAK_OVER_ENVELOPE: f32 = 2.0;
/// И не ниже этой доли самого сильного бина кадра (−60 дБ).
const PEAK_FLOOR: f32 = 1e-3;
/// Полуширина выравнивающей огибающей: октавы и не меньше бинов.
const WHITEN_OCTAVES: f64 = 1.0 / 6.0;
const WHITEN_MIN_BINS: usize = 12;
const WINDOW_SEMITONES: f64 = 4.0 / 3.0;
/// Кадры тише этой доли самого громкого (−40 дБ) не учитываются.
const QUIET_FRAME: f64 = 1e-4;

pub struct Chroma {
    /// Ступени до … си, максимум 1.
    pub pcp: [f64; 12],
    pub tuning_cents: f64,
}

struct Frame {
    energy: f64,
    notes: Vec<Note>,
}

pub fn compute(y: &[f32], sr: f64, rep: &dyn Reporter, base: f64, span: f64) -> Result<Chroma, Failure> {
    let n = smooth_size((sr * FRAME_SECONDS).round() as usize, 2);
    let hop = n / 4;
    let plan = Plan::new(n, blackman_harris(n));
    let bin_hz = sr / n as f64;
    let k_lo = ((F_MIN / bin_hz).floor() as usize).max(2);
    let k_hi = ((F_MAX.min(sr * 0.45) / bin_hz).ceil() as usize).min(n / 2 - 2);
    let frames = y.len() / hop + 1;
    let lo_ratio = 2f64.powf(-WHITEN_OCTAVES);
    let hi_ratio = 2f64.powf(WHITEN_OCTAVES);

    let done = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let all: Vec<Frame> = (0..frames)
        .into_par_iter()
        .map_init(
            || (plan.workspace(), vec![0f32; k_hi + 2], vec![0f64; k_hi + 3]),
            |(ws, mag, prefix), t| {
                if stop.load(Relaxed) {
                    return Frame { energy: 0.0, notes: Vec::new() };
                }
                ws.forward(y, t * hop);
                ws.magnitudes(mag);
                let energy: f64 = mag.iter().map(|&m| (m as f64) * (m as f64)).sum();
                let frame_max = mag[k_lo..=k_hi].iter().cloned().fold(0.0, f32::max);
                prefix[0] = 0.0;
                for k in 0..mag.len() {
                    prefix[k + 1] = prefix[k] + mag[k] as f64;
                }
                let mut peaks: Vec<(f64, f64)> = Vec::new();
                for k in k_lo..=k_hi {
                    let m = mag[k];
                    if !(m > mag[k - 1] && m >= mag[k + 1]) || m < PEAK_FLOOR * frame_max {
                        continue;
                    }
                    let a = ((k as f64 * lo_ratio) as usize).min(k.saturating_sub(WHITEN_MIN_BINS));
                    let b = (((k as f64 * hi_ratio).ceil() as usize).max(k + WHITEN_MIN_BINS)).min(mag.len() - 1);
                    let env = ((prefix[b + 1] - prefix[a]) / (b + 1 - a) as f64) as f32;
                    if env <= 0.0 {
                        continue;
                    }
                    let white = m / env;
                    if white < PEAK_OVER_ENVELOPE {
                        continue;
                    }
                    // параболическое уточнение по логарифму модуля
                    let (la, lb, lc) = (
                        (mag[k - 1].max(1e-12) as f64).ln(),
                        (m.max(1e-12) as f64).ln(),
                        (mag[k + 1].max(1e-12) as f64).ln(),
                    );
                    let den = la - 2.0 * lb + lc;
                    let d = if den.abs() > 1e-12 { (0.5 * (la - lc) / den).clamp(-0.5, 0.5) } else { 0.0 };
                    peaks.push(((k as f64 + d) * bin_hz, white as f64));
                }
                if peaks.len() > MAX_PEAKS {
                    peaks.sort_by(|p, q| q.1.total_cmp(&p.1));
                    peaks.truncate(MAX_PEAKS);
                }
                let k = done.fetch_add(1, Relaxed) + 1;
                if k % 64 == 0 {
                    rep.progress(base + span * 0.95 * k as f64 / frames as f64);
                    if rep.cancelled() {
                        stop.store(true, Relaxed);
                    }
                }
                Frame { energy, notes: notes::extract(&peaks) }
            },
        )
        .collect();
    if stop.load(Relaxed) {
        return Err(Failure::Cancelled);
    }

    let loudest = all.iter().map(|f| f.energy).fold(0.0, f64::max);
    let used: Vec<&Frame> = all.iter().filter(|f| f.energy >= QUIET_FRAME * loudest && !f.notes.is_empty()).collect();
    if used.is_empty() {
        return Err(Failure::msg("no tonal content found"));
    }

    // общий строй: круговое среднее отклонения основных тонов от равномерной сетки полутонов
    let (mut zr, mut zi) = (0.0f64, 0.0f64);
    for f in &used {
        for note in &f.notes {
            let c = 12.0 * (note.freq / 440.0).log2();
            let ang = 2.0 * PI * (c - c.round());
            zr += note.salience * ang.cos();
            zi += note.salience * ang.sin();
        }
    }
    let deviation = if zr.abs() + zi.abs() > 0.0 { zi.atan2(zr) / (2.0 * PI) } else { 0.0 };
    let ref_a = 440.0 * 2f64.powf(deviation / 12.0);

    let mut total = [0f64; 12];
    for f in &used {
        let mut h = [0f64; 12];
        for note in &f.notes {
            // ступень: до = 0, ля = 9
            let pc = (12.0 * (note.freq / ref_a).log2() + 9.0).rem_euclid(12.0);
            for (s, hs) in h.iter_mut().enumerate() {
                let mut d = pc - s as f64;
                if d > 6.0 {
                    d -= 12.0;
                } else if d < -6.0 {
                    d += 12.0;
                }
                if d.abs() < WINDOW_SEMITONES / 2.0 {
                    let c = (PI * d / WINDOW_SEMITONES).cos();
                    *hs += note.salience * c * c;
                }
            }
        }
        let mx = h.iter().cloned().fold(0.0, f64::max);
        if mx > 0.0 {
            for s in 0..12 {
                total[s] += h[s] / mx;
            }
        }
    }
    let mx = total.iter().cloned().fold(1e-12, f64::max);
    total.iter_mut().for_each(|v| *v /= mx);
    rep.progress(base + span);

    Ok(Chroma { pcp: total, tuning_cents: deviation * 100.0 })
}
