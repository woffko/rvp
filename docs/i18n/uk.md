# Ukrainian (uk)

Modern standard Ukrainian, spelled by the orthography of 2019 (проєкт),
in the plain, neutral register of current Ukrainian desktop software. The
user is addressed politely in the plural (ви), as Windows, GNOME and KDE
address their Ukrainian users, but most strings need no address at all:
commands are infinitives, options are infinitives or noun phrases, and
help texts say what RVP does. Every string is translated from the English,
in the words a Ukrainian user of a media player knows (список відтворення,
субтитри, скін, еквалайзер, перемотування). The Russian translation shows
meaning only: Ukrainian never follows its words or its constructions, and
avoids calques and Russianisms (see "Words that trap"). Names, codecs, file
names and keys stay as README lists them.

## Style

- **Address.** Instructions use the imperative of the polite plural:
  Виберіть дію, потім додайте клавішу або жест миші. Натисніть клавішу
  для «%1» (%2). Esc — скасувати. Leave out the pronouns (ви, ваш,
  свій) wherever the sentence allows; where one is needed it is
  lowercase (ви, ваш), never the capitalized Ви of letters. Never the
  familiar ти forms, and no будь ласка in labels and short prompts. The
  plural past tense shows no gender (ви завантажили), so the user's
  gender never shows; still prefer the present tense (файли, які ви
  завантажуєте) or a participle (завантажені моделі).
- **Help texts** (sentences under a page's options): present tense,
  third person, with RVP, mpv or the feature as the subject: RVP
  вимірює..., Калібрування триває від однієї до трьох хвилин. Reports of
  what happened use the impersonal -но/-то form, which agrees with
  nothing: Застосовано. Імпортовано. Експортовано. Скасовано. Без змін.
  Нічого не знайдено. Вилучено: %1.
- **Commands** (menu items, buttons, actions): the perfective
  infinitive, as Ukrainian desktops write their commands: Відкрити
  теку..., Зберегти, Додати закладку, Видалити, Перейти до часу/кадру...,
  Закрити. The desktops' own words stay as users know them: Копіювати,
  Вставити, Вирізати, Скасувати (both Undo and Cancel), Повторити (Redo),
  Виділити все, Зняти виділення, Застосувати, Огляд… (Browse…), Вийти
  (Exit). "Cycle X" (each press takes the next mode) is Перемкнути X.
  Where the English label is a noun (Playback, Subtitles, Audio
  Recording), Ukrainian uses a noun (Відтворення, Субтитри, Запис звуку).
- **Checkboxes and options for standing behaviour:** the imperfective
  infinitive, which says "every time": Запам’ятовувати позиції
  відтворення, Показувати підказки, Призупиняти відтворення під час
  згортання, Додавати схожі файли, Пропускати вступ, Блокувати заставку
  під час відтворення. "Enable X" stays perfective: Увімкнути пропуск,
  Увімкнути XInput. A checkbox whose English is a noun phrase may stay
  one.
- **Choices** (radio buttons, list items, menu values): a noun phrase,
  an adjective that agrees with the label's noun, or an adverb, which
  never changes: Ніколи, Завжди, Автоматично, Під час відтворення,
  Випадково. A state with no noun in sight takes the impersonal form,
  which needs no agreement: Увімкнено, Вимкнено, Приховано. "Default"
  standing alone is Типово. "(Recommended)" is always "(рекомендовано)"
  and "(Default)" "(типово)": PCM (типово) (рекомендовано).
- **Titles** of windows, pages, groups and columns are noun phrases:
  Відтворення, Повноекранний режим, Тривалість, Налаштування
  візуалізації. A window that a command opens keeps the command's words
  without the ellipsis, as the source does: Перейти до часу/кадру.
- **Questions** in dialogs take the infinitive: Перемістити файли (%1)
  до Кошика? Повернути типові значення параметрів сторінки «%1»? The
  answer buttons are infinitives too: Перемістити, Залишити.
- **Capitalization:** sentence case everywhere, menus, buttons and window
  titles included: only the first word and names take a capital
  (Налаштування відтворення..., Показувати мініатюри на шкалі
  відтворення). The English Title Case is never copied. After a slash
  inside a label the next word is lowercase (Відтворити/призупинити,
  Розширення/обрізання), and after a colon too (Калібрування:
  скасовано; Дія: глибокий сон), names apart. Languages, days and months
  are lowercase (англійська, українська, as in the whisper language
  list); the interface language's own name, a label of its own, is
  Українська. Кошик, the name of Windows' Recycle Bin, keeps its capital.
- **Quotation marks:** the Ukrainian «…» (U+00AB, U+00BB) for quoted
  labels and option names, „…“ (U+201E, U+201C) inside them; the
  source's straight and curly quotes both become «…». Help texts quote
  buttons and options with their Ukrainian label exactly: «Застосувати»,
  «Підганяти вікно під щойно відкрите відео». Menu paths stay unquoted,
  with the Ukrainian labels and the source's ">": Відео > Нижнє поле.
- **Apostrophe:** always the typographic ’ (U+2019): пам’ять,
  комп’ютер, об’ємний, з’являється, пом’якшення. Never the straight '
  (U+0027) or ʼ (U+02BC), and never left out.
- **Punctuation:** a comma before every subordinate clause (що, коли,
  якщо, щоб, який, бо, чи, поки) and around an adverbial participle
  phrase (-ючи, -вши); none before і, й, та, або in a simple list. A
  spaced em dash " — " where the English has " - " or a pause
  (%1/%2/%3 — %4 стерео), and where Ukrainian leaves out "is" between
  two nouns (X1 — кнопка «Назад»); an unspaced en dash in ranges
  (0–255, 16–235); a hyphen in compounds and ordinals (10-бітний,
  %1-й). Abbreviations: напр. (e.g.), тощо (etc.), макс., мін.
  (minimum: minutes are хв, so there is no clash); "i.e." is written out
  as тобто. "No." and "#" before a number are № with a space: "Style
  #%1" is "Стиль № %1".
- **The ellipsis** copies the source's form: "..." where it has three
  periods, "…" where it has the one character, with no space before it
  (Відкрити файл...), except where the source has one (Shutdown at ...:
  Вимкнути комп’ютер о ...). Never add or drop one.
- **Units:** a space between a number and its unit, also where the
  English writes them together: %1 с, %1 мс, %1 хв, %1 год, %1 Гц,
  %1 кГц, %1 дБ, %1 МБ, %1 ГБ, %1 Вт, 60 кадр/с (%1ms: %1 мс, %1Hz:
  %1 Гц). The symbols take no period: seconds с (never сек.), minutes
  хв, hours год; the source's Sec., Sec and Min become с and хв, and
  "frames a second" in a label is кадр/с. Unit symbols are not inflected,
  which makes them the safe choice after a placeholder: Перемотати на
  %1 с назад, never %1 секунд. %, x (2x, %1x%2), K, p (1080p), px and °
  stay exactly as the source writes them, with or without its space.
- **Numbers:** a decimal comma in numbers the translation writes itself
  (1,5 с, 0,5x); the period stays where it is part of a name or of what
  the user types: versions, the channel layouts 5.1 and 7.1, codec
  names, the times typed into a field (г:хх:сс.с). Thousands take a
  space from five digits (10 000), none in four (1000 мс, 2160p).
  Ordinals are the digit, a hyphen and the ending, which depends on the
  number in every form but the masculine nominative (1-ша, 2-ге, 3-тя):
  with a placeholder, use the masculine nominative -й, which fits every
  number, and pick a masculine noun: "%1nd Device" is "%1-й пристрій",
  "{Same as %1st logo}" is "{Як %1-й логотип}". Time-format hints use
  Ukrainian letters: h:mm:ss.s is г:хх:сс.с, hh:mm is гг:хх, h:mm is
  г:хх. Numbers that come through placeholders are RVP's and stay as
  they come.
- **Counts:** Ukrainian has three forms after a number (1 файл, 2–4
  файли, 5–20 файлів, 21 файл, 22 файли, 25 файлів, and 11–14 файлів)
  and the strings have no plural forms, so a noun never follows %1–%9
  directly unless its number is known. Instead: the noun in the genitive
  plural before a colon (Файлів: %1, Знайдено скриптів: %2); the count
  in parentheses after a plural noun (Перемістити файли (%1) до
  Кошика?); a unit symbol (с, хв, ГБ; the source's chs has none in
  Ukrainian: каналів: щонайменше %1); or the noun before the number
  where that reads well (рядів: до %1). Where a pair of strings gives
  the singular and the plural and the page shows which numbers they take
  (Reserve %1 line, Reserve %1 lines: 1, then 2 and 3), use those
  numbers' form: Резервувати %1 рядок, Резервувати %1 рядки. A verb
  after a count becomes impersonal: "%1 removed" is "Вилучено: %1",
  "%1 done, at most %2 left" is "Виконано: %1, залишилося не більше
  %2".
- **Gender and agreement:** adjectives and participles agree with their
  noun (типовий розмір, власна висота, увімкнена анімація). "Custom" is
  власний, власна or власне by its noun; standing alone it is Власний
  (a variant, a size), never Власне, which reads as "actually". RVP, mpv,
  SVP and the other programs need no gender in the present tense and in
  the impersonal forms (RVP встановлено); where one shows, they are
  masculine, like програвач. The user's gender never shows (see
  Address).
- **Names are not inflected.** A name follows a noun as it is (скіни
  PotPlayer, рендерер mpv, фільтр SVP, відеокарти NVIDIA, бібліотека
  FFmpeg), and prepositions take it directly, as Ukrainian does with
  foreign names it does not decline: у Windows, для Linux, у GNOME, з
  PotPlayer. Never PotPlayer’а, Windows’і. "RVP's own X" is "власний X
  RVP".
- **Euphony.** у/в, і/й and з/із/зі alternate by the sounds around them:
  аудіо й відео, звук і відео, у вікні, зі звуку, із файлу. A label or
  sentence starts with У, І, З (У вікні, І звук, З файлу).
- **Short labels:** Ukrainian runs about a quarter longer than English.
  To stay within a third: leave out what the page or menu already says
  (the page's name repeated in every option); take the shorter common
  word (Показувати, not Відображати; на весь екран, not у повноекранному
  режимі; список where the menu or page is the playlist's); use a noun
  phrase instead of a clause (Пауза під час згортання); write the
  compounds with аудіо-, відео-, авто- and мульти- as one word, as the
  orthography wants (аудіофайл, відеодоріжка, автовибір, автоповорот);
  use only the abbreviations above. Never cut words or make up
  abbreviations.
- **Words that trap** (Russianisms and calques, and Ukrainian's own
  pairs):
  - On and off are увімкнути / вимкнути (Увімкнено, Вимкнено); включити
    means include, and even then prefer додавати or з (Include the time:
    З часом).
  - Default is типовий, типово; never за замовчуванням or по
    замовчуванню.
  - Engine is рушій, never движок; floating point is рухома кома, never
    плаваюча.
  - Sort by is сортувати за, never по; depending on is залежно від; next
    is наступний; "is" is є or a dash, never являється; to match is
    збігатися; any is будь-який; to get is отримувати; during is
    протягом or під час; at (a moment, a condition) is під час, у разі,
    коли rather than при everywhere.
  - Address is адреса (feminine), never адрес. Speech is мовлення and
    language мова (розпізнавання мовлення). Task is завдання (панель
    завдань), never задача.
  - The genitive of the screen is екрана, of a disc диска, of a
    processor процесора, but of a file файлу, of a frame кадру, of sound
    звуку, of a list списку, of a channel каналу, of a line рядка.
  - Ukrainian has one verb, завантажити, for load and download. Keep it
    for downloads (models, TensorRT) and for loading a skin or a
    settings file; external subtitles and audio added to the playing
    video are підключити (Підключити субтитри..., Автоматично підключати
    зовнішні аудіофайли).
  - Remove (from a list) is вилучити, delete (the file itself, a
    bookmark, an album) is видалити.
  - The mouse pointer is вказівник миші; the desktop as a place is
    робочий стіл; the desktop environment (GNOME, KDE) is графічне
    середовище. The home folder is домашня тека, the clipboard буфер
    обміну, an optical drive привід and a drive letter диск (диск V:).
  - A text line is рядок; a row of pictures (thumbnails, scenes) is ряд.
  - "&" is і or та (Add & Edit: Додати та змінити).
- **Keys:** inside a combination, keys keep the names RVP's menus show:
  Ctrl+Left, Shift+Right, Alt+Enter, Space, PgUp, X1, X2 (Left / Right
  as a key label stays too). Arrow keys named in a sentence are клавіші
  зі стрілками; a mouse button named by its use takes the use in quotes:
  кнопка «Назад».

## Glossary

| English | Ukrainian | Note |
|---------|-----------|------|
| play / pause / stop | відтворити / призупинити / зупинити | Commands. The paused state is пауза (Пауза під час згортання); Play/Pause: Відтворити/призупинити. Never Стоп as a command. |
| playback | відтворення | Menu and page name Відтворення; during playback: під час відтворення. |
| close playback | завершити відтворення | Stops and unloads the file. Not зупинити (stop) and not Вийти, which is Exit. |
| previous / next | попередній / наступний | Agree with the noun: попередній елемент, наступний кадр, наступна доріжка. Never слідуючий. |
| seek | перемотування | Verb перемотати (both ways: вперед, назад). PotPlayer's Time Jump (the arrow keys' steps) is seeking too: Перемотування. Never пошук (search). |
| jump | перехід | Verb перейти до: Перейти до часу/кадру..., Перейти до наступного слайда. |
| frame step | покадровий перегляд | Next frame / Previous frame: Наступний кадр / Попередній кадр. |
| speed | швидкість | Швидкість відтворення. |
| repeat | повтор | Options use the imperfective infinitive: Повторювати файл, Повторювати список. |
| A-B repeat / section repeat | повтор A-B / повтор фрагмента | The marks are точка A and точка B (Latin letters); a marked part: фрагмент. |
| shuffle | випадковий порядок | Adverb випадково; Shuffle playback: Відтворення у випадковому порядку. |
| queue | черга | Playback queue: черга відтворення. |
| resume / remember position | продовження / запам’ятовувати позицію | Page name Продовження; one file: продовжити відтворення. Not відновлення, which reads as restore or recovery. |
| position | позиція | Only the time in the file (позиція відтворення). A place on the screen (window, logo, subtitles) is положення; a file's place on the disk is розташування. |
| duration | тривалість | Detect durations: Визначати тривалість. Not довжина. |
| elapsed / remaining time | час, що минув / час, що залишився | Short labels and choices: Минуло / Залишилося (Минуло й залишилося). |
| chapter | розділ | A DVD's title is заголовок (see title). |
| bookmark | закладка | Trap: a tab is вкладка, a different word; RVP's album tabs are вкладки. |
| skip (intro, ending) | пропустити (вступ, кінцівку) | Options: пропускати. The ending is кінцівка, not фінальні титри: титри is too close to субтитри. |
| seek bar | шкала відтворення | Also for nav bar and navigation bar. Never смуга прокрутки, which is the scroll bar. |
| thumbnail | мініатюра |  |
| preview | попередній перегляд | Thumbnail previews: мініатюри кадрів. |
| scene browser | переглядач сцен | Browse Scenes by...: Переглянути сцени за... Not браузер, which is a web browser. |
| file | файл | Genitive файлу. |
| folder | тека | GNOME's, KDE's and LibreOffice's word, everywhere, also on Windows: one word for one thing. Not папка, not каталог. |
| URL / address | URL / адреса | URL is not inflected: Відкрити URL..., Додати URL... Адреса is feminine (нова адреса), also for the remote control's IP address. |
| stream | потік | Network media. A track inside a file is доріжка: Вибрати аудіодоріжку, відеодоріжка. |
| device / capture device | пристрій / пристрій захоплення | Webcam: вебкамера. |
| screen capture | захоплення екрана | Not знімок екрана (a screenshot). |
| disc | диск | The drive that reads discs is привід (DVD-привід); a drive letter is диск too: name the thing where both could be read. |
| title | заголовок | A DVD's title only; Title Menu: Меню заголовків. The title of an entry, a bookmark or a window is назва (Назва:, Змінити назву). |
| angle | ракурс | Angle Menu: Меню ракурсів. |
| disc menu | меню диска |  |
| drag and drop | перетягування | Verb перетягнути; dropped files: перетягнуті файли. |
| recent files | нещодавні файли | Recent items: нещодавні елементи. Not останні, which is last. |
| favorites | обране | A singular collective noun: Додати до обраного, Змінити обране. |
| file association / file type | зіставлення файлів / тип файлу | As Windows says it; page name Зіставлення. Associate: зіставити з RVP. |
| playlist | список відтворення | Short список where the menu or page is the playlist's: Зберегти список як... |
| album | альбом |  |
| entry | елемент | Not позиція (the position in a file) and not запис (a recording). |
| sort | сортувати | Sort by: Сортувати за. |
| add / remove / delete | додати / вилучити / видалити | Вилучити takes an entry off the list and keeps the file; видалити deletes the file itself (and a bookmark, an album). To the Recycle Bin: перемістити до Кошика. |
| recycle bin / trash | Кошик / смітник | Windows' Recycle Bin is a name: Кошик, до Кошика. Linux's trash: смітник. |
| import / export | імпортувати / експортувати | Nouns імпорт / експорт: Імпорт налаштувань. |
| window / main window | вікно / головне вікно |  |
| fullscreen / windowed | на весь екран / у вікні | The command and the state: На весь екран, на весь екран на іншому дисплеї; a named mode or page: повноекранний режим, віконний режим. |
| maximize / minimize | розгорнути / згорнути | Restore: відновити; to the taskbar or tray: на панель завдань, у трей. |
| always on top / stay on top | завжди зверху / поверх інших вікон |  |
| opacity | непрозорість | Trap: transparency (Transparency of the menus) is прозорість. |
| skin | скін | Plural скіни, genitive скіна. Not тема (a color theme), not оболонка (a shell), not шкурка. |
| color theme | колірна тема | Колірна (of color), not кольорова (colorful). |
| logo | логотип | Masculine: власний логотип. Not лого. |
| title bar | рядок заголовка |  |
| control panel | панель керування | Керування, not управління, in every control: дистанційне керування. |
| OSD / on-screen display | OSD / екранні повідомлення | OSD stays where the source writes it (Повідомлення OSD); in sentences екранні повідомлення. Message: повідомлення. |
| on-screen controller | екранна панель | mpv's on-screen controller: екранна панель mpv; distinct from the F7 window, панель керування. |
| tooltip | підказка |  |
| context menu | контекстне меню |  |
| tray icon / notification area | значок у треї / область сповіщень | Трей in short labels (Значок на панелі завдань/у треї); область сповіщень in sentences. |
| taskbar | панель завдань | Never панель задач. On Linux, the dock is док. |
| notification | сповіщення | Trap: an OSD message is повідомлення. |
| dialog | діалогове вікно | Page name Діалогові вікна; short діалог only in help texts. |
| settings / preferences | налаштування / налаштування | One word for both: the F5 window, its title and Preferences... in the menu are Налаштування. One option is параметр (параметри сторінки). |
| page | сторінка |  |
| preset / configuration | пресет / конфігурація | Пресет for every saved set (settings, equalizer, frame sizes, quality): Пресет %1, типовий пресет, автопресет. Never передустановка. Configuration management: Керування конфігураціями. |
| reset / default | скинути / типовий | Скинути все; Restore to Default: Відновити типові; a Default button: Типово. Never за замовчуванням. |
| apply | застосувати | The report: Застосовано. |
| key / hotkey / shortcut | клавіша / гаряча клавіша / сполучення клавіш | Not скорочення. |
| mouse / wheel / button | миша / коліщатко / кнопка | Of the mouse: миші (коліщатко миші, жест миші). |
| gesture | жест |  |
| touch | сенсорне керування | Page name Сенсорне керування; the input itself: дотик (проведення одним пальцем). |
| controller / gamepad | контролер / геймпад | The Xbox controller: контролер Xbox. |
| video / picture / image | відео / зображення / зображення | Ukrainian has one word for the video's picture and a still image; a picture file is also зображення. Відео is neuter and not inflected. |
| frame | кадр | Genitive кадру. |
| frame rate / fps | частота кадрів / кадр/с | 60 кадр/с; FPS in a command name is частота кадрів (Convert Subtitle FPS: Змінити частоту кадрів субтитрів). |
| aspect ratio | співвідношення сторін | AR in the source is the same term written out. |
| crop | обрізати | Noun обрізання. |
| zoom / pan | масштаб / зсув | Zoom In / Zoom Out: Збільшити / Зменшити; Zoom %1: Масштаб %1. Verb for pan: зсунути. An offset is зміщення. |
| rotate / flip / mirror | повернути / перевернути / віддзеркалити | Noun поворот. Flip vertical: Перевернути по вертикалі; flip horizontal and mirror: Віддзеркалити по горизонталі. |
| deinterlace | деінтерлейсинг | The known term of video players; interlaced: черезрядковий; field order: порядок полів. |
| brightness / contrast / saturation / hue / gamma | яскравість / контрастність / насиченість / відтінок / гамма | Luma and chroma in effects: яскравість and кольоровість. |
| sharpen / denoise | різкість / знешумлення | Sharpen as an effect: Різкість; soften: Пом’якшення. Sound's noise reduction is шумозаглушення. |
| scaler / upscaling / resize | масштабування / збільшення / зміна розміру | Upscaler / Downscaler: Збільшення / Зменшення; Chroma scaler: Масштабування кольоровості. |
| pixel shader | піксельний шейдер |  |
| renderer | рендерер | Video renderer: рендерер відео; Audio Renderer: аудіорендерер. Output alone is виведення. |
| hardware decoding / decoder | апаратне декодування / декодер | Software: програмне декодування. |
| HDR | HDR |  |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / відео 360° / анагліф / поруч / один над одним | SBS and TAB stay where the source writes them. |
| bars / letterbox / margin | смуги / чорні смуги / поле | Bottom margin: нижнє поле; screen margins: поля екрана. An interlaced field is поле too: the context tells them apart. |
| frame capture / screenshot | захоплення кадру / знімок екрана |  |
| motion smoothing | згладжування руху |  |
| frame interpolation | інтерполяція кадрів |  |
| engine | рушій | Never движок. Engines and models: Рушії та моделі. |
| calibration | калібрування | Recalibrate: Повторити калібрування. |
| quality preset (light, medium, high) | пресет якості (легкий, середній, високий) | Fast / Medium / Quality as preset names: Швидкий / Середній / Якісний. |
| GPU / graphics card | GPU / відеокарта | GPU stays as the source writes it; "the card" is відеокарта. |
| processor / CPU | процесор / CPU | CPU stays as the source writes it, like GPU. |
| power budget | бюджет потужності | Power in watts is потужність, not енергія. |
| dropped frames | пропущені кадри |  |
| refresh rate | частота оновлення | Of the screen: частота оновлення екрана. |
| audio / sound | аудіо / звук | Menu and page name Аудіо; in sentences звук; compounds as one word: аудіофайл, аудіодоріжка, аудіопристрій. |
| volume / mute | гучність / без звуку | Mute as a command and a state: Без звуку. |
| master volume | загальна гучність | The system's own volume: загальна гучність системи. |
| equalizer | еквалайзер |  |
| normalizer | нормалізатор |  |
| reverb | реверберація |  |
| noise reduction | шумозаглушення | Of the sound only; the picture's is знешумлення. |
| channel / speaker | канал / динамік | Front, center, surround speakers: фронтальні, центральний, тилові. |
| downmix | зведення | Mixing down: зводити 5.1 до стерео. |
| surround / stereo | об’ємний звук / стерео | Стерео is neuter and not inflected. |
| output device | пристрій виведення | Виведення, not вивід or вихід. |
| sample rate / bitrate | частота дискретизації / бітрейт |  |
| audio delay / audio sync | затримка звуку / синхронізація звуку |  |
| pitch / time stretching | висота тону / розтягування в часі | Pitch shifting: зсув висоти тону. |
| audio recording | запис звуку | A recording is запис; record: записати. |
| second audio device | другий аудіопристрій |  |
| pass-through | пряма передача | The encoded sound goes to the receiver as it is. |
| subtitles / subtitle track | субтитри / доріжка субтитрів | Plural only: субтитри. One line of them: рядок субтитрів; one subtitle (an event): субтитр. |
| external / embedded subtitles | зовнішні / вбудовані субтитри |  |
| font / outline / shadow | шрифт / контур / тінь | Outline width: товщина контуру. |
| subtitle position / subtitle delay | положення субтитрів / затримка субтитрів |  |
| encoding | кодування | A text file's character set: кодування тексту. |
| subtitle browser / subtitle editor | переглядач субтитрів / редактор субтитрів |  |
| subtitles from audio / speech recognition | субтитри зі звуку / розпізнавання мовлення | Мовлення (speech), not мова (language). |
| model | модель |  |
| language | мова | Of the interface, a track, the speech: мова інтерфейсу, мова доріжки. |
| remote control | дистанційне керування | A remote program: пульт (Unified Remote's remotes: пульти). |
| token | токен |  |
| port / server | порт / сервер |  |
| log | журнал | To log: записувати в журнал. |
| crash | збій | Crash logs: журнали збоїв. |
| download / install | завантажити / встановити | See "Words that trap" for load. Installed: встановлено. |
| installer | інсталятор |  |
| accessibility | спеціальні можливості | As Windows calls them. |
| text to speech (TTS) / screen reader | синтез мовлення (TTS) / програма читання з екрана | TTS stays where the source writes it. |
| screen saver | заставка |  |
| shut down / sleep / hibernate | вимкнути комп’ютер / сон / глибокий сон | The computer's power actions; as choices: Вимкнути комп’ютер, Сон, Глибокий сон. |
| timer | таймер | Shutdown timer: таймер вимкнення. |
| visualization / spectrum | візуалізація / спектр |  |
| status bar / info | рядок стану / відомості | Playback/System Info: Відомості про відтворення/систему. Not інфо. |
| codec / format | кодек / формат |  |
| bit depth / 10-bit | глибина кольору / 10-бітний | The adjective agrees with its noun: 10-бітне виведення; the sample formats agree with формат: 16-бітний, 32-бітний (з рухомою комою). |
| channel layout (5.1, 7.1) | схема каналів (5.1, 7.1) |  |
| Auto select (recommended) | Автовибір (рекомендовано) | Auto select alone: Автовибір. |
