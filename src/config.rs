// ============================================================================
//  config.rs — настройки приложения и их сохранение/загрузка
// ----------------------------------------------------------------------------
//  Формат файла snaply_config.txt — простой key=value, по одной настройке
//  на строку. Никакого serde/JSON ради экономии зависимостей.
// ============================================================================

use std::path::PathBuf;

// ----------------------------------------------------------------------------
//  Поддерживаемые форматы изображения
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageFmt {
    Png,
    Jpg,
    Bmp,
}

impl ImageFmt {
    pub fn ext(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Bmp => "bmp",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpg => "JPG",
            Self::Bmp => "BMP",
        }
    }

    pub fn image_format(&self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Jpg => image::ImageFormat::Jpeg,
            Self::Bmp => image::ImageFormat::Bmp,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "JPG" | "JPEG" => Self::Jpg,
            "BMP" => Self::Bmp,
            _ => Self::Png,
        }
    }
}

// ----------------------------------------------------------------------------
//  Настройки приложения
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct Settings {
    /// Куда складывать скриншоты.
    pub save_path: PathBuf,
    /// VK-код клавиши (0x78 = F9). См. Win32 Virtual-Key Codes.
    pub hotkey: u32,
    /// Показывать ли нативный тост при скриншоте.
    pub show_notifications: bool,
    /// Автоматически копировать снимок в буфер обмена.
    pub auto_copy: bool,
    /// Формат файла по умолчанию.
    pub format: ImageFmt,
}

impl Default for Settings {
    fn default() -> Self {
        let default_path = dirs::picture_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("FiveM_Screens");
        Self {
            save_path: default_path,
            hotkey: 0x78, // F9
            show_notifications: true,
            auto_copy: false,
            format: ImageFmt::Png,
        }
    }
}

const CONFIG_FILE: &str = "snaply_config.txt";

impl Settings {
    /// Загрузить настройки из файла рядом с exe. Если файла нет — дефолт.
    pub fn load() -> Self {
        let path = std::path::Path::new(CONFIG_FILE);
        if !path.exists() {
            return Self::default();
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        let mut s = Self::default();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                match k {
                    "save_path" => s.save_path = PathBuf::from(v),
                    "hotkey" => {
                        // Поддерживаем 0x.. и десятичный формат
                        if let Ok(code) = u32::from_str_radix(v.trim_start_matches("0x"), 16) {
                            s.hotkey = code;
                        } else if let Ok(code) = v.parse::<u32>() {
                            s.hotkey = code;
                        }
                    }
                    "show_notifications" => s.show_notifications = v == "true",
                    "auto_copy" => s.auto_copy = v == "true",
                    "format" => s.format = ImageFmt::from_str(v),
                    _ => {}
                }
            }
        }
        s
    }

    /// Сохранить настройки в файл.
    pub fn save(&self) {
        let content = format!(
            "# Snaply configuration\n\
             save_path={}\n\
             hotkey=0x{:02X}\n\
             show_notifications={}\n\
             auto_copy={}\n\
             format={}\n",
            self.save_path.display(),
            self.hotkey,
            self.show_notifications,
            self.auto_copy,
            self.format.label()
        );
        let _ = std::fs::write(CONFIG_FILE, content);
    }
}

// ----------------------------------------------------------------------------
//  Человекочитаемая подпись VK-кода для UI
// ----------------------------------------------------------------------------
pub fn vk_label(vk: u32) -> String {
    // F1–F12: 0x70–0x7B
    if (0x70..=0x7B).contains(&vk) {
        return format!("F{}", vk - 0x70 + 1);
    }
    // A–Z: 0x41–0x5A
    if (0x41..=0x5A).contains(&vk) {
        return char::from_u32(vk).map(|c| c.to_string()).unwrap_or_default();
    }
    // 0–9: 0x30–0x39
    if (0x30..=0x39).contains(&vk) {
        return char::from_u32(vk).map(|c| c.to_string()).unwrap_or_default();
    }
    match vk {
        0x20 => "Space".into(),
        0x2C => "PrtScn".into(),
        0x1B => "Esc".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        _ => format!("0x{:02X}", vk),
    }
}
