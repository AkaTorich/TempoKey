// Нестандартная форма окна: окно обрезается по многоугольнику, который присылает окно (ui/js/shape.js).
// Вне контура окна нет вовсе — ни рисунка, ни щелчков: они уходят к тому, что под ним. Внутри контура всё
// закрашено непрозрачно: прозрачность WebView2 в Windows слетает (скрытый запуск, модальные диалоги) и
// заливается белым, поэтому форма держится только на вырезе.
//
// Оба окна создаются скрытыми и показываются только со своим контуром — прямоугольника без формы на экране
// не бывает: заставка — сразу, как пришлёт контур; главное окно — по готовности (boot::reveal → reveal_main),
// присланный раньше контур запоминается. (Пустой вырез у скрытого окна не годится: Windows сбрасывает его,
// пока создаётся WebView2.)
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{Manager, State, WebviewWindow};

#[derive(Default)]
pub struct Shape {
    revealed: AtomicBool,
    pending: Mutex<Option<Vec<[f64; 2]>>>,
}

/// `points` — вершины в точках CSS относительно левого верхнего угла окна; меньше трёх — снять форму.
#[tauri::command]
pub fn set_window_shape(window: WebviewWindow, shape: State<Shape>, points: Vec<[f64; 2]>) -> Result<(), String> {
    if window.label() == "main" && !shape.revealed.load(Ordering::SeqCst) {
        *shape.pending.lock().unwrap() = Some(points);
        return Ok(());
    }
    apply(&window, &points)?;
    // заставка появляется, уже имея форму
    if window.label() != "main" && !window.is_visible().unwrap_or(true) {
        let _ = window.show();
    }
    Ok(())
}

/// Показ главного окна: поставить запомненный контур (до показа).
pub fn reveal_main(window: &WebviewWindow) {
    let shape = window.app_handle().state::<Shape>();
    shape.revealed.store(true, Ordering::SeqCst);
    let points = shape.pending.lock().unwrap().take().unwrap_or_default();
    let _ = apply(window, &points);
}

pub fn is_revealed(window: &WebviewWindow) -> bool {
    window.app_handle().state::<Shape>().revealed.load(Ordering::SeqCst)
}

fn apply(window: &WebviewWindow, points: &[[f64; 2]]) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{HWND, POINT};
        use windows_sys::Win32::Graphics::Gdi::{CreatePolygonRgn, SetWindowRgn, ALTERNATE};

        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        // вырез считается от угла окна, а страница — от угла рабочей области; у развёрнутого окна
        // Windows выносит рамку на 8 точек за экран, и они расходятся
        let (dx, dy) = match (window.inner_position(), window.outer_position()) {
            (Ok(i), Ok(o)) => ((i.x - o.x) as f64, (i.y - o.y) as f64),
            _ => (0.0, 0.0),
        };
        unsafe {
            if points.len() < 3 {
                SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                return Ok(());
            }
            let pts: Vec<POINT> = points
                .iter()
                .map(|p| POINT { x: (p[0] * scale + dx).round() as i32, y: (p[1] * scale + dy).round() as i32 })
                .collect();
            let rgn = CreatePolygonRgn(pts.as_ptr(), pts.len() as i32, ALTERNATE);
            if rgn.is_null() {
                return Err("window shape not created".into());
            }
            // после успешного вызова областью владеет система
            SetWindowRgn(hwnd, rgn, 1);
        }
    }
    #[cfg(not(windows))]
    let _ = (window, points);
    Ok(())
}
