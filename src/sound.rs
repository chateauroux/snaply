// ============================================================================
//  sound.rs — звук уведомления
// ----------------------------------------------------------------------------
//  Стратегия:
//  1) Ищем файл notification.wav рядом с .exe (или в подпапке assets/)
//  2) Если нашли — проигрываем через Win32 PlaySoundW (SND_FILENAME | SND_ASYNC)
//  3) Если не нашли — фолбэк на системный MessageBeep
//
//  Чтобы поставить свою мелодию (например из FL Studio):
//  • Экспортируй трек из FL как WAV (рекомендуется: 44.1kHz, 16-bit, моно/стерео,
//    длительность 0.3–1.0 сек)
//  • Переименуй в notification.wav
//  • Положи рядом с snaply.exe (или в assets/ рядом с exe)
//  • Программа подхватит автоматически при следующем запуске
// ============================================================================

use std::path::PathBuf;

// Win32 PlaySoundW из winmm.dll. В windows-sys её нет — объявляем FFI напрямую.
// SND_FILENAME = 0x00020000  — pszSound это путь к файлу
// SND_ASYNC    = 0x0001      — не блокировать вызывающий поток
// SND_NODEFAULT= 0x0002      — не играть дефолтный звук если файл не найден
const SND_FILENAME: u32 = 0x00020000;
const SND_ASYNC: u32 = 0x0001;
const SND_NODEFAULT: u32 = 0x0002;

extern "system" {
    fn PlaySoundW(psz_sound: *const u16, hmod: usize, fdw_sound: u32) -> i32;
    fn MessageBeep(u_type: u32) -> i32;
}

// ----------------------------------------------------------------------------
//  Найти notification.wav в любом из мест:
//    1) Текущая рабочая директория (откуда запускают cargo run)
//    2) <cwd>/assets/    3) <cwd>/sounds/
//    4) target/debug/    5) target/debug/assets/    6) target/debug/sounds/
//    7) target/release/  8) target/release/assets/  9) target/release/sounds/
//   10) Рядом с .exe     11) <exe>/assets/          12) <exe>/sounds/
// ----------------------------------------------------------------------------
fn find_sound_file() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1) Текущая рабочая директория (cargo run из корня проекта)
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("notification.wav"));
        candidates.push(cwd.join("assets").join("notification.wav"));
        candidates.push(cwd.join("sounds").join("notification.wav"));

        // target/debug и target/release — относительно корня проекта
        // (cargo run выполняется из корня, CWD = корню)
        candidates.push(cwd.join("target").join("debug").join("notification.wav"));
        candidates.push(cwd.join("target").join("debug").join("assets").join("notification.wav"));
        candidates.push(cwd.join("target").join("debug").join("sounds").join("notification.wav"));
        candidates.push(cwd.join("target").join("release").join("notification.wav"));
        candidates.push(cwd.join("target").join("release").join("assets").join("notification.wav"));
        candidates.push(cwd.join("target").join("release").join("sounds").join("notification.wav"));
    }

    // 2) Рядом с .exe (для финальной поставки)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("notification.wav"));
            candidates.push(dir.join("assets").join("notification.wav"));
            candidates.push(dir.join("sounds").join("notification.wav"));
        }
    }

    // Возвращаем первый существующий
    candidates.into_iter().find(|p| p.exists())
}

// ----------------------------------------------------------------------------
//  Проиграть звук уведомления.
//  Никогда не блокирует — SND_ASYNC возвращает управление сразу.
// ----------------------------------------------------------------------------
pub fn play_notification() {
    if let Some(path) = find_sound_file() {
        // Конвертируем путь в wide-строку с нулём в конце
        let wide: Vec<u16> = path
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let _ = PlaySoundW(
                wide.as_ptr(),
                0,
                SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
            );
        }
    } else {
        // Фолбэк: системный звук «информация» (MB_ICONASTERISK = 0x40)
        unsafe {
            let _ = MessageBeep(0x00000040);
        }
    }
}
