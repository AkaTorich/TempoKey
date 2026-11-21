// Заставка: главное окно строит себя скрытым (см. shape.rs) и сообщает шаги (boot_step);
// заставка (ui/splash.html) показывает их полосой хода и по готовности зовёт app_ready — главное окно
// получает свой контур и фокус, заставка закрывается.
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::shape;

/// Если заставка не отработала, главное окно показывается само через это время.
const FALLBACK: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct Boot(Mutex<BootStep>);

#[derive(Serialize, Clone, Default)]
pub struct BootStep {
    pub progress: f64,
    pub label: String,
}

/// Главное окно: очередной шаг сборки.
#[tauri::command]
pub fn boot_step(app: AppHandle, boot: State<Boot>, progress: f64, label: String) {
    let step = BootStep { progress, label };
    *boot.0.lock().unwrap() = step.clone();
    let _ = app.emit("boot", step);
}

/// Заставка: где сейчас сборка (если заставка открылась позже первых шагов).
#[tauri::command]
pub fn boot_status(boot: State<Boot>) -> BootStep {
    boot.0.lock().unwrap().clone()
}

/// Заставка: всё отрисовано — показать главное окно, закрыть заставку.
#[tauri::command]
pub fn app_ready(app: AppHandle) {
    reveal(&app);
}

/// При запуске (оба окна созданы скрытыми; заставка покажется сама, получив контур, — shape.rs):
/// запасной показ главного окна, если заставка не отработает.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FALLBACK);
        let hidden = app.get_webview_window("main").map(|w| !shape::is_revealed(&w)).unwrap_or(false);
        if hidden {
            reveal(&app);
        }
    });
}

fn reveal(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        shape::reveal_main(&main);
        let _ = main.show();
        let _ = main.set_focus();
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
}
