// Выраженность темпа по способу Peeters (2007): произведение автокорреляции огибающей (пики на периоде
// и его кратных — темпах T, T/2, T/3) и её спектра (пики на частоте и кратных — темпах T, 2T, 3T).
// Совпадают они только на настоящем темпе, поэтому ошибки «вдвое быстрее / медленнее» гаснут.
use rustfft::num_complex::Complex64;
use rustfft::FftPlanner;

pub struct Peak {
    pub bpm: f64,
    /// У лучшего 1.
    pub salience: f64,
}

/// Шаг сетки темпа, удары в минуту.
const GRID_STEP: f64 = 0.05;
/// Ширина поиска пика спектра около частоты темпа (доля): терпит лёгкий дрейф темпа.
const SPECTRUM_TOLERANCE: f64 = 0.003;
/// Кандидаты ближе этой доли октавы — один пик.
const DISTINCT_OCTAVES: f64 = 0.02;

pub fn candidates(env: &[f32], fr: f64, lo: f64, hi: f64, max: usize) -> Vec<Peak> {
    let n = env.len();
    if n < 16 {
        return Vec::new();
    }
    let mean = env.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let m = (2 * n).next_power_of_two();
    let mut buf: Vec<Complex64> = (0..m)
        .map(|i| Complex64::new(if i < n { env[i] as f64 - mean } else { 0.0 }, 0.0))
        .collect();
    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_forward(m).process(&mut buf);
    let mag: Vec<f64> = buf.iter().map(|c| c.norm()).collect();

    // обобщённая автокорреляция: обратное преобразование |X| (мощность, сжатая корнем) — пики острее
    let mut acf_buf: Vec<Complex64> = mag.iter().map(|&v| Complex64::new(v, 0.0)).collect();
    planner.plan_fft_inverse(m).process(&mut acf_buf);
    let r0 = acf_buf[0].re.max(1e-12);
    let acf: Vec<f64> = acf_buf[..m / 2].iter().map(|c| c.re / r0).collect();
    let half = &mag[..m / 2];
    let spec_peak = half[1..].iter().cloned().fold(1e-12, f64::max);

    let count = ((hi - lo) / GRID_STEP).floor() as usize + 1;
    let sal: Vec<f64> = (0..count)
        .map(|i| {
            let bpm = lo + i as f64 * GRID_STEP;
            let a = interp(&acf, 60.0 * fr / bpm).max(0.0);
            let bin = bpm / 60.0 * m as f64 / fr;
            let b0 = (bin * (1.0 - SPECTRUM_TOLERANCE)).floor() as usize;
            let b1 = ((bin * (1.0 + SPECTRUM_TOLERANCE)).ceil() as usize).min(half.len() - 1);
            let s = half[b0.min(b1)..=b1].iter().cloned().fold(0.0, f64::max) / spec_peak;
            a * s
        })
        .collect();

    let mut peaks: Vec<(f64, f64)> = Vec::new();
    for i in 1..count.saturating_sub(1) {
        let (a, b, c) = (sal[i - 1], sal[i], sal[i + 1]);
        if b > a && b >= c && b > 0.0 {
            let den = a - 2.0 * b + c;
            let d = if den.abs() > 1e-18 { 0.5 * (a - c) / den } else { 0.0 };
            peaks.push((lo + (i as f64 + d) * GRID_STEP, b));
        }
    }
    peaks.sort_by(|x, y| y.1.total_cmp(&x.1));

    let mut out: Vec<Peak> = Vec::new();
    for (bpm, s) in peaks {
        if out.iter().all(|p| (bpm / p.bpm).log2().abs() > DISTINCT_OCTAVES) {
            out.push(Peak { bpm, salience: s });
            if out.len() == max {
                break;
            }
        }
    }
    let top = out.first().map(|p| p.salience).unwrap_or(1.0).max(1e-18);
    out.iter_mut().for_each(|p| p.salience /= top);
    out
}

fn interp(v: &[f64], x: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    let i = x.floor() as usize;
    if i + 1 >= v.len() {
        return 0.0;
    }
    let f = x - i as f64;
    v[i] * (1.0 - f) + v[i + 1] * f
}
