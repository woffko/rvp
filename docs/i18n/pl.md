# Polish (pl)

Standard Polish as current desktop software in Poland writes it: the
plain, neutral register of Windows', GNOME's and KDE's Polish
interfaces, with the user addressed in the second person singular where
an instruction needs it, as those desktops do. The translation says what
each control does in the terms a Polish user of a media player knows
(lista odtwarzania, napisy, skórka, korektor), not in calques of the
English or of the Russian, which is read for meaning only. Polish
letters (ą ć ę ł ń ó ś ź ż) are always written; names, codecs, file
names and keys stay as README lists them.

## Style

- **Address.** Instructions use the imperative, second person singular:
  Wybierz działanie, a następnie dodaj klawisz lub gest myszy. Naciśnij
  klawisz dla „%1”. Leave out the pronouns (ty, twój, swój) wherever the
  sentence allows; where one is needed it is lowercase. Never Pan/Pani,
  Państwo or Proszę with an infinitive. The past tense of the second
  person shows gender (pobrałeś/pobrałaś): use the present tense or an
  impersonal form instead (pliki, które pobierasz; pobrane modele).
- **Help texts** (sentences under a page's options): present tense, third
  person, with RVP, mpv or the feature as the subject: RVP mierzy...,
  Kalibracja trwa od jednej do trzech minut. Reports of what happened use
  the impersonal -no/-to form: Zastosowano. Zaimportowano. Anulowano.
  Bez zmian. Nic nie znaleziono. Usunięto: %1.
- **Commands** (menu items, buttons, actions): the perfective imperative:
  Otwórz folder..., Zapisz, Dodaj zakładkę, Usuń, Przejdź do
  czasu/klatki..., Zamknij. The desktops' own forms stay as Polish users
  know them: Kopiuj, Wklej, Wytnij, Cofnij, Ponów, Zaznacz wszystko,
  Zastosuj, Anuluj, Przeglądaj…, Zakończ (Exit). Where the English label
  is a noun (Playback, Subtitles, Audio Recording), Polish uses a noun
  (Odtwarzanie, Napisy, Nagrywanie dźwięku).
- **Checkboxes and options for standing behaviour:** the imperfective
  imperative, which says "every time": Zapamiętuj pozycje odtwarzania,
  Pokazuj podpowiedzi, Wstrzymuj odtwarzanie po zminimalizowaniu, Dodawaj
  podobne pliki, Pomijaj czołówkę, Blokuj wygaszacz ekranu podczas
  odtwarzania. "Enable X" stays perfective: Włącz pomijanie, Włącz
  XInput. A checkbox whose English is a noun phrase may stay one.
- **Choices** (radio buttons, list items, menu values): a noun phrase, an
  adjective that agrees with the label's noun, or an adverb, which never
  changes: Nigdy, Zawsze, Automatycznie, Podczas odtwarzania, Losowo. A
  state or value with no noun in sight is neuter, as Windows' switches
  are: Włączone, Wyłączone, Domyślne. "(Recommended)" is always
  "(zalecane)" and "(Default)" "(domyślne)": PCM (domyślne) (zalecane).
- **Titles** of windows, pages, groups and columns are noun phrases:
  Odtwarzanie, Pełny ekran, Czas trwania, Ustawienia wizualizacji. A
  window that a command opens keeps the command's words without the
  ellipsis, as the source does: Przejdź do czasu/klatki.
- **Questions** in dialogs take the infinitive, the usual Polish form:
  Przenieść pliki do Kosza (%1)? Przywrócić domyślne wartości opcji
  strony „%1”? The answer buttons are imperatives: Przenieś, Zachowaj.
- **Capitalization:** sentence case everywhere, menus, buttons and window
  titles included: only the first word and names take a capital
  (Ustawienia odtwarzania..., Pokazuj miniatury na pasku postępu). The
  English Title Case is never copied. After a slash inside a label the
  next word is lowercase (Odtwórz/wstrzymaj, Rozszerzanie/przycinanie),
  and after a colon too (Kalibracja: anulowano; Akcja: hibernacja),
  names apart. Languages, days and months are lowercase (angielski,
  polski, as in the whisper language list); the interface language's own
  name, a label of its own, is Polski. Kosz, the name of Windows' Recycle
  Bin, keeps its capital.
- **Quotation marks:** Polish „…” (U+201E, U+201D) for quoted labels and
  option names, «…» inside them; the source's straight and curly
  quotes both become „…”. Help texts quote buttons and options with
  their Polish label exactly: „Zastosuj”, „Dopasuj okno do nowo
  otwartego wideo”. Menu paths stay unquoted, with the Polish labels and
  the source's ">": Wideo > Margines dolny.
- **Punctuation:** a comma before every subordinate clause (że, gdy,
  kiedy, jeśli, aby, który, ponieważ, czy) and before an adverbial
  participle (-ąc, -wszy); none before i, oraz, lub in a list. A spaced
  en dash " – " where the English has " - " or a pause
  (%1/%2/%3 – %4 stereo); an unspaced en dash in ranges (0–255,
  16–235); hyphens in compounds (10-bitowy, 3D-...). The usual
  abbreviations take a period: np. (e.g.), itp. (etc.), tj. (i.e.),
  maks.; "No." and "#" before a number are nr ("Style #%1" is
  "Styl nr %1").
  Do not write min. for "minimum": min is minutes.
- **The ellipsis** copies the source's form: "..." where it has three
  periods, "…" where it has the one character, with no space before it
  (Otwórz plik...), except where the source has one (Shutdown at ...:
  Wyłącz komputer o ...). Never add or drop one.
- **Units:** a space between a number and its unit, also where the
  English writes them together: %1 s, %1 ms, %1 min, %1 h, %1 Hz,
  %1 kHz, %1 dB, %1 MB, %1 GB, %1 W, 60 fps (%1ms: %1 ms, %1Hz: %1 Hz).
  Seconds are s (never sek.), minutes min with no period, hours h; the
  source's Sec., Sec and Min become s and min. %, x (2x, %1x%2), K,
  p (1080p), px and ° stay exactly as the source writes them, with or
  without its space. Unit symbols are not inflected, which makes them the
  safe choice after a placeholder: Przewiń o %1 s wstecz, never
  %1 sekund.
- **Numbers:** a decimal comma in numbers the translation writes itself
  (1,5 s, 0,5x); the period stays where it is part of a name or of what
  the user types: versions, the channel layouts 5.1 and 7.1, codec names,
  time formats (g:mm:ss.s). Thousands take a space from five digits
  (10 000), none in four (1000 ms, 2160p). Ordinals are the digit and a
  period: "%1nd Device" is "%1. urządzenie", "{Same as %1st logo}" is
  "{Jak %1. logo}". Time-format hints use Polish letters: hh:mm is gg:mm, h:mm
  is g:mm. Numbers that come through placeholders are RVP's and stay as
  they come.
- **Counts:** Polish has three forms after a number (1 plik, 2–4 pliki,
  5–21 plików, 22–24 pliki, 25 plików) and the strings have no plural
  forms, so a noun never follows %1–%9 directly unless its number is
  known. Instead: the noun in the genitive plural before a colon
  (Plików: %1, Znalezionych skryptów: %2); the count in parentheses after
  a plural noun (Przenieść pliki do Kosza (%1)?); a unit symbol (s, min,
  GB; the source's chs has none in Polish: kanałów: co najmniej %1); or
  a preposition that takes the genitive, which serves every number
  above 1 (do %1 wierszy). Where a pair of strings gives the singular
  and the plural and the page shows which numbers they take (Reserve %1
  line, Reserve %1 lines: 1, then 2 and 3), use those numbers' form:
  Zarezerwuj %1 wiersz, Zarezerwuj %1 wiersze. A verb after a count
  becomes impersonal: "%1 removed" is "Usunięto: %1".
- **Gender:** adjectives and participles agree with their noun (domyślny
  rozmiar, własna wysokość, włączona animacja); "Custom" is własny,
  własna or własne by its noun, and Własne when it stands alone. RVP,
  mpv, SVP and the other programs are masculine (a program); a library
  called biblioteka is feminine. The user's gender never shows (see Address).
- **Names are not inflected.** A name may follow a noun as it is
  (skórki PotPlayer, renderer mpv, filtr SVP, karty NVIDIA, biblioteka
  FFmpeg); where a preposition needs a case, a declinable noun goes
  before the name (w programie PotPlayer, w systemie Windows, dla
  systemu Linux, w środowisku GNOME). Never PotPlayera, PotPlayer'a or
  Windowsie.
- **Short labels:** Polish runs about a quarter longer than English. To
  stay within a third: leave out what the page or menu already says (the
  page's name repeated in every option); take the shorter common word
  (Pokazuj, not Wyświetlaj; na pełnym ekranie, not w trybie
  pełnoekranowym; lista where the menu or page is the playlist's); use a
  noun phrase instead of a clause (Pauza przy minimalizacji); put audio
  and wideo after a noun instead of a longer adjective (urządzenie audio,
  plik wideo); use only the abbreviations above. Never cut words or make
  up abbreviations.
- **Words that trap:** wideo, not video (both wideo and audio are not
  inflected; Wideo is the menu's name). The desktop as a place is pulpit
  (względem pulpitu); the desktop environment (GNOME, KDE) is środowisko
  graficzne. The home folder is folder domowy, the clipboard schowek, an
  optical drive napęd and a drive letter dysk. "&" is i (Add & Edit:
  Dodaj i edytuj).
- **Keys:** inside a combination, keys keep the names RVP's menus show:
  Ctrl+Left, Shift+Right, Alt+Enter, Space, PgUp, X1, X2 (Left / Right
  as a key label stays too). Arrow keys named in a sentence are
  strzałki: klawisze strzałek.

## Glossary

| English | Polish | Note |
|---------|--------|------|
| play / pause / stop | odtwórz / wstrzymaj / zatrzymaj | Commands. The paused state is pauza (Pauza przy minimalizacji); Play/Pause: Odtwórz/wstrzymaj. |
| playback | odtwarzanie | Menu and page name Odtwarzanie; during playback: podczas odtwarzania. |
| close playback | zakończ odtwarzanie | Stops and unloads the file. Not zatrzymaj (stop) and not Zakończ alone, which is Exit. |
| previous / next | poprzedni / następny | Agree with the noun: poprzedni element, następna klatka, następny rozdział. |
| seek | przewijanie | Verb przewiń (both ways). PotPlayer's Time Jump (the arrow keys' steps) is seeking too: przewijanie. Never szukanie (search). |
| jump | przejście | Verb przejdź do: Przejdź do czasu/klatki..., Przejdź do następnego slajdu. |
| frame step | klatka po klatce | Next frame / Previous frame: Następna klatka / Poprzednia klatka. |
| speed | szybkość | Szybkość odtwarzania; not prędkość. |
| repeat | powtarzanie | Options use the imperfective imperative: Powtarzaj plik, Powtarzaj listę. |
| A-B repeat / section repeat | powtarzanie A-B / powtarzanie fragmentu | The marks are punkt A and punkt B; a marked part: fragment. |
| shuffle | odtwarzanie losowe | Adverb losowo; random order: losowa kolejność. |
| queue | kolejka | Playback queue: kolejka odtwarzania. |
| resume / remember position | wznawianie / zapamiętuj pozycję | Page name Wznawianie; one file: wznów. |
| position | pozycja | Only the time in the file (pozycja odtwarzania). A place on the screen (window, logo, subtitles) is położenie. |
| duration | czas trwania | Detect durations: Ustalaj czas trwania. Not długość. |
| elapsed / remaining time | czas od początku / czas do końca | Short labels: Od początku / Do końca. |
| chapter | rozdział |  |
| bookmark | zakładka | Trap: zakładka is also a tab in other software; RVP's album tabs are karty. |
| skip (intro, ending) | pomiń (czołówkę, zakończenie) | Options: pomijaj. Not napisy końcowe: napisy are subtitles. |
| seek bar | pasek postępu | Also for nav bar and navigation bar. Never pasek przewijania, which is Windows' scroll bar. |
| thumbnail | miniatura |  |
| preview | podgląd |  |
| scene browser | przeglądarka scen |  |
| file | plik |  |
| folder | folder | As in Windows; not katalog. |
| URL / address | URL / adres | URL is masculine and not inflected in labels: Otwórz URL..., Dodaj URL... |
| stream | strumień | Network media. A track inside a file is ścieżka: Wybierz ścieżkę dźwiękową, ścieżka wideo. |
| device / capture device | urządzenie / urządzenie przechwytujące | Webcam: kamera internetowa. |
| screen capture | przechwytywanie ekranu | Not zrzut ekranu (a screenshot). |
| disc | płyta | Trap: dysk is a disk or a drive letter; the drive that reads discs is napęd (napęd DVD). |
| title | tytuł | A DVD's title; Title Menu: Menu tytułów. |
| angle | kąt kamery | Short kąt where the DVD context is clear: Menu kątów. |
| disc menu | menu płyty |  |
| drag and drop | przeciąganie i upuszczanie | Verb przeciągnij; dropped files: upuszczone pliki. |
| recent files | ostatnie pliki | Recent items: ostatnie elementy. |
| favorites | ulubione | Used as a plural noun: Dodaj do ulubionych, Edytuj ulubione. |
| file association / file type | skojarzenie plików / typ pliku | Page name Skojarzenia. |
| playlist | lista odtwarzania | Short lista where the menu or page is the playlist's: Zapisz listę jako... |
| album | album |  |
| entry | element | Not pozycja: that is the position in a file. |
| sort | sortuj | Sort by: Sortuj według. |
| add / remove / delete | dodaj / usuń z listy / usuń plik | Polish has one verb, usuń: name what goes where both could be read. Usuń z listy keeps the file; usuń plik or pliki deletes it. To the Recycle Bin: przenieś do Kosza. |
| recycle bin / trash | Kosz / kosz | Windows' Recycle Bin is a name: Kosz, do Kosza. Linux's trash: kosz. |
| import / export | importuj / eksportuj | Nouns import / eksport: Import ustawień. |
| window / main window | okno / okno główne |  |
| fullscreen / windowed | pełny ekran / w oknie | In fullscreen: na pełnym ekranie; a named mode: tryb pełnoekranowy, tryb okienkowy. |
| maximize / minimize | maksymalizuj / minimalizuj | Restore: przywróć; to the taskbar or tray: na pasek zadań, do zasobnika. |
| always on top / stay on top | zawsze na wierzchu / na wierzchu |  |
| opacity | nieprzezroczystość | Trap: transparency (Transparency of the menus) is przezroczystość. |
| skin | skórka | Plural skórki; the audio skin: skórka audio. |
| color theme | motyw kolorów |  |
| logo | logo | Neuter, not inflected; a plural, if needed: logotypy. |
| title bar | pasek tytułu |  |
| control panel | panel sterowania | RVP's F7 window. The same words name Windows' Control Panel: write panel sterowania RVP where a sentence could mean Windows'. |
| OSD / on-screen display | OSD / komunikaty ekranowe | OSD Messages page: Komunikaty ekranowe; mpv's OSD: OSD mpv. |
| on-screen controller | nakładka sterująca | mpv's OSC and scripts; the Controller choice: Nakładka. Not kontroler, which is a gamepad. |
| tooltip | podpowiedź |  |
| context menu | menu kontekstowe |  |
| tray icon / notification area | ikona w zasobniku / obszar powiadomień | Tray alone: zasobnik (w zasobniku, do zasobnika). |
| taskbar | pasek zadań | Also a Linux dock. |
| notification | powiadomienie |  |
| dialog | okno dialogowe | Short okno where clear. |
| settings / preferences | ustawienia / ustawienia | One word for both, as in Windows: the window and the Preferences... item are Ustawienia. Not preferencje (preferowane is preferred: Preferowane języki). |
| page | strona | In help texts: strona „Wideo”. |
| preset / configuration | profil / konfiguracja | Every saved set of settings: RVP's presets, the equalizer's, the frame sizes'. The page that manages them: Konfiguracje. Not preset or ustawienie wstępne. |
| reset / default | resetuj / domyślne | Reset (button): Resetuj; back to the defaults: przywróć domyślne. Default agrees with its noun (domyślny rozmiar, domyślna czcionka); with no noun: domyślne. |
| apply | zastosuj | The report "Applied." is "Zastosowano." |
| key / hotkey / shortcut | klawisz / klawisz skrótu / skrót klawiszowy |  |
| mouse / wheel / button | mysz / kółko / przycisk | kółko myszy; X1 and X2 stay. |
| gesture | gest |  |
| touch | dotyk | Touch page and feature: sterowanie dotykowe; a touchscreen: ekran dotykowy. |
| controller / gamepad | kontroler / gamepad | kontroler Xbox. |
| video / picture / image | wideo / obraz / obraz | wideo is not inflected (pliku wideo); never video in Polish text except in names. A still image is obraz as well; a slideshow's photos may be zdjęcia. |
| frame | klatka | One picture in time. The picture's area (Frame Size, Keep Video Frame, the frame changes) is kadr. |
| frame rate / fps | liczba klatek na sekundę / fps | fps after a number (60 fps). The video's or display's rate in short labels: częstotliwość (Target rate: Częstotliwość docelowa). |
| aspect ratio | proporcje obrazu | Short proporcje (plural); AR in menu labels too. |
| crop | przytnij | Noun przycinanie: Rozszerzanie/przycinanie. |
| zoom / pan | powiększenie / przesunięcie | Zoom In / Zoom Out: Powiększ / Pomniejsz. |
| rotate / flip / mirror | obróć / odwróć / odbij lustrzanie | Flip Vertical (upside down): Odwróć w pionie; Flip Horizontal (left-right): Odwróć w poziomie. |
| deinterlace | usuwanie przeplotu | Interlaced: z przeplotem; mpv's filter names stay. |
| brightness / contrast / saturation / hue / gamma | jasność / kontrast / nasycenie / odcień / gamma |  |
| sharpen / denoise | wyostrzanie / odszumianie | Effect names are nouns: Wyostrzanie (Ctrl+R), Odszumianie 3D. |
| scaler / upscaling / resize | skaler / powiększanie / zmiana rozmiaru | Upscaler / Downscaler labels: Powiększanie / Pomniejszanie. The Resize page: Zmiana rozmiaru. |
| pixel shader | shader pikseli | Plural shadery pikseli; pre-/post-resize: przed zmianą rozmiaru / po zmianie rozmiaru. |
| renderer | renderer | renderer wideo; PotPlayer's Audio Renderer (the list of output devices): wyjście audio. |
| hardware decoding / decoder | dekodowanie sprzętowe / dekoder | Software decoding: dekodowanie programowe. |
| HDR | HDR |  |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / wideo 360° / anaglif / obok siebie / góra-dół | SBS and TAB stay; cyan and magenta: cyjan, magenta. |
| bars / letterbox / margin | czarne pasy / letterbox / margines | letterbox is the videophiles' word and stays; a bar alone is pas. |
| frame capture / screenshot | przechwytywanie klatki / zrzut ekranu | Saved frames: zapisane klatki. |
| motion smoothing | wygładzanie ruchu |  |
| frame interpolation | interpolacja klatek |  |
| engine | silnik | silnik whisper.cpp, silnik TensorRT. |
| calibration | kalibracja | Calibrate…: Kalibruj… |
| quality preset (light, medium, high) | profil jakości (lekki, średni, wysoki) | Fast / Medium / Quality choices: Szybki / Średni / Jakościowy (masculine: profil). |
| GPU / graphics card | GPU / karta graficzna | Short karta where the source says the card. |
| processor / CPU | procesor / CPU |  |
| power budget | budżet mocy | Not limit mocy, which is the card's own power limit. |
| dropped frames | utracone klatki | Not pominięte (skipped); dropping: utrata klatek. |
| refresh rate | częstotliwość odświeżania |  |
| audio / sound | audio / dźwięk | Menus and pages: Dźwięk. audio is not inflected after a noun (urządzenie audio, plik audio); an audio track: ścieżka dźwiękowa. |
| volume / mute | głośność / wycisz | The state and the toggle: wyciszenie. |
| master volume | głośność główna | System Master volume: systemowa głośność główna. |
| equalizer | korektor | Not equalizer; SuperEQ stays. |
| normalizer | normalizator |  |
| reverb | pogłos |  |
| noise reduction | redukcja szumów | Sound only; the picture's denoise is odszumianie. |
| channel / speaker | kanał / głośnik | Output Speakers: Głośniki. |
| downmix | downmiks | Polish spelling, inflected (downmiksu); the verb: miksować do stereo. |
| surround / stereo | dźwięk przestrzenny / stereo | Surround as a speaker group (Front, Center, Surround): tylne (kanały tylne). |
| output device | urządzenie wyjściowe |  |
| sample rate / bitrate | częstotliwość próbkowania / przepływność | przepływność bitowa where przepływność alone could be misread. |
| audio delay / audio sync | opóźnienie dźwięku / synchronizacja dźwięku |  |
| pitch / time stretching | wysokość dźwięku / rozciąganie czasu | Pitch shifting: zmiana wysokości dźwięku. |
| audio recording | nagrywanie dźwięku |  |
| second audio device | drugie urządzenie audio |  |
| pass-through | przekazywanie bez dekodowania | Short przekazywanie only where the same label or list already says bez dekodowania. |
| subtitles / subtitle track | napisy / ścieżka napisów | Plural noun; one subtitle (a line) is napis. |
| external / embedded subtitles | napisy zewnętrzne / wbudowane |  |
| font / outline / shadow | czcionka / obrys / cień |  |
| subtitle position / subtitle delay | położenie napisów / opóźnienie napisów |  |
| encoding | kodowanie znaków | Short kodowanie: Kodowanie tekstu. |
| subtitle browser / subtitle editor | przeglądarka napisów / edytor napisów |  |
| subtitles from audio / speech recognition | napisy z dźwięku / rozpoznawanie mowy |  |
| model | model |  |
| language | język |  |
| remote control | zdalne sterowanie | The remotes (programs on a phone, Unified Remote's) are piloty. |
| token | token |  |
| port / server | port / serwer |  |
| log | dziennik | The Log page: Dziennik; to log: zapisywać w dzienniku. |
| crash | awaria | A crash log: raport awarii. |
| download / install | pobierz / zainstaluj | Not ściągnij. |
| installer | instalator |  |
| accessibility | ułatwienia dostępu |  |
| text to speech (TTS) / screen reader | synteza mowy (TTS) / czytnik ekranu |  |
| screen saver | wygaszacz ekranu |  |
| shut down / sleep / hibernate | wyłącz komputer / uśpij / hibernuj | Choices and states are nouns: wyłączenie, uśpienie, hibernacja. Shutdown Timer: Timer wyłączenia. |
| timer | timer |  |
| visualization / spectrum | wizualizacja / widmo |  |
| status bar / info | pasek stanu / informacje |  |
| codec / format | kodek / format |  |
| bit depth / 10-bit | głębia bitowa / 10-bitowy | The adjective agrees: 10-bitowe wyjście, 16-bitowy format. |
| channel layout (5.1, 7.1) | układ kanałów (5.1, 7.1) |  |
| Auto select (recommended) | Wybór automatyczny (zalecane) |  |
