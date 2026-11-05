// ============================================================================
//  sound.rs — звук уведомления
// ----------------------------------------------------------------------------
//  Стратегия:
//  1) notification.wav встроен в бинарник (embedded::NOTIFICATION_WAV).
//     Если embed непустой — извлекаем в %TEMP%/snaply_notification.wav и играем.
//  2) Если embed пустой (заглушка 0 байт от build.rs) — фолбэк MessageBeep.
//
//  Чтобы поставить свою мелодию (например из FL Studio):
//  • Экспортируй трек из FL как WAV (44.1kHz, 16-bit, длительность 0.3–1.0 сек)
//  • Положи в assets/notification.wav в проекте
//  • cargo build — файл встроятся в .exe
//  • Программа подхватит автоматически
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
//  Проиграть звук уведомления.
//  Никогда не блокирует — SND_ASYNC возвращает управление сразу.
// ----------------------------------------------------------------------------
pub fn play_notification() {
    // 1) Берём embed'нутый WAV. Если пустой (заглушка) — фолбэк MessageBeep.
    let wav_bytes = match crate::embedded::get_notification_wav() {
        Some(b) => b,
        None => {
            unsafe {
                let _ = MessageBeep(0x00000040); // MB_ICONASTERISK
            }
            return;
        }
    };

    // 2) Извлекаем во временную папку (PlaySoundW требует file path, не буфер).
    //    Перезаписываем всегда — если пользователь обновил .exe, WAV может
    //    измениться. Файл маленький (~50 КБ), это быстро.
    let temp_path = std::env::temp_dir().join("snaply_notification.wav");
    if let Err(_) = std::fs::write(&temp_path, wav_bytes) {
        // Не удалось записать — фолбэк MessageBeep
        unsafe {
            let _ = MessageBeep(0x00000040);
        }
        return;
    }

    // 3) Играем через PlaySoundW (SND_ASYNC — не блокирует)
    let wide: Vec<u16> = temp_path
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
}
