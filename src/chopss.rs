// ============================================================================
//  chopss.rs — открытие утилиты ChopSS в системном браузере
// ----------------------------------------------------------------------------
//  ChopSS встроен в бинарник (embedded::CHOPSS_HTML + CHOPSS_HTML2CANVAS).
//  При первом запуске извлекаем файлы в %TEMP%/snaply_chopss/, чтобы браузер
//  мог открыть file:// URL. При последующих запусках — используем уже
//  извлечённые файлы (быстро).
//
//  Почему не встроенный WebView (wry+tao)?
//  • tao требует EventLoop в главном потоке — конфликтует с eframe
//  • with_any_thread(true) вызывает STATUS_FATAL_USER_CALLBACK_EXCEPTION на Windows
//
//  Решение: открыть index.html в дефолтном системном браузере через
//  `cmd /C start "" <path>`. 100% функциональности сохраняется.
// ============================================================================

use std::path::PathBuf;

/// Открыть ChopSS в дефолтном системном браузере.
///
/// Извлекает index.html и html2canvas.min.js из бинарника в %TEMP%/snaply_chopss/,
/// затем запускает `cmd /C start "" <path>` — Windows сама откроет файл в браузере.
///
/// Возвращает Ok(()) если команда запустилась, Err с сообщением если
/// не удалось извлечь файлы или запустить процесс.
pub fn open() -> Result<(), String> {
    // 1) Извлекаем ChopSS из бинарника во временную папку
    let html_path = crate::embedded::extract_chopss()?;

    // 2) На Windows: `cmd /C start "" "C:\path\to\index.html"`
    // Пустые кавычки "" важны — без них start воспринимает путь как заголовок окна.
    let path_str = html_path.to_string_lossy().to_string();

    std::process::Command::new("cmd")
        .args(["/C", "start", "", &path_str])
        .spawn()
        .map_err(|e| format!("Не удалось открыть браузер: {}", e))?;

    Ok(())
}
