// Звук: чтение файла в моно и понижение частоты дискретизации для разбора.
pub mod decimate;
pub mod decode;

/// Расширения, которые принимает программа.
pub const EXTENSIONS: &[&str] = &["flac", "wav", "mp3", "aac", "m4a", "mp4", "ogg", "oga"];

pub fn is_audio(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
