// Команды, которые вызывает окно.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::State;

use crate::audio;
use crate::worker::{Job, Queue};

#[derive(Deserialize)]
pub struct TrackIn {
    pub id: u64,
    pub path: String,
}

/// Поставить файлы в очередь разбора; результаты приходят событиями track-progress и track-done.
#[tauri::command]
pub fn enqueue_tracks(queue: State<Queue>, tracks: Vec<TrackIn>) {
    queue.push(tracks.into_iter().map(|t| Job { id: t.id, path: t.path }).collect());
}

/// Снять всё, что ждёт, и прервать текущий разбор.
#[tauri::command]
pub fn cancel_queue(queue: State<Queue>) {
    queue.cancel_all();
}

/// Файлы и папки (с вложенными) → звуковые файлы в порядке имён.
#[tauri::command]
pub async fn scan_paths(paths: Vec<String>) -> Vec<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        for p in paths {
            collect(Path::new(&p), &mut out);
        }
        out.into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
    })
    .await
    .unwrap_or_default()
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(rd) = std::fs::read_dir(path) else { return };
        let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        for e in entries {
            collect(&e, out);
        }
    } else if audio::is_audio(path) {
        out.push(path.to_path_buf());
    }
}

/// Запись текста в UTF-8 с BOM (так CSV открывается в Excel с русскими буквами).
#[tauri::command]
pub fn write_text_file(path: String, text: String) -> Result<(), String> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| format!("cannot write file: {e}"))
}
