// ============================================================================
//  material.rs — тёмная тема + Material-виджеты + подключение шрифтов
// ----------------------------------------------------------------------------
//  Палитра: почти чёрный фон, тёмно-серые поверхности, бордовый/красный акцент.
//  Шрифты: Inter (OFL), подключается из assets/fonts/Inter.ttf.
//  Виджеты: card, filled_button (с векторной иконкой), outlined_button,
//  switch (анимированный), segmented (для выбора формата), key_badge,
//  nav_button (для боковой навигации).
// ============================================================================

use eframe::egui::{
    self, Align2, Color32, FontId, Margin, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui,
    Vec2, Visuals,
};

use crate::icons;

// ----------------------------------------------------------------------------
//  Палитра — тёмный фон + бордовые/красные акценты
// ----------------------------------------------------------------------------
#[allow(dead_code)]
pub const BG:          Color32 = Color32::from_rgb(10,  8,   10); // #0A080A — почти чёрный
#[allow(dead_code)]
pub const BG_GRADIENT: Color32 = Color32::from_rgb(18,  8,   10); // нижний градиент
pub const SURFACE:     Color32 = Color32::from_rgb(18,  14,  16); // #120E10 — карточки
pub const SURFACE_VAR: Color32 = Color32::from_rgb(28,  22,  24); // #1C1618 — elevated
pub const SURFACE_HOV: Color32 = Color32::from_rgb(38,  28,  30); // #261C1E — hover
pub const OUTLINE:     Color32 = Color32::from_rgb(50,  36,  38); // #322426 — рамки
pub const PRIMARY:     Color32 = Color32::from_rgb(225, 29,  46); // #E11D2E — бордовый
pub const PRIMARY_DIM: Color32 = Color32::from_rgb(110, 18,  28); // #6E121C — container
pub const ACCENT:      Color32 = Color32::from_rgb(255, 71,  87); // #FF4757 — яркий красный
pub const ON_PRIMARY:  Color32 = Color32::from_rgb(255, 245, 245);
pub const ON_SURFACE:  Color32 = Color32::from_rgb(235, 230, 230);
pub const ON_SURF_VAR: Color32 = Color32::from_rgb(155, 145, 145);
pub const SUCCESS:     Color32 = Color32::from_rgb(34,  197, 94);
pub const ERROR:       Color32 = Color32::from_rgb(239, 68,  68);

// ----------------------------------------------------------------------------
//  Применить тёмную тему + подключить шрифты Inter
// ----------------------------------------------------------------------------
pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut v = Visuals::dark();
    v.panel_fill       = Color32::TRANSPARENT; // прозрачный — фон рисуем сами
    v.extreme_bg_color = Color32::from_rgb(5, 4, 5);
    v.window_fill      = SURFACE;
    v.faint_bg_color   = SURFACE;
    v.code_bg_color    = SURFACE_VAR;
    v.hyperlink_color  = ACCENT;
    v.selection.bg_fill = PRIMARY;
    v.selection.stroke  = Stroke::new(1.0_f32, ON_PRIMARY);

    v.widgets.noninteractive.bg_fill    = SURFACE;
    v.widgets.noninteractive.fg_stroke  = Stroke::new(1.0_f32, ON_SURF_VAR);
    v.widgets.noninteractive.bg_stroke  = Stroke::new(1.0_f32, OUTLINE);
    v.widgets.noninteractive.rounding   = Rounding::same(10.0);

    v.widgets.inactive.bg_fill   = SURFACE_VAR;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, ON_SURFACE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, OUTLINE);
    v.widgets.inactive.rounding  = Rounding::same(10.0);

    v.widgets.hovered.bg_fill   = SURFACE_HOV;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, PRIMARY);
    v.widgets.hovered.rounding  = Rounding::same(10.0);

    v.widgets.active.bg_fill   = PRIMARY_DIM;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.active.rounding  = Rounding::same(10.0);

    v.widgets.open.bg_fill   = SURFACE_HOV;
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, PRIMARY);
    v.widgets.open.rounding  = Rounding::same(10.0);

    v.window_rounding = Rounding::same(16.0);
    v.window_stroke   = Stroke::new(1.0_f32, OUTLINE);
    v.menu_rounding   = Rounding::same(12.0);

    ctx.set_visuals(v);
}

// ----------------------------------------------------------------------------
//  Установка шрифтов Inter (OFL, см. assets/fonts/LICENSE.txt)
// ----------------------------------------------------------------------------
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Пытаемся загрузить Inter из возможных мест (assets/fonts/, рядом с exe)
    if let Some(ttf_bytes) = load_inter_font() {
        fonts.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_owned(ttf_bytes).into(),
        );

        // Inter как Proportional (основной шрифт UI)
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Inter".to_owned());

        // Семейство по имени «Inter» — для явного использования
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Name("Inter".into())) {
            fam.insert(0, "Inter".to_owned());
        }

        // Также для Monospace — на всякий случай, чтобы не было квадратиков
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.push("Inter".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

fn load_inter_font() -> Option<Vec<u8>> {
    // Шрифт встроен в бинарник (embedded::INTER_FONT).
    // Если по какой-то причине embed недоступен — пробуем с диска.
    Some(crate::embedded::INTER_FONT.to_vec())
}

// ----------------------------------------------------------------------------
//  Card — поверхность с outline и скруглением 16dp
// ----------------------------------------------------------------------------
pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(SURFACE)
        .rounding(Rounding::same(16.0))
        .stroke(Stroke::new(1.0_f32, OUTLINE))
        .inner_margin(Margin::same(20.0))
        .show(ui, add);
}

// ----------------------------------------------------------------------------
//  Filled Button — крупная красная кнопка с векторной иконкой и стрелкой
//  icon: имя иконки из icons.rs (например "camera"). None = без иконки.
//  Возвращает true при клике.
// ----------------------------------------------------------------------------
pub fn filled_button(ui: &mut Ui, text: &str, icon_name: Option<&str>) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 56.0),
        Sense::click(),
    );
    let painter = ui.painter();

    // Холст кнопки
    let bg = if resp.hovered() {
        ACCENT
    } else if resp.is_pointer_button_down_on() {
        PRIMARY_DIM
    } else {
        PRIMARY
    };
    painter.rect_filled(rect, Rounding::same(12.0), bg);

    // Тонкая внутренняя обводка для глубины
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0_f32, Color32::from_rgb(255, 100, 110)));

    // Иконка слева (если есть) — рисуем векторную
    let mut text_x = rect.left() + 20.0;
    if let Some(name) = icon_name {
        let icon_size = 24.0;
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 28.0, rect.center().y),
            Vec2::splat(icon_size),
        );
        icons::draw_named(painter, name, icon_rect, ON_PRIMARY);
        text_x = icon_rect.right() + 12.0;
    }

    // Текст по центру (с учётом иконки слева и стрелки справа)
    let arrow_size = 18.0;
    let arrow_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 28.0, rect.center().y),
        Vec2::splat(arrow_size),
    );
    let text_rect = Rect::from_min_max(
        Pos2::new(text_x, rect.top()),
        Pos2::new(arrow_rect.left() - 8.0, rect.bottom()),
    );
    painter.text(
        text_rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(16.0),
        ON_PRIMARY,
    );

    // Стрелка справа
    icons::draw_named(painter, "arrow_right", arrow_rect, ON_PRIMARY);

    resp.clicked()
}

// ----------------------------------------------------------------------------
//  Outlined Button — прозрачный, красная рамка, красный текст
// ----------------------------------------------------------------------------
#[allow(dead_code)]
pub fn outlined_button(ui: &mut Ui, text: &str) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(text)
            .color(ACCENT)
            .font(FontId::proportional(13.0))
            .strong(),
    )
    .fill(Color32::TRANSPARENT)
    .rounding(Rounding::same(10.0))
    .stroke(Stroke::new(1.0_f32, PRIMARY))
    .min_size(Vec2::new(88.0, 34.0));
    ui.add(btn).clicked()
}

// ----------------------------------------------------------------------------
//  Outlined Button с векторной иконкой (для «Обзор», «Свернуть в трей»)
//  Ширина адаптивна к тексту: иконка (16) + gap (8) + текст + padding (24).
//  Если текст длиннее min_width — кнопка растётся. Если короче — min_width.
// ----------------------------------------------------------------------------
pub fn outlined_button_icon(ui: &mut Ui, text: &str, icon_name: &str) -> bool {
    // Эвристика ширины текста: ~0.6 * font_size * chars
    let text_w = text.chars().count() as f32 * 13.0 * 0.6;
    // иконка (16) + gap (8) + текст + правый padding (16) + левый padding (8)
    let computed_w = 16.0 + 8.0 + text_w + 16.0 + 8.0;
    let w = computed_w.max(88.0); // минимальная ширина

    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(w, 34.0),
        Sense::click(),
    );
    let painter = ui.painter();

    let stroke_color = if resp.hovered() { ACCENT } else { PRIMARY };
    let fill = if resp.hovered() { SURFACE_HOV } else { Color32::TRANSPARENT };
    painter.rect_filled(rect, Rounding::same(10.0), fill);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, stroke_color));

    // Иконка слева
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 16.0, rect.center().y),
        Vec2::splat(16.0),
    );
    let text_color = if resp.hovered() { ACCENT } else { PRIMARY };
    icons::draw_named(painter, icon_name, icon_rect, text_color);

    // Текст — выравнивание по левому краю после иконки, с небольшим отступом
    let text_rect = Rect::from_min_max(
        Pos2::new(icon_rect.right() + 8.0, rect.top()),
        Pos2::new(rect.right() - 12.0, rect.bottom()),
    );
    painter.text(
        text_rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        text_color,
    );

    resp.clicked()
}

// ----------------------------------------------------------------------------
//  Switch — анимированный тумблер (как в Material 3)
// ----------------------------------------------------------------------------
pub fn switch(ui: &mut Ui, checked: &mut bool, id_source: &str) -> Response {
    let anim_id = ui.id().with(id_source);
    let size = Vec2::new(46.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    let target = if *checked { 1.0_f32 } else { 0.0_f32 };
    let t = ui.ctx().animate_value_with_time(anim_id, target, 0.15);

    let painter = ui.painter();

    // Трек
    let track_rect = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 18.0));
    let track_color = if *checked {
        PRIMARY
    } else {
        Color32::from_rgb(60, 50, 52)
    };
    painter.rect_filled(track_rect, Rounding::same(9.0), track_color);

    // Палец (увеличивается при включении)
    let thumb_radius = 9.0 + t * 3.0;
    let travel = (rect.width() - thumb_radius * 2.0 - 4.0).max(0.0);
    let thumb_x = rect.left() + 2.0 + thumb_radius + t * travel;
    let thumb_y = rect.center().y;
    let thumb_color = if *checked {
        ON_PRIMARY
    } else {
        Color32::from_rgb(180, 175, 175)
    };
    painter.circle_filled(Pos2::new(thumb_x, thumb_y), thumb_radius, thumb_color);

    if resp.clicked() {
        *checked = !*checked;
    }
    resp
}

// ----------------------------------------------------------------------------
//  Segmented — для выбора формата PNG/JPG/BMP
// ----------------------------------------------------------------------------
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    let count = options.len();
    let total_w = ui.available_width();
    let h = 40.0;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, h), Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, Rounding::same(10.0), SURFACE);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, OUTLINE));

    let seg_w = total_w / count as f32;
    for (i, (val, label)) in options.iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            Pos2::new(rect.left() + i as f32 * seg_w, rect.top()),
            Vec2::new(seg_w, h),
        );
        let selected = *value == *val;
        let resp = ui.interact(
            seg_rect,
            ui.id().with("seg").with(i),
            Sense::click(),
        );

        if resp.hovered() {
            painter.rect_filled(seg_rect, Rounding::same(10.0), SURFACE_HOV);
        }
        if selected {
            let inner = seg_rect.shrink2(Vec2::new(3.0, 3.0));
            painter.rect_filled(inner, Rounding::same(8.0), PRIMARY_DIM);
        }

        if resp.clicked() && !selected {
            *value = *val;
            changed = true;
        }

        let text_color = if selected { ACCENT } else { ON_SURF_VAR };
        painter.text(
            seg_rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            text_color,
        );
    }
    changed
}

// ----------------------------------------------------------------------------
//  Key Badge — бейдж для отображения текущей клавиши
// ----------------------------------------------------------------------------
pub fn key_badge(ui: &mut Ui, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(60.0, 30.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), SURFACE_VAR);
    painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, OUTLINE));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        ACCENT,
    );
}

// ----------------------------------------------------------------------------
//  Icon Button — квадратная кнопка с векторной иконкой (для ✎ карандаша)
// ----------------------------------------------------------------------------
pub fn icon_button(ui: &mut Ui, icon_name: &str, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let painter = ui.painter();

    let bg = if resp.hovered() { SURFACE_HOV } else { SURFACE_VAR };
    let stroke = if resp.hovered() { PRIMARY } else { OUTLINE };
    let icon_color = if resp.hovered() { ACCENT } else { ON_SURFACE };

    painter.rect_filled(rect, Rounding::same(8.0), bg);
    painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, stroke));

    // Иконка на 60% размера кнопки
    let icon_rect = Rect::from_center_size(
        rect.center(),
        Vec2::splat(size * 0.6),
    );
    icons::draw_named(painter, icon_name, icon_rect, icon_color);

    resp
}

// ----------------------------------------------------------------------------
//  Nav Button — пункт боковой навигации
//  active: true → подсвечен красным, слева красная полоса
//  collapsed: true → только иконка, без текста
// ----------------------------------------------------------------------------
pub fn nav_button(
    ui: &mut Ui,
    icon_name: &str,
    label: &str,
    active: bool,
    collapsed: bool,
) -> Response {
    let h = 44.0;
    let w = if collapsed { 44.0 } else { ui.available_width() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let painter = ui.painter();

    // Фон
    let bg = if active {
        PRIMARY_DIM
    } else if resp.hovered() {
        SURFACE_HOV
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, Rounding::same(10.0), bg);
    }

    // Левая красная полоса для активного пункта
    if active {
        let bar = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + 6.0),
            Vec2::new(3.0, rect.height() - 12.0),
        );
        painter.rect_filled(bar, Rounding::same(1.5), ACCENT);
    }

    // Иконка
    let icon_color = if active { ACCENT } else { ON_SURF_VAR };
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Vec2::splat(20.0),
    );
    icons::draw_named(painter, icon_name, icon_rect, icon_color);

    // Текст (если не свёрнута навигация)
    if !collapsed {
        let text_color = if active { ON_SURFACE } else { ON_SURF_VAR };
        let text_rect = Rect::from_min_max(
            Pos2::new(icon_rect.right() + 12.0, rect.top()),
            Pos2::new(rect.right() - 8.0, rect.bottom()),
        );
        painter.text(
            text_rect.left_center(),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.0),
            text_color,
        );
    }

    resp
}

// ----------------------------------------------------------------------------
//  Декоративные элементы
// ----------------------------------------------------------------------------

/// Тонкая горизонтальная линия красного цвета.
#[allow(dead_code)]
pub fn accent_line(ui: &mut Ui, width_frac: f32) {
    let w = ui.available_width() * width_frac;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
    let painter = ui.painter();
    painter.line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0_f32, PRIMARY),
    );
}

/// Ряд маленьких красных точек (декоративный индикатор).
#[allow(dead_code)]
pub fn accent_dots(ui: &mut Ui, count: usize) {
    let dot_size = 3.0;
    let gap = 4.0;
    if count == 0 {
        return;
    }
    let total_w = count as f32 * dot_size + (count - 1) as f32 * gap;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, dot_size), Sense::hover());
    let painter = ui.painter();
    for i in 0..count {
        let cx = rect.left() + i as f32 * (dot_size + gap) + dot_size / 2.0;
        let cy = rect.center().y;
        painter.circle_filled(Pos2::new(cx, cy), dot_size / 2.0, PRIMARY);
    }
}

/// Красная точка-индикатор перед текстом (как статус «активно»).
#[allow(dead_code)]
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), Sense::hover());
    let painter = ui.painter();
    painter.circle_filled(rect.center(), 3.0, color);
}
