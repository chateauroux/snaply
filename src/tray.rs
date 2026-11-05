// ============================================================================
//  tray.rs — иконка в системном трее с клик-событиями
// ----------------------------------------------------------------------------
//  Используем крейт tray-icon: он создаёт скрытое message-only окно под
//  капотом и корректно получает клики. В отличие от raw Shell_NotifyIconW
//  с hWnd=0 (которая вообще не получает сообщений), здесь клик по иконке
//  восстанавливает окно.
//
//  Иконка встроена в бинарник (embedded::ICON_ICO). Если по какой-то причине
//  не декодировалась — фолбэк: бордовый круг, генерируемый в памяти.
//
//  Drop impl: при выходе из приложения (в т.ч. через «Выход» из меню трея)
//  TrayIcon удаляется, и Windows сразу убирает иконку из трея.
// ============================================================================

use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Идентификаторы пунктов меню.
pub const MENU_SHOW: &str = "snaply_show";
pub const MENU_QUIT: &str = "snaply_quit";

/// Обёртка над tray-icon. Держим TrayIcon, чтобы он жил всё время работы приложения.
pub struct TrayManager {
    tray: TrayIcon,
}

impl TrayManager {
    pub fn new() -> Result<Self, String> {
        // Меню: «Показать окно» / разделитель / «Выход»
        let menu = Menu::new();
        let _ = menu.append(&MenuItem::with_id(MENU_SHOW, "Показать окно", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(MENU_QUIT, "Выход", true, None));

        // Пытаемся загрузить .ico пользователя, иначе — генерируем фолбэк
        let icon = load_icon().unwrap_or_else(|| make_fallback_icon());

        let tray = TrayIconBuilder::new()
            .with_tooltip("Snaply — быстрые скриншоты (F9)")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self { tray })
    }

    /// Явно убрать иконку из трея (вызывается при выходе).
    pub fn remove(&mut self) {
        self.tray.set_visible(false).ok();
    }
}

impl Drop for TrayManager {
    fn drop(&mut self) {
        self.remove();
    }
}

// ----------------------------------------------------------------------------
//  Загрузка иконки из встроенного в бинарник snaply.ico
// ----------------------------------------------------------------------------
fn load_icon() -> Option<Icon> {
    // Иконка встроена в бинарник (embedded::ICON_ICO).
    let bytes = crate::embedded::ICON_ICO;
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let w = rgba.width();
    let h = rgba.height();
    Icon::from_rgba(rgba.into_raw(), w, h).ok()
}

// ----------------------------------------------------------------------------
//  Фолбэк-иконка: бордовый круг 32×32 (если .ico не найден)
// ----------------------------------------------------------------------------
fn make_fallback_icon() -> Icon {
    let size: u32 = 32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let i = ((y * size + x) * 4) as usize;
            if d < 11.0 {
                rgba[i] = 225;
                rgba[i + 1] = 29;
                rgba[i + 2] = 46;
                rgba[i + 3] = 255;
            } else if d < 12.5 {
                let alpha = ((12.5 - d) / 1.5 * 255.0) as u8;
                rgba[i] = 225;
                rgba[i + 1] = 29;
                rgba[i + 2] = 46;
                rgba[i + 3] = alpha;
            }
        }
    }
    Icon::from_rgba(rgba, size, size)
        .unwrap_or_else(|_| Icon::from_rgba(vec![0, 0, 0, 255], 1, 1).unwrap())
}
