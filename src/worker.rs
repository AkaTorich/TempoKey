// Очередь разбора: один поток берёт файлы по одному; внутри файла работа делится на половину ядер,
// чтобы компьютер оставался отзывчивым. Ход и итог уходят в окно событиями.
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::analysis::{self, Reporter};
use crate::report::TrackReport;

pub struct Job {
    pub id: u64,
    pub path: String,
}

#[derive(Default)]
pub struct Queue {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    /// (поколение очереди, задание): после отмены старые поколения пропускаются.
    items: Mutex<VecDeque<(u64, Job)>>,
    ready: Condvar,
    generation: AtomicU64,
}

impl Queue {
    pub fn push(&self, jobs: Vec<Job>) {
        let g = self.shared.generation.load(Ordering::SeqCst);
        let mut q = self.shared.items.lock().unwrap();
        q.extend(jobs.into_iter().map(|j| (g, j)));
        self.shared.ready.notify_all();
    }

    pub fn cancel_all(&self) {
        let mut q = self.shared.items.lock().unwrap();
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
        q.clear();
    }
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    id: u64,
    progress: f64,
}

#[derive(Serialize, Clone)]
struct DoneEvent {
    id: u64,
    report: Option<TrackReport>,
    error: Option<String>,
}

struct EventReporter<'a> {
    app: &'a AppHandle,
    shared: &'a Shared,
    id: u64,
    generation: u64,
    /// Последний отправленный процент — чаще раза в процент окно не дёргается.
    last: AtomicI64,
}

impl Reporter for EventReporter<'_> {
    fn progress(&self, fraction: f64) {
        let p = (fraction.clamp(0.0, 1.0) * 100.0).floor() as i64;
        let prev = self.last.fetch_max(p, Ordering::Relaxed);
        if p > prev {
            let _ = self.app.emit("track-progress", ProgressEvent { id: self.id, progress: p as f64 / 100.0 });
        }
    }

    fn cancelled(&self) -> bool {
        self.shared.generation.load(Ordering::Relaxed) != self.generation
    }
}

pub fn start(app: AppHandle) {
    let shared = app.state::<Queue>().shared.clone();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).div_ceil(2);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .thread_name(|i| format!("tempokey-{i}"))
        .build()
        .expect("пул потоков разбора");

    std::thread::Builder::new()
        .name("tempokey-queue".into())
        .spawn(move || loop {
            let (generation, job) = {
                let mut q = shared.items.lock().unwrap();
                loop {
                    if let Some(item) = q.pop_front() {
                        break item;
                    }
                    q = shared.ready.wait(q).unwrap();
                }
            };
            if shared.generation.load(Ordering::SeqCst) != generation {
                continue;
            }
            let rep = EventReporter { app: &app, shared: &shared, id: job.id, generation, last: AtomicI64::new(-1) };
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                pool.install(|| analysis::analyze_file(Path::new(&job.path), &rep))
            }));
            let event = match outcome {
                Ok(Ok(report)) => DoneEvent { id: job.id, report: Some(report), error: None },
                Ok(Err(e)) => DoneEvent { id: job.id, report: None, error: Some(e.text()) },
                Err(_) => DoneEvent { id: job.id, report: None, error: Some("internal analysis error".into()) },
            };
            let _ = app.emit("track-done", event);
        })
        .expect("поток очереди");
}
