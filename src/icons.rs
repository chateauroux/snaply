// ============================================================================
//  icons.rs — векторные иконки через egui::Painter
// ----------------------------------------------------------------------------
//  Все иконки рисуются примитивами (линии, круги, прямоугольники) — никаких
//  эмодзи и Unicode-символов, которые на Windows 10 могут отображаться
//  квадратиками. Иконки масштабируются под размер rect, цвета — параметр.
//
//  Каждая функция принимает (painter, rect, color) и рисует иконку
//  вписанную в rect, с отступами ~15% от размера.
// ============================================================================

use eframe::egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

// ----------------------------------------------------------------------------
//  Хелпер: получить центр и масштаб для rect
// ----------------------------------------------------------------------------
fn metrics(rect: Rect) -> (Pos2, f32, f32) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let half = size / 2.0;
    (center, half, size)
}

// ----------------------------------------------------------------------------
//  Иконка камеры — для главной кнопки скриншота
// ----------------------------------------------------------------------------
pub fn camera(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let s = size * 0.35; // полу-размер корпуса камеры
    let stroke = Stroke::new(size * 0.06, color);

    // Корпус камеры (скруглённый прямоугольник)
    let body = Rect::from_center_size(c, Vec2::splat(s * 2.0));
    painter.rect_stroke(body, Rounding::same(s * 0.2), stroke);

    // «Горка» сверху (вспышка) — треугольник
    let top_y = body.top();
    let peak = Pos2::new(c.x - s * 0.4, top_y);
    let left = Pos2::new(c.x - s * 0.7, top_y + s * 0.3);
    let right = Pos2::new(c.x - s * 0.1, top_y + s * 0.3);
    painter.line_segment([peak, left], stroke);
    painter.line_segment([peak, right], stroke);

    // Линза — круг в центре
    let lens_r = s * 0.55;
    painter.circle_stroke(c, lens_r, stroke);
    // Внутренний круг линзы
    painter.circle_stroke(c, lens_r * 0.55, stroke);
}

// ----------------------------------------------------------------------------
//  Иконка шестерёнки — для настроек
// ----------------------------------------------------------------------------
pub fn gear(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.07, color);
    let r_outer = size * 0.38;
    let r_inner = size * 0.22;
    let r_hole = size * 0.12;

    // Внешнее кольцо
    painter.circle_stroke(c, r_outer, stroke);
    // Внутреннее кольцо
    painter.circle_stroke(c, r_inner, stroke);
    // Отверстие
    painter.circle_stroke(c, r_hole, stroke);

    // 8 зубцов — радиальные линии
    let teeth = 8;
    for i in 0..teeth {
        let angle = (i as f32) * std::f32::consts::TAU / teeth as f32;
        let p1 = c + Vec2::new(angle.cos() * r_inner, angle.sin() * r_inner);
        let p2 = c + Vec2::new(angle.cos() * r_outer, angle.sin() * r_outer);
        painter.line_segment([p1, p2], stroke);
    }
}

// ----------------------------------------------------------------------------
//  Иконка карандаша — для редактирования хоткея
// ----------------------------------------------------------------------------
pub fn pencil(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.08, color);
    let s = size * 0.32;

    // Карандаш рисуется по диагонали (сверху-справа вниз-влево)
    let top_right = Pos2::new(c.x + s, c.y - s);
    let bottom_left = Pos2::new(c.x - s, c.y + s);

    // Две линии образуют корпус карандаша
    let perp = Vec2::new(-s * 0.35, -s * 0.35); // перпендикуляр
    let tr1 = top_right + perp;
    let tr2 = top_right - perp;
    let bl1 = bottom_left + perp;
    let bl2 = bottom_left - perp;

    painter.line_segment([tr1, bl1], stroke);
    painter.line_segment([tr2, bl2], stroke);
    // Кончики (поперечные)
    painter.line_segment([tr1, tr2], stroke);
    // Остриё
    painter.line_segment([bl1, bl2], stroke);

    // Маленький квадратик у острия (графит)
    let tip_size = s * 0.2;
    let tip_rect = Rect::from_center_size(bottom_left, Vec2::splat(tip_size));
    painter.rect_stroke(tip_rect, Rounding::same(1.0), stroke);
}

// ----------------------------------------------------------------------------
//  Иконка папки — для выбора пути сохранения
// ----------------------------------------------------------------------------
pub fn folder(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.07, color);
    let s = size * 0.35;

    // Корпус папки
    let body = Rect::from_center_size(
        Pos2::new(c.x, c.y + s * 0.15),
        Vec2::new(s * 2.0, s * 1.4),
    );
    painter.rect_stroke(body, Rounding::same(s * 0.15), stroke);

    // «Язычок» сверху
    let tab = Rect::from_min_max(
        Pos2::new(body.left(), body.top() - s * 0.4),
        Pos2::new(body.left() + s * 0.9, body.top()),
    );
    painter.rect_stroke(tab, Rounding::same(s * 0.1), stroke);
}

// ----------------------------------------------------------------------------
//  Иконка стрелки вправо →
// ----------------------------------------------------------------------------
pub fn arrow_right(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.1, color);
    let s = size * 0.3;

    // Горизонтальная линия
    let left = Pos2::new(c.x - s, c.y);
    let right = Pos2::new(c.x + s, c.y);
    painter.line_segment([left, right], stroke);

    // Остриё стрелы (две линии под углом)
    let tip1 = Pos2::new(c.x + s * 0.4, c.y - s * 0.5);
    let tip2 = Pos2::new(c.x + s * 0.4, c.y + s * 0.5);
    painter.line_segment([right, tip1], stroke);
    painter.line_segment([right, tip2], stroke);
}

// ----------------------------------------------------------------------------
//  Иконка дома — для главной страницы навигации
// ----------------------------------------------------------------------------
pub fn home(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.08, color);
    let s = size * 0.35;

    // Крыша — треугольник
    let top = Pos2::new(c.x, c.y - s);
    let left = Pos2::new(c.x - s, c.y - s * 0.1);
    let right = Pos2::new(c.x + s, c.y - s * 0.1);
    painter.line_segment([top, left], stroke);
    painter.line_segment([top, right], stroke);
    painter.line_segment([left, right], stroke);

    // Корпус дома
    let body_top = c.y - s * 0.1;
    let body_bot = c.y + s;
    painter.line_segment(
        [Pos2::new(c.x - s * 0.85, body_top), Pos2::new(c.x - s * 0.85, body_bot)],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(c.x + s * 0.85, body_top), Pos2::new(c.x + s * 0.85, body_bot)],
        stroke,
    );
    painter.line_segment(
        [Pos2::new(c.x - s * 0.85, body_bot), Pos2::new(c.x + s * 0.85, body_bot)],
        stroke,
    );

    // Дверь
    let door = Rect::from_center_size(
        Pos2::new(c.x, body_bot - s * 0.35),
        Vec2::new(s * 0.4, s * 0.7),
    );
    painter.rect_stroke(door, Rounding::same(s * 0.05), stroke);
}

// ----------------------------------------------------------------------------
//  Иконка «i» в круге — для страницы «О программе»
// ----------------------------------------------------------------------------
pub fn info(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.08, color);
    let r = size * 0.38;

    // Круг
    painter.circle_stroke(c, r, stroke);

    // Точка над i
    let dot_y = c.y - r * 0.45;
    painter.circle_filled(Pos2::new(c.x, dot_y), size * 0.045, color);

    // Вертикальная линия «i»
    let line_top = c.y - r * 0.15;
    let line_bot = c.y + r * 0.5;
    painter.line_segment(
        [Pos2::new(c.x, line_top), Pos2::new(c.x, line_bot)],
        stroke,
    );
}

// ----------------------------------------------------------------------------
//  Иконка «минимизировать» (стрелка вниз) — для кнопки «Свернуть в трей»
// ----------------------------------------------------------------------------
pub fn chevron_down(painter: &Painter, rect: Rect, color: Color32) {
    let (c, _half, size) = metrics(rect);
    let stroke = Stroke::new(size * 0.1, color);
    let s = size * 0.3;

    let top_left = Pos2::new(c.x - s, c.y - s * 0.3);
    let bottom = Pos2::new(c.x, c.y + s * 0.5);
    let top_right = Pos2::new(c.x + s, c.y - s * 0.3);

    painter.line_segment([top_left, bottom], stroke);
    painter.line_segment([top_right, bottom], stroke);
}

// ----------------------------------------------------------------------------
//  Логотип Snaply — стилизованная камера в круге (для шапки)
// ----------------------------------------------------------------------------
pub fn logo(painter: &Painter, rect: Rect, accent: Color32, surface: Color32) {
    let (c, _, size) = metrics(rect);
    let r = size * 0.45;

    // Внешний круг — фон
    painter.circle_filled(c, r, accent);
    // Внутренний круг — поверхность (для контраста)
    painter.circle_filled(c, r * 0.85, surface);
    // Камера внутри
    let inner_rect = Rect::from_center_size(c, Vec2::splat(r * 1.3));
    camera(painter, inner_rect, accent);
}

// ----------------------------------------------------------------------------
//  Универсальный помощник: нарисовать иконку по имени в заданном rect
// ----------------------------------------------------------------------------
pub fn draw_named(painter: &Painter, name: &str, rect: Rect, color: Color32) {
    match name {
        "camera" => camera(painter, rect, color),
        "gear" => gear(painter, rect, color),
        "pencil" => pencil(painter, rect, color),
        "folder" => folder(painter, rect, color),
        "arrow_right" => arrow_right(painter, rect, color),
        "home" => home(painter, rect, color),
        "info" => info(painter, rect, color),
        "chevron_down" => chevron_down(painter, rect, color),
        _ => {}
    }
}
