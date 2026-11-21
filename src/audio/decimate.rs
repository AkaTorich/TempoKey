// Понижение частоты дискретизации в целое число раз: фильтр нижних частот (окно Блэкмана) и прореживание.
// Разбор идёт на 16…32 кГц — выше нет ни атак, ни нот, нужных для темпа и тональности.
use rayon::prelude::*;
use std::f64::consts::PI;

/// Во сколько раз прореживать, чтобы частота осталась не ниже 22 050 Гц.
pub fn factor_for(sample_rate: u32) -> usize {
    ((sample_rate as f64 / 22050.0).floor() as usize).max(1)
}

pub fn decimate(x: &[f32], d: usize) -> Vec<f32> {
    if d <= 1 {
        return x.to_vec();
    }
    let half = 16 * d;
    let len = 2 * half + 1;
    // срез на 92 % новой половины частоты дискретизации
    let fc = 0.5 / d as f64 * 0.92;
    let mut h = vec![0f64; len];
    for (i, hv) in h.iter_mut().enumerate() {
        let n = i as f64 - half as f64;
        let sinc = if n == 0.0 { 2.0 * fc } else { (2.0 * PI * fc * n).sin() / (PI * n) };
        let w = 0.42 - 0.5 * (2.0 * PI * i as f64 / (len - 1) as f64).cos()
            + 0.08 * (4.0 * PI * i as f64 / (len - 1) as f64).cos();
        *hv = sinc * w;
    }
    let sum: f64 = h.iter().sum();
    let h: Vec<f32> = h.iter().map(|v| (v / sum) as f32).collect();

    let n = x.len();
    let out_len = n / d;
    (0..out_len)
        .into_par_iter()
        .map(|m| {
            let c = m * d;
            if c >= half && c + half < n {
                x[c - half..=c + half].iter().zip(&h).map(|(a, b)| a * b).sum()
            } else {
                let mut acc = 0f32;
                for (i, hv) in h.iter().enumerate() {
                    let idx = c as isize + i as isize - half as isize;
                    if idx >= 0 && (idx as usize) < n {
                        acc += hv * x[idx as usize];
                    }
                }
                acc
            }
        })
        .collect()
}
