Snaply — assets
================

📁 Структура:

assets/
├── fonts/
│   ├── Inter.ttf              — переменный шрифт Inter (OFL 1.1)
│   ├── Inter-Bold.ttf         — копия Inter.ttf (для совместимости)
│   └── LICENSE.txt            — SIL Open Font License 1.1
├── snaply.ico                 — иконка приложения (трей, заголовок, taskbar)
├── notification.wav           — звук при показе тоста (опционально)
└── background.png             — фон главного окна (опционально)

Где программа ищет файлы (в порядке приоритета):
  1. ./assets/<file>                    ← для разработки (cargo run)
  2. ./<file>
  3. ./target/debug/assets/<file>
  4. ./target/release/assets/<file>
  5. рядом с .exe/assets/<file>          ← для финальной поставки
  6. рядом с .exe/<file>

Если файл не найден — используется фолбэк:
  • Inter.ttf: системный шрифт egui (Cantarell/Ubuntu), кириллица может
    отображаться квадратиками без Inter
  • snaply.ico: генерируемый бордовый круг 32×32
  • notification.wav: системный MessageBeep
  • background.png: тёмный градиент #0C080A → #160A0C

Требования к файлам:
  • Inter.ttf: TrueType Font, переменный или обычный (Regular)
  • snaply.ico: формат .ico, желательно с размерами 16/32/48/256
  • notification.wav: WAV 16-bit 44.1kHz, длительность 0.3-1.0 сек
  • background.png: PNG, любой размер (растягивается на весь экран),
    рекомендуется тёмная картинка — поверх накладывается оверлей 80%

Лицензии:
  • Inter — SIL Open Font License 1.1 (можно коммерческое использование)
  • Snaply — MIT
