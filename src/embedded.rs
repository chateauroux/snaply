// ============================================================================
//  embedded.rs — ВСЕ ассеты встроены в бинарник
// ----------------------------------------------------------------------------
//  Все файлы из assets/ встраиваются в .exe через include_bytes!/include_str!.
//  Это позволяет раздавать Snaply одним .exe без папки assets/.
//
//  build.rs гарантирует что все файлы существуют перед сборкой — если
//  background.png или notification.wav отсутствуют, создаётся пустая
//  заглушка (0 байт). На runtime отличаем пустой файл от настоящего по длине.
// ============================================================================

use std::path::PathBuf;

// ----------------------------------------------------------------------------
//  Embed файлов на этапе компиляции.
//  Пути относительные — от корня crate (где лежит Cargo.toml).
//  build.rs гарантирует что файлы существуют (создаёт заглушки если нет).
// ----------------------------------------------------------------------------

/// Шрифт Inter (OFL 1.1). ~880 КБ.
pub const INTER_FONT: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");

/// Иконка приложения snaply.ico.
pub const ICON_ICO: &[u8] = include_bytes!("../assets/snaply.ico");

/// index.html ChopSS.
pub const CHOPSS_HTML: &str = include_str!("../assets/chopss/index.html");

/// html2canvas.min.js (~200 КБ).
pub const CHOPSS_HTML2CANVAS: &[u8] = include_bytes!("../assets/chopss/html2canvas.min.js");

/// Фоновая картинка background.png. Может быть пустой (0 байт) — тогда фолбэк.
pub const BACKGROUND_PNG: &[u8] = include_bytes!("../assets/background.png");

/// Звук уведомления notification.wav. Может быть пустым (0 байт) — тогда фолбэк.
pub const NOTIFICATION_WAV: &[u8] = include_bytes!("../assets/notification.wav");

// ----------------------------------------------------------------------------
//  Опциональные ассеты — возвращают Some только если embed непустой.
//  build.rs создаёт пустые заглушки (0 байт) если файла нет, и мы здесь
//  отличаем «нет файла» (0 байт) от «есть файл».
// ----------------------------------------------------------------------------

/// Вернуть background.png если он встроен и непустой.
/// Возвращает None если файл-заглушка (0 байт) — тогда используем градиент.
pub fn get_background_png() -> Option<&'static [u8]> {
    if BACKGROUND_PNG.is_empty() {
        None
    } else {
        Some(BACKGROUND_PNG)
    }
}

/// Вернуть notification.wav если он встроен и непустой.
/// Возвращает None если файл-заглушка (0 байт) — тогда системный MessageBeep.
pub fn get_notification_wav() -> Option<&'static [u8]> {
    if NOTIFICATION_WAV.is_empty() {
        None
    } else {
        Some(NOTIFICATION_WAV)
    }
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
    if !html_path.exists() {
        std::fs::write(&html_path, CHOPSS_HTML).map_err(|e| e.to_string())?;
    }
    if !js_path.exists() {
        std::fs::write(&js_path, CHOPSS_HTML2CANVAS).map_err(|e| e.to_string())?;
    }

    Ok(html_path)
}
