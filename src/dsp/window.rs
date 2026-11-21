// Оконные функции (периодические — для кадрового БПФ с перекрытием).
use std::f64::consts::PI;

pub fn hann(n: usize) -> Vec<f32> {
    (0..n).map(|i| (0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos()) as f32).collect()
}

/// Четырёхчленное окно Блэкмана — Харриса: боковые лепестки ниже −92 дБ, ноты не протекают в соседние.
pub fn blackman_harris(n: usize) -> Vec<f32> {
    let (a0, a1, a2, a3) = (0.35875, 0.48829, 0.14128, 0.01168);
    (0..n)
        .map(|i| {
            let x = 2.0 * PI * i as f64 / n as f64;
            (a0 - a1 * x.cos() + a2 * (2.0 * x).cos() - a3 * (3.0 * x).cos()) as f32
        })
        .collect()
}
