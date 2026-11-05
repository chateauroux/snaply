// ============================================================================
//  single_instance.rs — гарантия, что запущен только один процесс Snaply
// ----------------------------------------------------------------------------
//  Проблема: если пользователь закрыл окно крестиком → процесс остался в трее.
//  Если снова запустить cargo run — стартует второй процесс со своей иконкой
//  трея. В трее 2 иконки, оба процесса ловят F9 — месиво.
//
//  Решение: при старте создаём named mutex (CreateMutexW). Если он уже
//  существует (GetLastError == ERROR_ALREADY_EXISTS) — значит, Snaply уже
//  запущен. Тихо выходим.
//
//  FFI напрямую: CreateMutexW и CloseHandle объявлены через extern "system".
//  Это надёжнее чем windows-sys features — не зависит от Cargo.lock.
// ============================================================================

use std::ffi::c_void;

// Win32 constants
const ERROR_ALREADY_EXISTS: u32 = 183;

// Win32 FFI declarations (kernel32.dll)
// HANDLE — это *mut c_void в windows-sys 0.59.
// SECURITY_ATTRIBUTES — передаём NULL (нам не важны).
extern "system" {
    fn CreateMutexW(
        lp_mutex_attributes: *const c_void, // NULL
        b_initial_owner: i32,               // BOOL
        lp_name: *const u16,                // PCWSTR
    ) -> *mut c_void;                       // HANDLE

    fn CloseHandle(hObject: *mut c_void) -> i32;

    fn GetLastError() -> u32;
}

/// Имя мьютекса — wide-строка с нулём в конце: "Snaply_SingleInstance\0"
const MUTEX_NAME: &[u16] = &[
    b'S' as u16, b'n' as u16, b'a' as u16, b'p' as u16, b'l' as u16, b'y' as u16,
    b'_' as u16, b'S' as u16, b'i' as u16, b'n' as u16, b'g' as u16, b'l' as u16,
    b'e' as u16, b'I' as u16, b'n' as u16, b's' as u16, b't' as u16, b'a' as u16,
    b'n' as u16, b'c' as u16, b'e' as u16, 0, // null-terminator
];

/// Guard, который держит handle мьютекса до конца программы.
/// При Drop — закрывает handle, мьютекс освобождается.
pub struct InstanceGuard(*mut c_void);

pub fn check() -> Option<InstanceGuard> {
    unsafe {
        let handle = CreateMutexW(
            std::ptr::null(),   // NULL security attributes
            1,                  // initial owner = TRUE
            MUTEX_NAME.as_ptr(),
        );
        if handle.is_null() {
            // Не удалось создать мьютекс — пускаем (лучше работать, чем не работать)
            return Some(InstanceGuard(std::ptr::null_mut()));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // Уже запущен другой экземпляр — выходим
            let _ = CloseHandle(handle);
            return None;
        }
        Some(InstanceGuard(handle))
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}
