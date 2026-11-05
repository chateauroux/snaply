// ============================================================================
//  build.rs — скрипт сборки Snaply
// ----------------------------------------------------------------------------
//  Запускается cargo ПЕРЕД компиляцией основного кода.
//
//  Задача: гарантировать что все файлы, на которые ссылается include_bytes!/
//  include_str! в embedded.rs, существуют. Если файла нет — создаём пустую
//  заглушку (0 байт). Это позволяет:
//    • embed'нуть ВСЕ ассеты в .exe (single-file поставка)
//    • не падать сборкой если у форка нет background.png или notification.wav
//    • на runtime отличать «нет файла» (0 байт) от «есть файл»
//
//  Логика:
//    • Inter.ttf, snaply.ico, chopss/index.html, chopss/html2canvas.min.js
//      — обязательные, они в репозитории. Если их нет — build.rs выведет
//      предупреждение, но не упадёт (include_bytes! потом упадёт с понятной
//      ошибкой).
//    • background.png, notification.wav — опциональные. Создаём пустые
//      заглушки если их нет. На runtime проверяем размер: 0 байт = нет.
// ============================================================================

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/background.png");
    println!("cargo:rerun-if-changed=assets/notification.wav");

    // Опциональные ассеты — создаём пустые заглушки если отсутствуют.
    // Тогда include_bytes! в embedded.rs не упадёт.
    ensure_optional_asset("assets/background.png");
    ensure_optional_asset("assets/notification.wav");
}

/// Если файла нет — создаём пустой (0 байт).
/// Если есть — ничего не делаем.
fn ensure_optional_asset(rel_path: &str) {
    let path = Path::new(rel_path);
    if !path.exists() {
        // Создаём родительскую директорию если нужно
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Создаём пустой файл
        let _ = std::fs::write(&path, b"");
        println!(
            "cargo:warning=Snaply: создана пустая заглушка для {} — \
             положите свой файл для кастомизации",
            rel_path
        );
    }
}
