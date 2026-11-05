// ============================================================================
//  hotkey.rs — фоновый поток с глобальным хоткеем
// ----------------------------------------------------------------------------
//  Архитектура:
//  • RegisterHotKey(NULL, id, 0, vk) привязывает хоткей к ТЕКУЩЕМУ потоку.
//    WM_HOTKEY попадает в очередь сообщений этого потока.
//  • GetMessageW засыпает до получения сообщения → 0% CPU в простое.
//  • Переназначение хоткея делаем через PostThreadMessageW с WM_USER+1 —
//    тогда поток сам снимает старый RegisterHotKey и регистрирует новый
//    на себе (это обязательно: хоткей привязан к потоку).
// ============================================================================

use std::ptr;
use std::sync::{mpsc, Arc};

use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT,
};

use crate::Shared;

/// ID хоткея (произвольное число, передаётся в wParam WM_HOTKEY).
const HOTKEY_ID: i32 = 1;
/// Кастомное сообщение для переназначения хоткея (WM_USER + 1).
const WM_REHOTKEY: u32 = 0x0400 + 1;

// ----------------------------------------------------------------------------
//  Хэндл потока хоткеев
// ----------------------------------------------------------------------------
pub struct HotkeyHandle {
    thread_id: u32,
}

impl HotkeyHandle {
    /// Запустить фоновый поток с глобальным хоткеем.
    /// Возвращает хэндл для последующего переназначения клавиши.
    pub fn spawn(shared: Arc<Shared>) -> Self {
        let (tx, rx) = mpsc::channel();

        let shared_for_thread = shared.clone();
        std::thread::spawn(move || {
            // 1) Узнать ID этого потока и передать главному
            let tid = unsafe { GetCurrentThreadId() };
            let _ = tx.send(tid);

            // 2) Запустить рабочий цикл
            worker_loop(shared_for_thread);
        });

        let thread_id = rx.recv().expect("поток хоткеев не стартовал");
        Self { thread_id }
    }

    /// Переназначить хоткей на новый VK-код.
    /// Шлём потоку сообщение — он сам сделает UnregisterHotKey + RegisterHotKey.
    pub fn reregister(&self, new_vk: u32) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_REHOTKEY, new_vk as usize, 0);
        }
    }

    /// Корректно завершить поток хоткеев: послать WM_QUIT.
    /// GetMessageW вернёт 0, цикл прервётся, поток выйдет.
    /// ВАЖНО: без этого процесс не завершится — фоновый поток держит его живым.
    pub fn shutdown(&self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
    }
}

impl Drop for HotkeyHandle {
    fn drop(&mut self) {
        // При выходе из main() HotkeyHandle дропается → шлём WM_QUIT.
        // Иначе поток навсегда останется в GetMessageW, и процесс не закроется.
        self.shutdown();
    }
}

// ----------------------------------------------------------------------------
//  Рабочий цикл потока хоткеев
// ----------------------------------------------------------------------------
fn worker_loop(shared: Arc<Shared>) {
    unsafe {
        // 1) Первичная регистрация хоткея из настроек.
        //    HWND = NULL → хоткей привязывается к этому потоку.
        //    В windows-sys 0.59 HWND = *mut c_void, нужен ptr::null_mut().
        let vk = shared.settings.lock().unwrap().hotkey;
        if RegisterHotKey(ptr::null_mut(), HOTKEY_ID, 0, vk) == 0 {
            eprintln!(
                "Snaply: RegisterHotKey не удался для VK 0x{:02X} — возможно, клавиша занята",
                vk
            );
            return;
        }

        // 2) Цикл сообщений: спит в GetMessageW до события
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_HOTKEY && (msg.wParam as i32) == HOTKEY_ID {
                // --- Сработал хоткей: делаем скриншот ---
                let s = shared.settings.lock().unwrap();
                let path = s.save_path.clone();
                let notif = s.show_notifications;
                let copy = s.auto_copy;
                let fmt = s.format;
                drop(s);

                // Тост показывается в capture::take_screenshot — после того
                // как файл сохранён, с реальным именем файла. Передаём
                // notif=true. Будим UI сразу, чтобы не ждать 250ms.
                shared.wake_ui();

                // Скриншот в отдельном потоке — не блокирует GetMessageW.
                let notif_shared = shared.notif.clone();
                let status_shared = shared.clone();
                std::thread::spawn(move || {
                    match crate::capture::take_screenshot(&path, notif, copy, fmt, &notif_shared) {
                        Ok(fname) => status_shared.set_status(format!("✓ {}", fname)),
                        Err(e) => status_shared.set_status(format!("✗ {}", e)),
                    }
                });
            } else if msg.message == WM_REHOTKEY {
                // --- Просьба переназначить хоткей ---
                let new_vk = msg.wParam as u32;
                UnregisterHotKey(ptr::null_mut(), HOTKEY_ID);
                if RegisterHotKey(ptr::null_mut(), HOTKEY_ID, 0, new_vk) != 0 {
                    // Успех — обновляем настройки и сохраняем
                    let mut s = shared.settings.lock().unwrap();
                    s.hotkey = new_vk;
                    s.save();
                    drop(s);
                    shared.set_status(format!("✓ Хоткей: {}", crate::config::vk_label(new_vk)));
                } else {
                    // Не вышло (клавиша занята системой) — откатываемся
                    let old_vk = shared.settings.lock().unwrap().hotkey;
                    RegisterHotKey(ptr::null_mut(), HOTKEY_ID, 0, old_vk);
                    shared.set_status("✗ Клавиша занята".into());
                }
            }
            // WM_QUIT (от Drop HotkeyHandle): GetMessageW вернёт 0,
            // выходим из while, снимаем RegisterHotKey и поток завершается.
        }

        // Снимаем глобальный хоткей при выходе из потока.
        // Если этого не сделать, F9 останется занят до перезагрузки системы.
        UnregisterHotKey(ptr::null_mut(), HOTKEY_ID);
    }
}
