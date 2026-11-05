// ============================================================================
//  notification.rs — кастомный тост-оверлей в стиле Steam
// ----------------------------------------------------------------------------
//  Почему не системный Toast (winrt-notification)?
//  • Медленно: от 0.5 до 1.5 секунд до появления
//  • Зависит от настроек Windows (Focus Assist может блокировать)
//  • Стиль не настраивается
//
//  Решение: отдельное окно egui без рамок, прозрачное, AlwaysOnTop.
//  Появляется справа внизу экрана с плавным fade-in, живёт 2.5 сек,
//  плавно исчезает. Полностью наш стиль — тёмный фон + красный акцент.
//
//  Структура тоста:
//    ┌────────────────────────────────────────┐
//    │ ▌  📷  SNAPLY                          │  ← заголовок (красный)
//    │ ▌      Screen_2026-11-05_21-19-46.png  │  ← имя файла (белый)
//    └────────────────────────────────────────┘
//    Левая полоса-акцент повторяет скругление левого края тоста.
// ============================================================================

use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Id, Pos2, Rounding, Stroke, Vec2, ViewportBuilder, ViewportId};

use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

// ----------------------------------------------------------------------------
//  Состояние активной нотификации
// ----------------------------------------------------------------------------
pub struct Notification {
    /// Заголовок (например «Скриншот сохранён» — но мы используем «SNAPLY»)
    pub title: String,
    /// Тело — например имя файла «Screen_2026-11-05_21-19-46.png»
    pub body: String,
    pub started_at: Instant,
}

pub type SharedNotif = Arc<Mutex<Option<Notification>>>;

pub fn make_shared() -> SharedNotif {
    Arc::new(Mutex::new(None))
}

/// Показать тост. Потокобезопасно: кладём в shared-состояние,
/// а главный поток отрисует overlay в следующем кадре.
/// Также проигрывает звук уведомления (см. sound.rs).
///
/// `title` — крупный заголовок (например «Скриншот сохранён»).
/// `body`  — мелкий текст под заголовком (например имя файла).
pub fn show(shared: &SharedNotif, title: &str, body: &str) {
    *shared.lock().unwrap() = Some(Notification {
        title: title.to_string(),
        body: body.to_string(),
        started_at: Instant::now(),
    });
    // Звук — неблокирующий, можно вызывать из любого потока
    crate::sound::play_notification();
}

// ----------------------------------------------------------------------------
//  Палитра тоста (тёмная + красный акцент, как в Steam)
// ----------------------------------------------------------------------------
const TOAST_BG:     Color32 = Color32::from_rgb(18, 14, 16);
const TOAST_BORDER: Color32 = Color32::from_rgb(255, 71, 87);  // #FF4757 — жирная красная обводка
const TOAST_TEXT:   Color32 = Color32::from_rgb(240, 240, 245);
const TOAST_SUBTLE: Color32 = Color32::from_rgb(155, 150, 150);
const TOAST_ACCENT: Color32 = Color32::from_rgb(255, 71, 87); // #FF4757 — красный

const TOAST_W: f32 = 320.0;
const TOAST_H: f32 = 78.0;
const MARGIN:  f32 = 18.0;
const DURATION_SECS: f64 = 2.5;
const FADE_SECS:     f64 = 0.25;

// ----------------------------------------------------------------------------
//  Вызывается из SnaplyApp::update() каждый кадр.
// ----------------------------------------------------------------------------
pub fn render_overlay(ctx: &egui::Context, shared: &SharedNotif) {
    // 1) Достаём текущую нотификацию
    let (title, body, elapsed_secs) = {
        let guard = shared.lock().unwrap();
        match guard.as_ref() {
            Some(n) => (n.title.clone(), n.body.clone(), n.started_at.elapsed().as_secs_f64()),
            None => return,
        }
    };

    // 2) Время вышло — очищаем
    if elapsed_secs > DURATION_SECS + FADE_SECS {
        *shared.lock().unwrap() = None;
        return;
    }

    // 3) Позиция: правый нижний угол ЭКРАНА (через Win32, не ctx.screen_rect)
    let (screen_w, screen_h) = unsafe {
        (GetSystemMetrics(SM_CXSCREEN) as f32, GetSystemMetrics(SM_CYSCREEN) as f32)
    };
    let x = screen_w - TOAST_W - MARGIN;
    let y = screen_h - TOAST_H - MARGIN - 48.0;

    // 4) Анимация: fade-in → 1.0 → fade-out
    let alpha = if elapsed_secs < FADE_SECS {
        (elapsed_secs / FADE_SECS) as f32
    } else if elapsed_secs > DURATION_SECS {
        let t = (elapsed_secs - DURATION_SECS) / FADE_SECS;
        1.0 - t as f32
    } else {
        1.0
    };
    let slide = (1.0 - alpha) * 12.0;

    // 5) Открываем overlay-окно
    let viewport_id = ViewportId(Id::new("snaply_toast_overlay"));
    let builder = ViewportBuilder::default()
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size([TOAST_W, TOAST_H])
        .with_position(Pos2::new(x, y - slide))
        .with_window_level(egui::WindowLevel::AlwaysOnTop)
        .with_taskbar(false)
        .with_resizable(false)
        .with_visible(true);

    let title_c = title.clone();
    let body_c = body.clone();
    ctx.show_viewport_deferred(viewport_id, builder, move |ctx, _| {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                draw_toast(ui, &title_c, &body_c, alpha);
            });
    });
}

// ----------------------------------------------------------------------------
//  Отрисовка тоста — простой прямоугольник без скругления, с жирной обводкой
// ----------------------------------------------------------------------------
fn draw_toast(ui: &mut egui::Ui, title: &str, body: &str, alpha: f32) {
    let painter = ui.painter();

    // Внутренний отступ, чтобы обводка не обрезалась краем окна
    let pad = 4.0;
    let outer = ui.max_rect();
    let rect = outer.shrink(pad);
    // БЕЗ скругления — простой прямоугольник, как просил пользователь
    let rounding = Rounding::ZERO;

    // Применяем альфу ко всем цветам
    let bg     = with_alpha(TOAST_BG,     alpha);
    let border = with_alpha(TOAST_BORDER, alpha);
    let text_c = with_alpha(TOAST_TEXT,   alpha);
    let sub_c  = with_alpha(TOAST_SUBTLE, alpha);
    let accent = with_alpha(TOAST_ACCENT, alpha);

    // 1) Мягкая тень под тостом
    let shadow = with_alpha(Color32::from_rgb(0, 0, 0), alpha * 0.5);
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 3.0)),
        rounding,
        shadow,
    );

    // 2) Основной фон (прямоугольник, без скругления)
    painter.rect_filled(rect, rounding, bg);

    // 3) ЖИРНАЯ обводка (3px) — вместо красной полоски слева.
    //    Так аутентичнее и не ломает геометрию.
    painter.rect_stroke(rect, rounding, Stroke::new(3.0_f32, border));

    // 4) Иконка камеры слева (с отступом от левой обводки)
    let icon_pos = Pos2::new(rect.min.x + 28.0, rect.center().y);
    painter.text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        "📸",
        FontId::proportional(22.0),
        text_c,
    );

    // 5) Заголовок (красный, мелкий капс) — над основным текстом
    let title_pos = Pos2::new(rect.min.x + 56.0, rect.min.y + 14.0);
    painter.text(
        title_pos,
        egui::Align2::LEFT_TOP,
        title,
        FontId::proportional(9.0),
        accent,
    );

    // 6) Основной текст — имя файла или описание.
    //    Обрезаем с многоточием, если не влезает по ширине.
    let text_pos = Pos2::new(rect.min.x + 56.0, rect.min.y + 28.0);
    let max_w = rect.width() - 76.0;
    let display_body = truncate_text(body, max_w, 13.0);
    painter.text(
        text_pos,
        egui::Align2::LEFT_TOP,
        &display_body,
        FontId::proportional(13.0),
        text_c,
    );

    // 7) Внизу мелкая подпись-статус
    let sub_pos = Pos2::new(rect.min.x + 56.0, rect.max.y - 16.0);
    painter.text(
        sub_pos,
        egui::Align2::LEFT_TOP,
        "Сохранено в папку",
        FontId::proportional(10.0),
        sub_c,
    );
}

// ----------------------------------------------------------------------------
//  Хелперы
// ----------------------------------------------------------------------------

/// Применить альфу к цвету (умножение существующей альфы)
fn with_alpha(c: Color32, a: f32) -> Color32 {
    let a = (a.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Обрезать текст с многоточием, если он не влезает по ширине.
fn truncate_text(text: &str, max_w: f32, font_size: f32) -> String {
    let approx_char_w = font_size * 0.55;
    let max_chars = (max_w / approx_char_w) as usize;
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}
