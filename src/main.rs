// TempoKey: темп (BPM) и тональность звуковых файлов FLAC, WAV, MP3, AAC/M4A, OGG.
// Окно — Tauri 2 (ui/), разбор звука — модули audio, tempo, key; очередь файлов — worker.
#![windows_subsystem = "windows"]

mod analysis;
mod audio;
mod boot;
mod commands;
mod dsp;
mod key;
mod report;
mod shape;
mod tempo;
mod worker;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Разбор одного файла без окна: TempoKey.exe --analyze <файл> --out <файл.json>
    if let Some(i) = args.iter().position(|a| a == "--analyze") {
        std::process::exit(analysis::cli(&args[i + 1..]));
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(worker::Queue::default())
        .manage(boot::Boot::default())
        .manage(shape::Shape::default())
        .setup(|app| {
            worker::start(app.handle().clone());
            boot::start(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            boot::boot_step,
            boot::boot_status,
            boot::app_ready,
            commands::enqueue_tracks,
            commands::cancel_queue,
            commands::scan_paths,
            commands::write_text_file,
            shape::set_window_shape,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска TempoKey");
}
