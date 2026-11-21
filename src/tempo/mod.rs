// Темп: огибающая атак (onset) → кандидаты по выраженности (salience) → уточнение каждого (refine).
pub mod onset;
pub mod refine;
pub mod salience;

use rayon::prelude::*;

use crate::analysis::{Failure, Reporter};
use crate::report::{TempoCandidate, TempoReport};

/// Где искать кандидатов, удары в минуту. Выбор внутри диапазона из окна делает окно.
const SEARCH_LO: f64 = 30.0;
const SEARCH_HI: f64 = 300.0;
const MAX_CANDIDATES: usize = 8;
/// Кандидаты ближе этой доли после уточнения считаются одним.
const MERGE_RATIO: f64 = 0.004;
/// Точек огибающей для рисунка.
const ENVELOPE_POINTS: usize = 1440;
/// Длина отрезка для сетки долей, секунды.
const EXCERPT_S: f64 = 8.0;

pub fn analyze(x: &[f32], sr: f64, rep: &dyn Reporter, base: f64, span: f64) -> Result<TempoReport, Failure> {
    let env = onset::superflux(x, sr, rep, base, span * 0.55)?;
    let fr = env.frame_rate;
    let peaks = salience::candidates(&env.values, fr, SEARCH_LO, SEARCH_HI, MAX_CANDIDATES);
    rep.progress(base + span * 0.65);
    if rep.cancelled() {
        return Err(Failure::Cancelled);
    }

    let e: Vec<f64> = env.values.iter().map(|&v| v as f64).collect();
    let refined: Vec<TempoCandidate> = peaks
        .par_iter()
        .map(|p| {
            let bpm = refine::fine_tempo(&e, fr, p.bpm);
            TempoCandidate { bpm, salience: p.salience, offset: refine::beat_offset(&e, fr, bpm) }
        })
        .collect();
    let mut candidates: Vec<TempoCandidate> = Vec::new();
    for c in refined {
        if candidates.iter().all(|k| (c.bpm / k.bpm - 1.0).abs() > MERGE_RATIO) {
            candidates.push(c);
        }
    }
    rep.progress(base + span * 0.85);
    if rep.cancelled() {
        return Err(Failure::Cancelled);
    }

    let (drift, drift_step) = match candidates.first() {
        Some(best) => refine::local_tempo(&e, fr, best.bpm),
        None => (Vec::new(), refine::DRIFT_STEP_S),
    };
    rep.progress(base + span);

    let (excerpt, excerpt_start) = densest(&env.values, (EXCERPT_S * fr) as usize, fr);
    Ok(TempoReport {
        candidates,
        envelope: reduce(&env.values, ENVELOPE_POINTS),
        excerpt,
        excerpt_start,
        frame_rate: fr,
        drift,
        drift_step,
    })
}

/// Отрезок длиной `len` кадров с наибольшей суммой огибающей (там удары видны лучше всего), нормирован к 1.
fn densest(v: &[f32], len: usize, fr: f64) -> (Vec<f32>, f64) {
    let len = len.min(v.len());
    if len == 0 {
        return (Vec::new(), 0.0);
    }
    let mut sum: f64 = v[..len].iter().map(|&x| x as f64).sum();
    let (mut best, mut best_at) = (sum, 0usize);
    for i in len..v.len() {
        sum += v[i] as f64 - v[i - len] as f64;
        if sum > best {
            best = sum;
            best_at = i + 1 - len;
        }
    }
    let part = &v[best_at..best_at + len];
    let peak = part.iter().cloned().fold(1e-9, f32::max);
    (part.iter().map(|x| x / peak).collect(), best_at as f64 / fr)
}

/// Сжатие огибающей до `points` точек по максимуму, нормировка к 1.
fn reduce(v: &[f32], points: usize) -> Vec<f32> {
    let n = v.len();
    if n == 0 {
        return Vec::new();
    }
    let mut out: Vec<f32> = (0..points)
        .map(|i| {
            let a = i * n / points;
            let b = ((i + 1) * n / points).max(a + 1).min(n);
            v[a..b].iter().cloned().fold(0.0, f32::max)
        })
        .collect();
    let peak = out.iter().cloned().fold(1e-9, f32::max);
    out.iter_mut().for_each(|x| *x /= peak);
    out
}
