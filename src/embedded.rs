// ============================================================================
//  embedded.rs — встроенные в бинарник ассеты
// ----------------------------------------------------------------------------
//  Все файлы из assets/ встраиваются в .exe через include_bytes!/include_str!.
//  Это позволяет раздавать Snaply одним .exe без папки assets/.
//
//  Шрифт Inter, иконка, ChopSS (index.html + html2canvas.min.js)
//  — всё внутри бинарника.
//
//  Для ChopSS дополнительно: при запуске извлекаем index.html и html2canvas.min.js
//  в %TEMP%/snaply_chopss/, чтобы браузер мог открыть file:// URL.
//
//  Для background.png и notification.wav — опциональная загрузка с диска
//  (не embed'им, потому что они могут отсутствовать у разработчика).
// ============================================================================

use std::path::PathBuf;

// ----------------------------------------------------------------------------
//  Embed файлов на этапе компиляции.
//  Пути относительные — от корня crate (где лежит Cargo.toml).
//  Файлы ОБЯЗАТЕЛЬНО должны существовать при cargo build.
// ----------------------------------------------------------------------------

/// Шрифт Inter (OFL 1.1). ~880 КБ.
pub const INTER_FONT: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");

/// Иконка приложения snaply.ico.
pub const ICON_ICO: &[u8] = include_bytes!("../assets/snaply.ico");

/// index.html ChopSS.
pub const CHOPSS_HTML: &str = include_str!("../assets/chopss/index.html");

/// html2canvas.min.js (~200 КБ).
pub const CHOPSS_HTML2CANVAS: &[u8] = include_bytes!("../assets/chopss/html2canvas.min.js");

// ----------------------------------------------------------------------------
//  Опциональные ассеты — загружаются с диска при запуске (не embed'ятся).
//  Если файла нет — используется фолбэк.
// ----------------------------------------------------------------------------

/// Загрузить notification.wav с диска (опционально). Возвращает None если нет.
pub fn load_notification_wav() -> Option<Vec<u8>> {
    find_asset_file("notification.wav")
        .and_then(|p| std::fs::read(&p).ok())
}

/// Загрузить background.png с диска (опционально). Возвращает None если нет.
pub fn load_background_png() -> Option<Vec<u8>> {
    find_asset_file("background.png")
        .and_then(|p| std::fs::read(&p).ok())
}

fn find_asset_file(name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets").join(name));
        candidates.push(cwd.join("target").join("debug").join("assets").join(name));
        candidates.push(cwd.join("target").join("release").join("assets").join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets").join(name));
            candidates.push(dir.join(name));
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

// ----------------------------------------------------------------------------
//  Извлечь ChopSS (index.html + html2canvas.min.js) во временную папку,
//  чтобы браузер мог открыть file:// URL.
//  Возвращает путь к index.html или ошибку.
// ----------------------------------------------------------------------------
pub fn extract_chopss() -> Result<PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("snaply_chopss");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let html_path = temp_dir.join("index.html");
    let js_path = temp_dir.join("html2canvas.min.js");

    // Если файлы уже есть — не перезаписываем (быстрее запуск).
    // Контрольная сумма не проверяется — предполагаем что версия стабальна.
    if !html_path.exists() {
        std::fs::write(&html_path, CHOPSS_HTML).map_err(|e| e.to_string())?;
    }
    if !js_path.exists() {
        std::fs::write(&js_path, CHOPSS_HTML2CANVAS).map_err(|e| e.to_string())?;
    }

    Ok(html_path)
}
