// Кадровое БПФ: кадр t покрывает отсчёты [t·hop − n/2, t·hop + n/2), за краями сигнала — нули.
use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

/// Общая часть для всех потоков: план БПФ и окно.
#[derive(Clone)]
pub struct Plan {
    pub n: usize,
    pub forward: Arc<dyn Fft<f32>>,
    pub inverse: Arc<dyn Fft<f32>>,
    pub window: Arc<Vec<f32>>,
}

impl Plan {
    pub fn new(n: usize, window: Vec<f32>) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        Plan {
            n,
            forward: planner.plan_fft_forward(n),
            inverse: planner.plan_fft_inverse(n),
            window: Arc::new(window),
        }
    }

    pub fn workspace(&self) -> Workspace {
        let scratch_len = self
            .forward
            .get_inplace_scratch_len()
            .max(self.inverse.get_inplace_scratch_len());
        Workspace {
            plan: self.clone(),
            buf: vec![Complex32::new(0.0, 0.0); self.n],
            scratch: vec![Complex32::new(0.0, 0.0); scratch_len],
        }
    }
}

/// Буферы одного потока.
pub struct Workspace {
    pub plan: Plan,
    pub buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
}

impl Workspace {
    /// Спектр кадра с центром в отсчёте `center`. Результат — в `self.buf` (все n бинов).
    pub fn forward(&mut self, x: &[f32], center: usize) {
        let n = self.plan.n;
        let start = center as isize - (n / 2) as isize;
        let w = &self.plan.window;
        for (i, b) in self.buf.iter_mut().enumerate() {
            let idx = start + i as isize;
            let v = if idx >= 0 && (idx as usize) < x.len() { x[idx as usize] } else { 0.0 };
            *b = Complex32::new(v * w[i], 0.0);
        }
        self.plan.forward.process_with_scratch(&mut self.buf, &mut self.scratch);
    }

    /// Обратное БПФ содержимого `self.buf` на месте (без деления на n).
    pub fn inverse(&mut self) {
        self.plan.inverse.process_with_scratch(&mut self.buf, &mut self.scratch);
    }

    /// Модули бинов 0..count в `out`.
    pub fn magnitudes(&self, out: &mut [f32]) {
        for (o, c) in out.iter_mut().zip(&self.buf) {
            *o = c.norm();
        }
    }
}
