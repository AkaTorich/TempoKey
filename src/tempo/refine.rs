// Уточнение темпа по всей длине трека: максимум суммарной мощности огибающей на частоте темпа и трёх
// её гармониках (алгоритм Гёрцеля, частота — любая, не только по бинам). На треке ровного темпа
// разрешение растёт с длиной: 5 минут дают сотые доли удара в минуту.
use std::f64::consts::PI;

use rayon::prelude::*;

/// Гармоник частоты темпа в точном поиске.
const HARMONICS: usize = 4;
/// Поиск вокруг грубого значения, доля темпа.
const SEARCH_SPAN: f64 = 0.006;
/// Окно и шаг местного темпа, секунды.
const DRIFT_WINDOW_S: f64 = 40.0;
pub const DRIFT_STEP_S: f64 = 20.0;
/// Поиск местного темпа вокруг общего, доля.
const DRIFT_SPAN: f64 = 0.04;
/// Окно без ударов: сумма огибающей ниже этой доли медианной.
const DRIFT_QUIET: f64 = 0.25;

/// Мощность ряда на частоте `f` (циклов на кадр).
fn power(x: &[f64], f: f64) -> f64 {
    let c = 2.0 * (2.0 * PI * f).cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    s1 * s1 + s2 * s2 - c * s1 * s2
}

fn harmonic_power(x: &[f64], f: f64, harmonics: usize) -> f64 {
    (1..=harmonics).map(|h| power(x, f * h as f64)).sum()
}

/// Лучший темп на отрезке [lo, hi] с шагом `step`, с параболическим уточнением.
fn search(x: &[f64], fr: f64, lo: f64, hi: f64, step: f64, harmonics: usize) -> f64 {
    let count = ((hi - lo) / step).round() as usize + 1;
    let vals: Vec<f64> = (0..count)
        .map(|i| harmonic_power(x, (lo + i as f64 * step) / 60.0 / fr, harmonics))
        .collect();
    let i = (0..count).max_by(|&a, &b| vals[a].total_cmp(&vals[b])).unwrap_or(0);
    let mut d = 0.0;
    if i > 0 && i + 1 < count {
        let (a, b, c) = (vals[i - 1], vals[i], vals[i + 1]);
        let den = a - 2.0 * b + c;
        if den.abs() > 1e-300 {
            d = (0.5 * (a - c) / den).clamp(-0.5, 0.5);
        }
    }
    lo + (i as f64 + d) * step
}

pub fn fine_tempo(x: &[f64], fr: f64, bpm0: f64) -> f64 {
    let duration = x.len() as f64 / fr;
    let span = SEARCH_SPAN * bpm0;
    // шаг меньше четверти главного лепестка основной частоты
    let step1 = (0.0002 * bpm0).min(0.25 * 60.0 / duration);
    let b1 = search(x, fr, bpm0 - span, bpm0 + span, step1, 1);
    let step2 = step1 / 10.0;
    search(x, fr, b1 - step1, b1 + step1, step2, HARMONICS)
}

/// Время первой доли: фаза основной частоты темпа. Удары в t0 + j·T дают X ∝ e^(−iωt0).
pub fn beat_offset(x: &[f64], fr: f64, bpm: f64) -> f64 {
    let f = bpm / 60.0 / fr;
    let w = 2.0 * PI * f;
    let (cw, sw) = (w.cos(), w.sin());
    let (mut pr, mut pi) = (1.0f64, 0.0f64);
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (t, &v) in x.iter().enumerate() {
        re += v * pr;
        im -= v * pi;
        let npr = pr * cw - pi * sw;
        let npi = pr * sw + pi * cw;
        pr = npr;
        pi = npi;
        if t % 1024 == 1023 {
            let norm = (pr * pr + pi * pi).sqrt();
            pr /= norm;
            pi /= norm;
        }
    }
    let period = 1.0 / f;
    let t0 = (-im.atan2(re) / w).rem_euclid(period);
    t0 / fr
}

/// Местный темп окнами по 40 с через 20 с — отношение к общему; окна без ударов — None.
pub fn local_tempo(x: &[f64], fr: f64, bpm: f64) -> (Vec<Option<f64>>, f64) {
    let win = (DRIFT_WINDOW_S * fr) as usize;
    let hop = (DRIFT_STEP_S * fr) as usize;
    if x.len() < win || hop == 0 {
        return (Vec::new(), DRIFT_STEP_S);
    }
    let starts: Vec<usize> = (0..).map(|i| i * hop).take_while(|&s| s + win <= x.len()).collect();
    let sums: Vec<f64> = starts.iter().map(|&s| x[s..s + win].iter().sum()).collect();
    let mut sorted = sums.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    let out = starts
        .par_iter()
        .zip(sums.par_iter())
        .map(|(&s, &sum)| {
            if sum < DRIFT_QUIET * median {
                return None;
            }
            let span = DRIFT_SPAN * bpm;
            let local = search(&x[s..s + win], fr, bpm - span, bpm + span, 0.001 * bpm, HARMONICS);
            Some(local / bpm)
        })
        .collect();
    (out, DRIFT_STEP_S)
}
