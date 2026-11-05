// ============================================================================
//  Snaply v0.4 — главный модуль
// ----------------------------------------------------------------------------
//  Что нового в v0.4:
//  • Боковая навигация (SidePanel::left): Главная / Настройки / О программе
//  • 3 страницы через enum ActivePage, переключаются без потери состояния
//  • Векторные иконки через Painter (модуль icons.rs) — никаких эмодзи
//  • Шрифт Inter (OFL) подключается из assets/fonts/Inter.ttf
//  • Опциональный фон: assets/background.png с тёмным оверлеем 75%
//  • Адаптивность: при ширине < 500px боковая панель сворачивается в иконки
//  • Главная кнопка с векторной иконкой камеры + стрелкой
//  • Реальные контролы: Switch, Segmented, KeyBadge, IconButton
//
//  ВАЖНО: вся функциональность сохранена:
//  • trigger_screenshot — тост сразу + поток захвата
//  • poll_external_events — трей, MENU_QUIT, перехват закрытия
//  • poll_key_rebind — переназначение хоткея
//  • show_window/hide_window — Minimized(true/false) + Focus + request_repaint
//  • pending_hide — отложенная минимизация после CancelClose
//  • Reactive repaint — 30 FPS тост / 4 FPS простой / 1 FPS трей
// ============================================================================

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod chopss;
mod config;
mod embedded;
mod hotkey;
mod icons;
mod material;
mod notification;
mod single_instance;
mod sound;
mod tray;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use tray_icon::{menu::MenuEvent, MouseButton, TrayIconEvent};

use crate::config::{vk_label, ImageFmt, Settings};
use crate::material::*;
use crate::notification::SharedNotif;

// ============================================================================
//  Разделяемое состояние — доступно и GUI, и фоновому потоку хоткеев
// ============================================================================
pub struct Shared {
    pub settings: Mutex<Settings>,
    pub status: Mutex<Option<(String, Instant)>>,
    pub notif: SharedNotif,
    pub ctx: Mutex<Option<egui::Context>>,
}

impl Shared {
    fn new(settings: Settings) -> Arc<Self> {
        Arc::new(Self {
            settings: Mutex::new(settings),
            status: Mutex::new(None),
            notif: notification::make_shared(),
            ctx: Mutex::new(None),
        })
    }

    pub fn set_status(&self, msg: String) {
        *self.status.lock().unwrap() = Some((msg, Instant::now()));
        self.wake_ui();
    }

    pub fn wake_ui(&self) {
        if let Some(ctx) = self.ctx.lock().unwrap().as_ref() {
            ctx.request_repaint();
        }
    }
}

// ============================================================================
//  Активная страница навигации
// ============================================================================
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Settings,
    About,
}

// ============================================================================
//  Состояние GUI
// ============================================================================
struct SnaplyApp {
    shared: Arc<Shared>,
    hotkey: hotkey::HotkeyHandle,
    _tray: Option<tray::TrayManager>,

    path_buf: String,
    listening_for_key: bool,
    is_hidden: bool,
    should_quit: bool,
    pending_hide: bool,

    /// Активная страница навигации
    active_page: Page,
    // settings_open убран — формат файла теперь на странице настроек,
    // раскрывающихся секций больше нет

    /// Загруженное фоновое изображение (если есть в assets/background.png)
    background_texture: Option<egui::TextureHandle>,
}

impl SnaplyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 1) Тема + шрифты
        apply_theme(&cc.egui_ctx);

        // 2) Грузим настройки
        let settings = Settings::load();
        let path_buf = settings.save_path.to_string_lossy().to_string();
        let shared = Shared::new(settings);

        // 3) Сохраняем egui::Context для wake_ui
        *shared.ctx.lock().unwrap() = Some(cc.egui_ctx.clone());

        // 4) Фоновый поток хоткеев
        let hotkey = hotkey::HotkeyHandle::spawn(shared.clone());

        // 5) Иконка в трее
        let _tray = tray::TrayManager::new().ok();

        // 6) Загружаем background.png опционально (если есть)
        let background_texture = load_background_texture(&cc.egui_ctx);

        Self {
            shared,
            hotkey,
            _tray,
            path_buf,
            listening_for_key: false,
            is_hidden: false,
            should_quit: false,
            pending_hide: false,
            active_page: Page::Home,
            background_texture,
        }
    }

    // ----------------------------------------------------------------------
    //  Триггер скриншота (без изменений — из v0.3)
    // ----------------------------------------------------------------------
    fn trigger_screenshot(&self) {
        let shared = self.shared.clone();
        let (path, notif, copy, fmt) = {
            let s = shared.settings.lock().unwrap();
            (
                s.save_path.clone(),
                s.show_notifications,
                s.auto_copy,
                s.format,
            )
        };

        // Тост показывается в capture::take_screenshot — после того как
        // файл сохранён, с РЕАЛЬНЫМ именем файла. Это лучше, чем показывать
        // «Подождите…» заранее: пользователь сразу видит имя файла.
        // Передаём notif=true, чтобы capture сам показал тост.
        std::thread::spawn(move || {
            match capture::take_screenshot(&path, notif, copy, fmt, &shared.notif) {
                Ok(fname) => shared.set_status(format!("✓ {}", fname)),
                Err(e) => shared.set_status(format!("✗ {}", e)),
            }
        });
    }

    // ----------------------------------------------------------------------
    //  Опрос внешних событий (без изменений — из v0.3)
    // ----------------------------------------------------------------------
    fn poll_external_events(&mut self, ctx: &egui::Context) {
        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            match ev.id.as_ref() {
                tray::MENU_SHOW => {
                    self.show_window(ctx);
                }
                tray::MENU_QUIT => {
                    // 1) Сообщаем что выходим — CancelClose не вызовется
                    self.should_quit = true;
                    // 2) Будим поток хоткеев (WM_QUIT) — иначе он не выйдет
                    //    из GetMessageW и процесс останется живым.
                    self.hotkey.shutdown();
                    // 3) Снимаем минимизацию и просим eframe закрыть окно
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    ctx.request_repaint();

                    // 4) ГАРАНТИЯ ВЫХОДА: иногда eframe не закрывает процесс
                    //    сразу (фоновый поток, winit event loop). Запускаем
                    //    таймер на 200ms — если процесс ещё жив, вызываем
                    //    std::process::exit(0). Это крайняя мера, но она
                    //    гарантирует что «Выход» всегда работает.
                    std::thread::spawn(|| {
                        std::thread::sleep(Duration::from_millis(200));
                        std::process::exit(0);
                    });
                }
                _ => {}
            }
        }

        if let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = ev
            {
                self.show_window(ctx);
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_hide = true;
            ctx.request_repaint();
        }
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        self.is_hidden = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn hide_window(&mut self, ctx: &egui::Context) {
        self.is_hidden = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    // ----------------------------------------------------------------------
    //  Слушаем клавишу для переназначения хоткея (без изменений)
    // ----------------------------------------------------------------------
    fn poll_key_rebind(&mut self, ctx: &egui::Context) {
        if !self.listening_for_key {
            return;
        }
        let new_vk = ctx.input(|i| {
            for ev in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    ..
                } = ev
                {
                    return egui_key_to_vk(*key);
                }
            }
            None
        });
        if let Some(vk) = new_vk {
            self.hotkey.reregister(vk);
            self.listening_for_key = false;
        }
    }

    // ======================================================================
    //  РЕНДЕР СТРАНИЦ
    // ======================================================================

    // ----------------------------------------------------------------------
    //  Страница «Главная» — кнопка скриншота, хоткей, папка, тумблеры, статус
    // ----------------------------------------------------------------------
    fn render_home_page(&mut self, ui: &mut egui::Ui) {
        // Шапка страницы — единый ритм с карточкой
        self.render_page_header(ui, "Главная", "Сделайте скриншот одним нажатием");

        // Карточка с основным контентом — единые внутренние отступы
        card(ui, |ui| {
            // ── Секция 1: главное действие ──
            if filled_button(ui, "Сделать скриншот", Some("camera")) {
                self.trigger_screenshot();
            }

            // ── Разделитель секций ──
            ui.add_space(16.0);
            self.render_divider(ui);
            ui.add_space(16.0);

            // ── Секция 2: папка сохранения ──
            // Хоткей убран с главной — он настраивается на странице «Настройки»
            self.render_folder_row(ui);

            // ── Разделитель секций ──
            ui.add_space(16.0);
            self.render_divider(ui);
            ui.add_space(16.0);

            // ── Секция 3: переключатели ──
            self.render_toggles(ui);

            // ── Статус (если есть) ──
            self.render_status(ui);
        });

        // Кнопка «Свернуть в трей» — выровнена по центру, под карточкой
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            if outlined_button_icon(ui, "Свернуть в трей", "chevron_down") {
                self.hide_window(ui.ctx());
            }
        });
    }

    // ----------------------------------------------------------------------
    //  Тихий разделитель — тонкая линия цвета OUTLINE, без отступов
    // ----------------------------------------------------------------------
    fn render_divider(&self, ui: &mut egui::Ui) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(w, 1.0), egui::Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            egui::Stroke::new(1.0_f32, OUTLINE),
        );
    }

    // ----------------------------------------------------------------------
    //  Страница «Настройки» — формат файла, хоткей
    // ----------------------------------------------------------------------
    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        self.render_page_header(ui, "Настройки", "Формат файла и горячая клавиша");

        card(ui, |ui| {
            // ── Секция 1: формат файла ──
            ui.label(
                egui::RichText::new("Формат файла")
                    .color(ON_SURFACE)
                    .font(egui::FontId::proportional(14.0))
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("PNG — без потерь, JPG — меньше размер, BMP — без сжатия")
                    .color(ON_SURF_VAR)
                    .font(egui::FontId::proportional(11.0)),
            );
            ui.add_space(10.0);

            let mut fmt = self.shared.settings.lock().unwrap().format;
            let options = [
                (ImageFmt::Png, "PNG"),
                (ImageFmt::Jpg, "JPG"),
                (ImageFmt::Bmp, "BMP"),
            ];
            if segmented(ui, &mut fmt, &options) {
                self.shared.settings.lock().unwrap().format = fmt;
                self.shared.settings.lock().unwrap().save();
            }

            // ── Разделитель секций ──
            ui.add_space(16.0);
            self.render_divider(ui);
            ui.add_space(16.0);

            // ── Секция 2: горячая клавиша ──
            ui.label(
                egui::RichText::new("Горячая клавиша")
                    .color(ON_SURFACE)
                    .font(egui::FontId::proportional(14.0))
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Клавиша для быстрого скриншота из любого приложения")
                    .color(ON_SURF_VAR)
                    .font(egui::FontId::proportional(11.0)),
            );
            ui.add_space(10.0);
            self.render_hotkey_row(ui);
        });
    }

    // ----------------------------------------------------------------------
    //  Страница «О программе»
    // ----------------------------------------------------------------------
    fn render_about_page(&self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        self.render_page_header(ui, "О программе", "Информация о приложении");

        card(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);

                // Логотип
                let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(72.0), egui::Sense::hover());
                crate::icons::logo(ui.painter(), rect, PRIMARY, SURFACE);

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Snaply")
                        .color(ON_SURFACE)
                        .font(egui::FontId::proportional(28.0))
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("версия 0.4.0")
                        .color(ON_SURF_VAR)
                        .font(egui::FontId::proportional(12.0)),
                );

                ui.add_space(16.0);

                // Тонкая разделительная линия
                let w = ui.available_width() * 0.6;
                let (line_rect, _) = ui.allocate_exact_size(egui::Vec2::new(w, 1.0), egui::Sense::hover());
                ui.painter().line_segment(
                    [line_rect.left_center(), line_rect.right_center()],
                    egui::Stroke::new(1.0_f32, OUTLINE),
                );

                ui.add_space(16.0);

                ui.label(
                    egui::RichText::new("Быстрые скриншоты, без лишнего.")
                        .color(ON_SURFACE)
                        .font(egui::FontId::proportional(14.0)),
                );

                ui.add_space(20.0);

                // Информация об авторе и лицензии
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Автор: ")
                            .color(ON_SURF_VAR)
                            .font(egui::FontId::proportional(12.0)),
                    );
                    ui.label(
                        egui::RichText::new("chateauroux")
                            .color(ACCENT)
                            .font(egui::FontId::proportional(12.0))
                            .strong(),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Лицензия: ")
                            .color(ON_SURF_VAR)
                            .font(egui::FontId::proportional(12.0)),
                    );
                    ui.label(
                        egui::RichText::new("MIT")
                            .color(ON_SURFACE)
                            .font(egui::FontId::proportional(12.0))
                            .strong(),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("GitHub: ")
                            .color(ON_SURF_VAR)
                            .font(egui::FontId::proportional(12.0)),
                    );
                    ui.label(
                        egui::RichText::new("github.com/chateauroux/snaply")
                            .color(ACCENT)
                            .font(egui::FontId::proportional(12.0)),
                    );
                });
                ui.add_space(16.0);

                ui.label(
                    egui::RichText::new("Шрифт Inter — SIL Open Font License 1.1")
                        .color(ON_SURF_VAR)
                        .font(egui::FontId::proportional(10.0)),
                );
            });
        });
    }

    // ----------------------------------------------------------------------
    //  Шапка страницы: крупный заголовок + подзаголовок + тонкая линия
    //  Единый ритм: 4px сверху → заголовок → 2px → подзаголовок → 8px → линия → 16px
    // ----------------------------------------------------------------------
    fn render_page_header(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .color(ON_SURFACE)
                .font(egui::FontId::proportional(22.0))
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(subtitle)
                .color(ON_SURF_VAR)
                .font(egui::FontId::proportional(12.0)),
        );
        ui.add_space(8.0);
        // Тихая разделительная линия (не ярко-красная, как раньше)
        self.render_divider(ui);
        ui.add_space(16.0);
    }

    // ----------------------------------------------------------------------
    //  Строка «Горячая клавиша» — подпись слева, бейдж + карандаш справа
    // ----------------------------------------------------------------------
    fn render_hotkey_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Спокойная подпись без декоративной красной точки
            ui.label(
                egui::RichText::new("Горячая клавиша")
                    .color(ON_SURF_VAR)
                    .font(egui::FontId::proportional(13.0)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Иконка-карандаш (векторная)
                if icon_button(ui, "pencil", 32.0).clicked() {
                    self.listening_for_key = true;
                    self.shared.set_status("Нажмите любую клавишу…".into());
                }

                ui.add_space(8.0);

                // Бейдж с текущей клавишей
                let vk = self.shared.settings.lock().unwrap().hotkey;
                let label = if self.listening_for_key {
                    "…".to_string()
                } else {
                    vk_label(vk)
                };
                key_badge(ui, &label);
            });
        });
    }

    // ----------------------------------------------------------------------
    //  Строка «Папка сохранения» — подпись + поле пути с обрезкой + Обзор
    // ----------------------------------------------------------------------
    fn render_folder_row(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Папка сохранения")
                .color(ON_SURF_VAR)
                .font(egui::FontId::proportional(13.0)),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            // Поле пути — адаптивно занимает доступную ширину.
            // clip_text(true) обрезает длинный путь с многоточием.
            // При наведении показываем полный путь через tooltip.
            let field_w = (ui.available_width() - 100.0).max(120.0);
            let resp = ui.add_sized(
                [field_w, 34.0],
                egui::TextEdit::singleline(&mut self.path_buf)
                    .font(egui::FontId::proportional(12.0))
                    .text_color(ON_SURFACE)
                    .clip_text(true),
            );
            // on_hover_text забирает resp по значению и возвращает Response.
            // Поэтому сначала вызываем changed(), потом навешиваем tooltip.
            if resp.changed() {
                let mut s = self.shared.settings.lock().unwrap();
                s.save_path = std::path::PathBuf::from(&self.path_buf);
                s.save();
            }
            // Tooltip с полным путём — показывается при наведении
            resp.on_hover_text(&self.path_buf);

            // Кнопка «Обзор» с векторной иконкой папки
            if outlined_button_icon(ui, "Обзор", "folder") {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    self.path_buf = p.to_string_lossy().to_string();
                    let mut s = self.shared.settings.lock().unwrap();
                    s.save_path = p;
                    s.save();
                }
            }
        });
    }

    // ----------------------------------------------------------------------
    //  Тумблеры — уведомление + автокопирование.
    //  Ровное вертикальное выравнивание: switch слева, подпись справа.
    // ----------------------------------------------------------------------
    fn render_toggles(&mut self, ui: &mut egui::Ui) {
        // Уведомление
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0);
            let mut notif = self.shared.settings.lock().unwrap().show_notifications;
            let resp = switch(ui, &mut notif, "t_notif");
            if resp.clicked() {
                self.shared.settings.lock().unwrap().show_notifications = notif;
                self.shared.settings.lock().unwrap().save();
            }
            // Подпись выровнена по центру по вертикали с switch
            ui.label(
                egui::RichText::new("Показывать уведомление")
                    .color(ON_SURFACE)
                    .font(egui::FontId::proportional(13.0)),
            );
        });
        ui.add_space(10.0);

        // Авто-копирование
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0);
            let mut copy = self.shared.settings.lock().unwrap().auto_copy;
            let resp = switch(ui, &mut copy, "t_copy");
            if resp.clicked() {
                self.shared.settings.lock().unwrap().auto_copy = copy;
                self.shared.settings.lock().unwrap().save();
            }
            ui.label(
                egui::RichText::new("Копировать в буфер обмена")
                    .color(ON_SURFACE)
                    .font(egui::FontId::proportional(13.0)),
            );
        });
    }

    // ----------------------------------------------------------------------
    //  Строка состояния (исчезает через 4 сек)
    // ----------------------------------------------------------------------
    fn render_status(&mut self, ui: &mut egui::Ui) {
        let mut to_clear = false;
        if let Some((msg, t0)) = self.shared.status.lock().unwrap().as_ref() {
            if t0.elapsed().as_secs_f64() > 4.0 {
                to_clear = true;
            } else {
                let color = if msg.starts_with('✓') {
                    SUCCESS
                } else if msg.starts_with('✗') {
                    ERROR
                } else {
                    ON_SURF_VAR
                };
                ui.label(
                    egui::RichText::new(msg)
                        .color(color)
                        .font(egui::FontId::proportional(11.0)),
                );
            }
        }
        if to_clear {
            *self.shared.status.lock().unwrap() = None;
        }
    }
}

// ============================================================================
//  Реализация трейта eframe::App
// ============================================================================
impl eframe::App for SnaplyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0) Отложенная минимизация
        if self.pending_hide {
            self.pending_hide = false;
            self.hide_window(ctx);
        }

        // 1) Внешние события
        self.poll_external_events(ctx);

        // 2) Переназначение хоткея
        self.poll_key_rebind(ctx);

        // 3) Тост-оверлей
        let has_notif = self.shared.notif.lock().unwrap().is_some();
        if has_notif {
            notification::render_overlay(ctx, &self.shared.notif);
        }

        // 4) Адаптивность: при узком окне боковая панель сворачивается в иконки
        let screen_w = ctx.screen_rect().width();
        let collapsed = screen_w < 500.0;
        let side_panel_w = if collapsed { 56.0 } else { 160.0 };

        // 5) Боковая навигация
        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .exact_width(side_panel_w)
            .frame(
                egui::Frame::none()
                    .fill(SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, OUTLINE))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // Логотип в шапке боковой панели
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::Vec2::splat(if collapsed { 36.0 } else { 32.0 }),
                        egui::Sense::hover(),
                    );
                    crate::icons::logo(ui.painter(), rect, PRIMARY, SURFACE);
                    if !collapsed {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Snaply")
                                .color(ON_SURFACE)
                                .font(egui::FontId::proportional(16.0))
                                .strong(),
                        );
                    }
                });

                ui.add_space(20.0);

                // Пункты навигации
                if nav_button(ui, "home", "Главная", self.active_page == Page::Home, collapsed).clicked() {
                    self.active_page = Page::Home;
                }
                ui.add_space(4.0);
                if nav_button(ui, "gear", "Настройки", self.active_page == Page::Settings, collapsed).clicked() {
                    self.active_page = Page::Settings;
                }
                ui.add_space(4.0);
                if nav_button(ui, "info", "О программе", self.active_page == Page::About, collapsed).clicked() {
                    self.active_page = Page::About;
                }

                // Внизу — кнопка ChopSS + индикатор статуса
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    if collapsed {
                        // Просто точка-индикатор
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(8.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 4.0, SUCCESS);
                    } else {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(8.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, SUCCESS);
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Активно")
                                    .color(ON_SURF_VAR)
                                    .font(egui::FontId::proportional(11.0)),
                            );
                        });
                    }

                    // ── ChopSS — отдельная утилита нарезки скриншотов ──
                    // Открывается в отдельном окне через WebView (wry+tao).
                    ui.add_space(8.0);
                    let chopss_resp = nav_button(
                        ui,
                        "camera",
                        "ChopSS",
                        false, // не активная страница — отдельное окно
                        collapsed,
                    );
                    // Сначала проверяем клик, потом навешиваем tooltip
                    // (on_hover_text забирает resp по значению)
                    if chopss_resp.clicked() {
                        // Открываем ChopSS в системном браузере.
                        // Если файл не найден — пишем в статусную строку.
                        if let Err(e) = chopss::open() {
                            self.shared.set_status(format!("✗ ChopSS: {}", e));
                        }
                    }
                    // Tooltip с описанием при наведении
                    chopss_resp.on_hover_text(
                        "Данная утилита позволяет быстро собрать скриншот-ситуацию \
                         методом нарезки. Просто вставляйте в инпут необходимые \
                         скриншоты, а по окончании забирайте файл в буфер обмена \
                         или в папку вывода",
                    );
                });
            });

        // 6) Основная область с фоном
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(16.0)))
            .show(ctx, |ui| {
                // Сначала рисуем фон (градиент или background.png с оверлеем)
                paint_background(ui, self.background_texture.as_ref());

                // Затем контент поверх фона
                match self.active_page {
                    Page::Home => self.render_home_page(ui),
                    Page::Settings => self.render_settings_page(ui),
                    Page::About => self.render_about_page(ui),
                }
            });

        // 7) Reactive repaint (без изменений — из v0.3)
        let has_notif = self.shared.notif.lock().unwrap().is_some();
        let has_status = self.shared.status.lock().unwrap().is_some();

        let delay = if has_notif {
            Duration::from_millis(33)
        } else if self.listening_for_key {
            Duration::from_millis(33)
        } else if has_status {
            Duration::from_millis(250)
        } else if self.is_hidden {
            Duration::from_millis(250)
        } else {
            Duration::from_millis(250)
        };
        ctx.request_repaint_after(delay);
    }
}

// ============================================================================
//  Фон: background.png с тёмным оверлеем, или градиент
// ============================================================================
fn paint_background(ui: &mut egui::Ui, bg_texture: Option<&egui::TextureHandle>) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    if let Some(tex) = bg_texture {
        // Растягиваем background.png на весь экран, СОХРАНЯЯ пропорции
        // (cover-fit: масштабируем так, чтобы заполнить экран без искажений).
        let img_size = tex.size_vec2();
        let screen_size = rect.size();
        let scale_x = screen_size.x / img_size.x;
        let scale_y = screen_size.y / img_size.y;
        let scale = scale_x.max(scale_y); // cover-fit
        let scaled = img_size * scale;
        let img_rect = egui::Rect::from_center_size(rect.center(), scaled);
        painter.image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        // Тёмный оверлей 85% непрозрачности — фон не спорит с карточкой и текстом.
        // Раньше было 200/255 (~78%) — слишком прозрачно. Теперь 217/255 (~85%).
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_premultiplied(8, 6, 8, 217),
        );
    } else {
        // Градиент сверху вниз. В egui 0.29 Vertex находится в epaint,
        // который переэкспортируется через egui::epaint.
        let top = egui::Color32::from_rgb(12, 8, 10);
        let bot = egui::Color32::from_rgb(22, 10, 12);
        let mesh = egui::Mesh {
            indices: vec![0, 1, 2, 0, 2, 3],
            vertices: vec![
                egui::epaint::Vertex { pos: rect.left_top(), uv: egui::Pos2::ZERO, color: top },
                egui::epaint::Vertex { pos: rect.right_top(), uv: egui::Pos2::ZERO, color: top },
                egui::epaint::Vertex { pos: rect.right_bottom(), uv: egui::Pos2::ZERO, color: bot },
                egui::epaint::Vertex { pos: rect.left_bottom(), uv: egui::Pos2::ZERO, color: bot },
            ],
            texture_id: Default::default(),
        };
        painter.add(mesh);
    }
}

// ============================================================================
//  Загрузка background.png (опционально)
// ============================================================================
fn load_background_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    // background.png встроен в бинарник. Если embed пустой (0 байт — заглушка
    // созданная build.rs) — возвращаем None, будет градиент.
    let bytes = crate::embedded::get_background_png()?;
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();

    let color_image = egui::ColorImage {
        size,
        pixels: pixels
            .chunks_exact(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect(),
    };

    Some(ctx.load_texture("background", color_image, egui::TextureOptions::LINEAR))
}

// ============================================================================
//  Конвертация egui::Key → Win32 VK-код
// ============================================================================
fn egui_key_to_vk(key: egui::Key) -> Option<u32> {
    use egui::Key::*;
    Some(match key {
        F1 => 0x70, F2 => 0x71, F3 => 0x72, F4 => 0x73, F5 => 0x74, F6 => 0x75,
        F7 => 0x76, F8 => 0x77, F9 => 0x78, F10 => 0x79, F11 => 0x7A, F12 => 0x7B,
        A => 0x41, B => 0x42, C => 0x43, D => 0x44, E => 0x45, F => 0x46,
        G => 0x47, H => 0x48, I => 0x49, J => 0x4A, K => 0x4B, L => 0x4C,
        M => 0x4D, N => 0x4E, O => 0x4F, P => 0x50, Q => 0x51, R => 0x52,
        S => 0x53, T => 0x54, U => 0x55, V => 0x56, W => 0x57, X => 0x58,
        Y => 0x59, Z => 0x5A,
        Num0 => 0x30, Num1 => 0x31, Num2 => 0x32, Num3 => 0x33, Num4 => 0x34,
        Num5 => 0x35, Num6 => 0x36, Num7 => 0x37, Num8 => 0x38, Num9 => 0x39,
        Space => 0x20,
        Escape => 0x1B,
        _ => return None,
    })
}

// ============================================================================
//  Иконка окна + точка входа
// ============================================================================
fn load_window_icon() -> egui::IconData {
    if let Some(rgba) = load_icon_rgba() {
        return egui::IconData {
            rgba,
            width: 32,
            height: 32,
        };
    }
    egui::IconData {
        rgba: vec![0; 32 * 32 * 4],
        width: 32,
        height: 32,
    }
}

fn load_icon_rgba() -> Option<Vec<u8>> {
    // Иконка встроена в бинарник (embedded::ICON_ICO).
    let bytes = crate::embedded::ICON_ICO;
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    if rgba.width() == 32 && rgba.height() == 32 {
        Some(rgba.into_raw())
    } else {
        use image::imageops::FilterType;
        let resized = image::imageops::resize(&rgba, 32, 32, FilterType::Lanczos3);
        Some(resized.into_raw())
    }
}

fn find_icon_file() -> Option<std::path::PathBuf> {
    // Больше не нужно — иконка встроена в бинарник.
    // Оставлено для обратной совместимости, всегда возвращает None.
    None
}

fn main() -> eframe::Result<()> {
    let _instance_guard = match single_instance::check() {
        Some(g) => g,
        None => return Ok(()),
    };

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Snaply")
            .with_inner_size([720.0, 640.0])
            .with_min_inner_size([480.0, 480.0])
            .with_decorations(true)
            .with_active(true)
            .with_icon(load_window_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Snaply",
        opts,
        Box::new(|cc| Ok(Box::new(SnaplyApp::new(cc)))),
    )
}
