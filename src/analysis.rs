// Полный разбор файла: чтение → прореживание → темп → тональность → обзор волны.
use std::path::Path;
use std::time::Instant;

use crate::audio::{decimate, decode};
use crate::report::TrackReport;
use crate::{key, tempo};

/// Куда сообщать ход разбора; должен быть доступен из рабочих потоков.
pub trait Reporter: Sync {
    /// Общая доля готовности 0…1.
    fn progress(&self, fraction: f64);
    fn cancelled(&self) -> bool;
}

#[derive(Debug)]
pub enum Failure {
    Cancelled,
    Message(String),
}

impl Failure {
    pub fn msg(s: impl Into<String>) -> Self {
        Failure::Message(s.into())
    }

    pub fn text(&self) -> String {
        match self {
            Failure::Cancelled => "analysis cancelled".into(),
            Failure::Message(m) => m.clone(),
        }
    }
}

/// Наименьшая длительность, на которой темп ещё различим, секунды.
const MIN_SECONDS: f64 = 6.0;
/// Сколько отрезков в обзоре волны.
const WAVE_POINTS: usize = 720;

pub fn analyze_file(path: &Path, rep: &dyn Reporter) -> Result<TrackReport, Failure> {
    let started = Instant::now();
    let decoded = decode::decode_mono(path, rep, 0.0, 0.3)?;
    let duration = decoded.samples.len() as f64 / decoded.sample_rate as f64;
    if duration < MIN_SECONDS {
        return Err(Failure::msg(format!("file too short ({duration:.1} s)")));
    }
    let waveform = overview(&decoded.samples, WAVE_POINTS);

    let d = decimate::factor_for(decoded.sample_rate);
    let mut x = decimate::decimate(&decoded.samples, d);
    drop(decoded.samples);
    let sr = decoded.sample_rate as f64 / d as f64;
    normalize_rms(&mut x, 0.1);
    rep.progress(0.33);
    if rep.cancelled() {
        return Err(Failure::Cancelled);
    }

    let tempo = tempo::analyze(&x, sr, rep, 0.33, 0.27)?;
    let key = key::analyze(&x, sr, rep, 0.60, 0.40)?;
    rep.progress(1.0);

    Ok(TrackReport {
        path: path.to_string_lossy().into_owned(),
        sample_rate: decoded.sample_rate,
        channels: decoded.channels,
        duration,
        tempo,
        key,
        waveform,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// Громкость к заданному среднеквадратичному уровню — чтобы сжатие в разборе вело себя одинаково у тихих и громких треков.
fn normalize_rms(x: &mut [f32], target: f64) {
    let ms: f64 = x.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / x.len().max(1) as f64;
    if ms > 1e-12 {
        let g = (target / ms.sqrt()) as f32;
        x.iter_mut().for_each(|v| *v *= g);
    }
}

fn overview(x: &[f32], points: usize) -> Vec<[f32; 2]> {
    let n = x.len();
    let mut out = Vec::with_capacity(points);
    let mut peak = 1e-9f32;
    for i in 0..points {
        let a = i * n / points;
        let b = ((i + 1) * n / points).max(a + 1).min(n);
        let (mut lo, mut hi) = (0f32, 0f32);
        for &v in &x[a..b] {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        peak = peak.max(hi).max(-lo);
        out.push([lo, hi]);
    }
    out.iter_mut().for_each(|p| {
        p[0] /= peak;
        p[1] /= peak;
    });
    out
}

struct Silent;

impl Reporter for Silent {
    fn progress(&self, _: f64) {}
    fn cancelled(&self) -> bool {
        false
    }
}

/// Разбор из командной строки: `--analyze <файл> --out <файл.json>`. Код выхода 0 — успех.
pub fn cli(args: &[String]) -> i32 {
    let Some(input) = args.first() else { return 2 };
    let out = args.iter().position(|a| a == "--out").and_then(|i| args.get(i + 1));
    let text = match analyze_file(Path::new(input), &Silent) {
        Ok(r) => serde_json::to_string_pretty(&r).unwrap_or_default(),
        Err(e) => format!("{{\"error\": {}}}", serde_json::to_string(&e.text()).unwrap_or_default()),
    };
    match out {
        Some(p) => {
            if std::fs::write(p, &text).is_err() {
                return 3;
            }
        }
        None => println!("{text}"),
    }
    if text.starts_with("{\"error\"") { 1 } else { 0 }
}
