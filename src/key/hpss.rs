// Отделение гармонической части от ударной (Fitzgerald 2010): медиана вдоль времени оставляет протяжные
// ноты, медиана вдоль частоты — короткие широкополосные удары. Мягкая маска H²/(H²+P²) накладывается
// на спектр, сигнал собирается обратно наложением кадров. Бочка и тарелки больше не пачкают профиль нот.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

use rayon::prelude::*;
use rustfft::num_complex::Complex32;

use crate::analysis::{Failure, Reporter};
use crate::dsp::{median::median_filter, smooth_size, spectrum::Plan, window::hann};

const FRAME_SECONDS: f64 = 0.18;
/// Длина медианы вдоль времени (кадров) и вдоль частоты (бинов).
const TIME_MEDIAN: usize = 17;
const FREQ_MEDIAN: usize = 31;
/// Выше этой частоты для тональности ничего не нужно.
const F_MAX: f64 = 5500.0;
const CHUNK: usize = 384;

pub fn harmonic(x: &[f32], sr: f64, rep: &dyn Reporter, base: f64, span: f64) -> Result<Vec<f32>, Failure> {
    let n = smooth_size((sr * FRAME_SECONDS).round() as usize, 2);
    let hop = n / 4;
    let plan = Plan::new(n, hann(n));
    let kmax = ((F_MAX.min(sr * 0.475) * n as f64 / sr).ceil() as usize).min(n / 2 - 1);
    let frames = x.len() / hop + 1;
    let margin = TIME_MEDIAN / 2;
    // сумма квадратов окна по наложению — делитель сборки
    let ola: f32 = (0..hop)
        .map(|r| (0..n / hop).map(|j| plan.window[r + j * hop].powi(2)).sum::<f32>())
        .sum::<f32>()
        / hop as f32;

    let chunks = frames.div_ceil(CHUNK);
    let done = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let parts: Vec<(usize, Vec<f32>)> = (0..chunks)
        .into_par_iter()
        .map_init(
            || plan.workspace(),
            |ws, c| {
                let f0 = c * CHUNK;
                let f1 = ((c + 1) * CHUNK).min(frames);
                if stop.load(Relaxed) {
                    return (f0 * hop, Vec::new());
                }
                let a = f0.saturating_sub(margin);
                let b = (f1 + margin).min(frames);
                let rows = b - a;
                let mut cpx = vec![Complex32::new(0.0, 0.0); rows * kmax];
                let mut mag = vec![0f32; rows * kmax];
                for (r, t) in (a..b).enumerate() {
                    ws.forward(x, t * hop);
                    for k in 0..kmax {
                        let v = ws.buf[k];
                        cpx[r * kmax + k] = v;
                        mag[r * kmax + k] = v.norm();
                    }
                }
                // гармоническая оценка: медиана по времени для каждого бина
                let mut harm = vec![0f32; rows * kmax];
                let mut col = vec![0f32; rows];
                let mut col_out = vec![0f32; rows];
                for k in 0..kmax {
                    for r in 0..rows {
                        col[r] = mag[r * kmax + k];
                    }
                    median_filter(&col, TIME_MEDIAN, &mut col_out);
                    for r in 0..rows {
                        harm[r * kmax + k] = col_out[r];
                    }
                }
                let mut perc = vec![0f32; kmax];
                let mut out = vec![0f32; (f1 - f0 - 1) * hop + n];
                for t in f0..f1 {
                    let r = t - a;
                    // ударная оценка: медиана по частоте в кадре
                    median_filter(&mag[r * kmax..(r + 1) * kmax], FREQ_MEDIAN, &mut perc);
                    ws.buf.iter_mut().for_each(|v| *v = Complex32::new(0.0, 0.0));
                    for k in 0..kmax {
                        let h = harm[r * kmax + k];
                        let p = perc[k];
                        let h2 = h * h;
                        let m = h2 / (h2 + p * p + 1e-20);
                        let v = cpx[r * kmax + k] * m;
                        ws.buf[k] = v;
                        if k > 0 {
                            ws.buf[n - k] = v.conj();
                        }
                    }
                    ws.inverse();
                    let at = (t - f0) * hop;
                    for i in 0..n {
                        out[at + i] += ws.buf[i].re * plan.window[i];
                    }
                }
                let k = done.fetch_add(1, Relaxed) + 1;
                rep.progress(base + span * k as f64 / chunks as f64);
                if rep.cancelled() {
                    stop.store(true, Relaxed);
                }
                (f0 * hop, out)
            },
        )
        .collect();
    if stop.load(Relaxed) {
        return Err(Failure::Cancelled);
    }

    // out[0] части соответствует отсчёту f0·hop − n/2
    let mut y = vec![0f32; x.len()];
    let gain = 1.0 / (n as f32 * ola);
    for (s0, part) in parts {
        let start = s0 as isize - (n / 2) as isize;
        for (i, v) in part.iter().enumerate() {
            let idx = start + i as isize;
            if idx >= 0 && (idx as usize) < y.len() {
                y[idx as usize] += v * gain;
            }
        }
    }
    Ok(y)
}
