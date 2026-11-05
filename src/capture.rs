// ============================================================================
//  capture.rs — захват экрана и сохранение скриншота
// ----------------------------------------------------------------------------
//  Захват: классический GDI-метод — GetDC(NULL) → CreateCompatibleDC →
//  CreateCompatibleBitmap → BitBlt → GetDIBits с явным 32bpp BGRA top-down
//  форматом. Это надёжнее, чем GetBitmapBits (формат которого зависит от
//  битмапа и может быть 24bpp без альфы).
//
//  Работает в Borderless/Windowed Fullscreen. Для эксклюзивного Fullscreen
//  (DX/Vulkan exclusive mode) может дать чёрный кадр — тогда нужен DXGI
//  Desktop Duplication (crate `cap`).
//
//  ВАЖНО: эта функция запускается ИСКЛЮЧИТЕЛЬНО в фоновом потоке через
//  std::thread::spawn (см. hotkey.rs и main.rs::trigger_screenshot).
//  UI-поток ничего не знает про кодирование PNG — иначе фриз.
//
//  PNG-сжатие: используем CompressionType::Fast + FilterType::Sub вместо
//  дефолтного Best/Adaptive. Это даёт ~5x ускорение кодирования при
//  незначительном (+15-20%) увеличении размера файла. Критично для UI:
//  дефолтное сжатие грузит одно ядро на 100% по 200-400ms на 4K.
// ============================================================================

use chrono::Local;
use image::RgbaImage;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

use crate::config::ImageFmt;
use crate::notification::SharedNotif;

// ----------------------------------------------------------------------------
//  Захватить весь основной монитор → RgbaImage
// ----------------------------------------------------------------------------
pub fn capture_screen() -> Option<RgbaImage> {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w <= 0 || h <= 0 {
            return None;
        }

        let screen_dc = GetDC(ptr::null_mut());
        if screen_dc.is_null() {
            return None;
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(ptr::null_mut(), screen_dc);
            return None;
        }
        let bmp = CreateCompatibleBitmap(screen_dc, w, h);
        if bmp.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            return None;
        }

        let old_obj = SelectObject(mem_dc, bmp as HGDIOBJ);

        if BitBlt(mem_dc, 0, 0, w, h, screen_dc, 0, 0, SRCCOPY) == 0 {
            SelectObject(mem_dc, old_obj);
            DeleteObject(bmp as HGDIOBJ);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            return None;
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -h;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;

        let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
        let got = GetDIBits(
            mem_dc,
            bmp,
            0,
            h as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bi,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old_obj);
        DeleteObject(bmp as HGDIOBJ);
        DeleteDC(mem_dc);
        ReleaseDC(ptr::null_mut(), screen_dc);

        if got == 0 {
            return None;
        }

        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }

        RgbaImage::from_raw(w as u32, h as u32, pixels)
    }
}

// ----------------------------------------------------------------------------
//  Сохранить RgbaImage в файл с быстрым сжатием.
//  PNG: CompressionType::Fast + FilterType::Sub (5x быстрее дефолта).
//  JPG: конвертация RGBA → RGB (JPG не поддерживает альфу) + JpegEncoder.
//  BMP: BmpEncoder напрямую в файл.
// ----------------------------------------------------------------------------
fn save_image_fast(img: &RgbaImage, path: &Path, fmt: ImageFmt) -> Result<(), String> {
    use image::ImageEncoder;
    use std::io::BufWriter;

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);
    let w = img.width();
    let h = img.height();
    let pixels = img.as_raw();

    match fmt {
        ImageFmt::Png => {
            // БЫСТРОЕ сжатие PNG — критично для 4K, иначе 200-400ms фриза
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            let encoder = PngEncoder::new_with_quality(
                &mut writer,
                CompressionType::Fast,
                FilterType::Sub,
            );
            encoder
                .write_image(pixels.as_slice(), w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())
        }
        ImageFmt::Jpg => {
            // JPG НЕ ПОДДЕРЖИВАЕТ АЛЬФА-КАНАЛ. Конвертируем RGBA → RGB.
            // Раньше image::save_with_format для RGBA+Jpeg записывал битый
            // файл. Делаем явно через JpegEncoder с качеством 85.
            use image::codecs::jpeg::JpegEncoder;
            let rgb: Vec<u8> = pixels
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            let encoder = JpegEncoder::new_with_quality(&mut writer, 85);
            encoder
                .write_image(rgb.as_slice(), w, h, image::ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())
        }
        ImageFmt::Bmp => {
            // BMP через BmpEncoder напрямую в BufWriter
            use image::codecs::bmp::BmpEncoder;
            let encoder = BmpEncoder::new(&mut writer);
            encoder
                .write_image(pixels.as_slice(), w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())
        }
    }
}

// ----------------------------------------------------------------------------
//  Полный цикл: захват → сохранение → буфер обмена → тост
//  Возвращает имя файла или текст ошибки.
//  ЭТУ ФУНКЦИЮ ВЫЗЫВАТЬ ТОЛЬКО ИЗ ФОНОВОГО ПОТОКА.
// ----------------------------------------------------------------------------
pub fn take_screenshot(
    folder: &Path,
    notif: bool,
    copy: bool,
    fmt: ImageFmt,
    notif_shared: &SharedNotif,
) -> Result<String, String> {
    // 1) Захват экрана
    let img = capture_screen().ok_or_else(|| "Не удалось захватить экран".to_string())?;

    // 2) Папка существует?
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;

    // 3) Имя файла Screen_YYYY-MM-DD_HH-MM-SS.ext
    let ts = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let fname = format!("Screen_{}.{}", ts, fmt.ext());
    let path = folder.join(&fname);

    // 4) Сохраняем с БЫСТРЫМ сжатием (для PNG)
    save_image_fast(&img, &path, fmt)?;

    // 5) Копируем в буфер обмена, если включено
    if copy {
        use arboard::ImageData;
        use std::borrow::Cow;

        let w = img.width() as usize;
        let h = img.height() as usize;
        let pixels = img.as_raw().clone();
        let clip = ImageData {
            width: w,
            height: h,
            bytes: Cow::from(pixels),
        };
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_image(clip);
        }
    }

    // 6) Кастомный тост-оверлей (только если вызывающий не показал сам).
    //    Заголовок — «СКРИНШОТ СОХРАНЁН», тело — имя файла.
    if notif {
        crate::notification::show(
            notif_shared,
            "СКРИНШОТ СОХРАНЁН",
            &fname,
        );
    }

    Ok(fname)
}
