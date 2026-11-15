# German (de)

Standard German as written in Germany (the reformed spelling, with ß),
in words that readers in Austria and Switzerland use too: no regional
terms, and nothing that only one country's keyboards or programs call a
thing. The translation addresses the user formally ("Sie"), as the
Windows, GNOME and KDE desktops do in German, but most strings need no
address at all: labels are infinitive or noun phrases, and help texts
describe what RVP does, as the English does. The aim is the plain
vocabulary of a German desktop media player (Wiedergabe, Untertitel,
Zeitleiste, Papierkorb), established loanwords where Germans use them
(Skin, Playlist, Stream, Codec, Equalizer), and labels short enough for
the settings window's 240-pixel labels and 300-pixel choice lists.

## Style

- **Address.** Formal "Sie", always capitalized ("Sie", "Ihr", "Ihnen").
  Use it only where the source speaks to the user ("you download",
  "Type a subtitle and press Enter"): "Geben Sie einen Untertitel ein
  und drücken Sie Enter". Otherwise write impersonally, as the source
  does ("RVP misst ...", "Die Kalibrierung läuft, wenn ..."). Never "du",
  never "bitte" in labels; in help texts only where a real request is
  made.
- **Instructions and questions.** Help texts are full sentences in the
  present tense. Confirmation questions take the infinitive form that
  German dialogs use: "%1 Datei(en) in den Papierkorb verschieben?",
  "Allen Aktionen wieder die Tasten von PotPlayer zuweisen?". Status
  notices take a past participle: "Übernommen.", "Abgebrochen.",
  "Importiert.", "Unverändert.". Progress texts take the passive or a
  noun: "Wird gestartet…", "%1 wird gemessen…".
- **Menu items and buttons** are commands: the object first, the verb in
  the infinitive last, no article ("Datei(en) öffnen...", "Lesezeichen
  hinzufügen", "Playlist speichern unter..."). A one-word command is the
  bare infinitive ("Öffnen", "Speichern", "Abbrechen", "Schließen",
  "Übernehmen"). The standard desktop words: Save As "Speichern unter",
  Browse "Durchsuchen", Choose "Auswählen", Exit "Beenden", About "Über",
  Tools "Extras", Misc "Sonstiges", Copy "Kopieren", Paste "Einfügen",
  Undo "Rückgängig", Redo "Wiederherstellen" (never "Wiederholen", which
  is repeat).
- **Checkboxes** are infinitive phrases with the verb last
  ("Wiedergabepositionen merken", "Tooltips anzeigen",
  "Bildschirmschoner während der Wiedergabe verhindern"). Enable /
  Disable / Use are "aktivieren" / "deaktivieren" / "verwenden"; On /
  Off as choices are "Ein" / "Aus"; Enabled / Disabled are "Aktiviert" /
  "Deaktiviert".
- **Choices** in lists are short noun phrases, adjectives or adverbs:
  "Nie", "Immer", "Während der Wiedergabe", "Hohe Qualität",
  "Automatisch (empfohlen)". "None" depends on the noun's gender in
  German: write "Keine", "Kein" or "Ohne" to suit what is chosen, and
  check the other choices of the same list.
- **Page titles, headings, window titles and field labels** are nouns or
  noun phrases ("Wiedergabe", "Zeitsprung", "Seitenverhältnis",
  "Schriftgröße"); "Section Repeat" becomes "Abschnittswiederholung",
  not a verb. A label before a field ends with a colon only where the
  source has one.
- **Capitalization** follows German grammar, not the English Title
  Case: the first word and every noun are capitalized, everything else
  is lowercase ("Datei(en) öffnen...", "Nächstes Kapitel", "Zu Zeit/Bild
  springen..."). After a colon, lowercase unless a full sentence or a
  noun follows. Names keep their own case even at the start (mpv,
  libmpv, yt-dlp, whisper.cpp, dav1d).
- **Quotation marks** are German: „…“ (and ‚…‘ inside them), both for the
  source's “…” and for its straight "...": „%1“, „Hinzufügen zu“. When a
  text names another label, page or menu, it quotes that label's German
  text exactly ("ist „Fenster an neu geöffnetes Video anpassen“ aktiv
  (Seite „Wiedergabe“)"); menu paths keep the source's ">" between the
  German labels.
- **Other punctuation.** No space before ":", ";", "?", "!" or ")". The
  source's colons and semicolons stay where German allows them; a dash
  that sets off a phrase is " – " (an en dash with spaces). Hyphens join
  names, abbreviations and digits to German words
  (see the length rules). The genitive of a name is not written with an
  English apostrophe: "RVP's settings" is "die Einstellungen von RVP" or
  "RVP-Einstellungen", "RVP's own filters" "RVP-eigene Filter", "mpv's
  default" "mpv-Standard", "Windows' default" "Windows-Standard".
- **"..." and "…".** The end of a menu item or button keeps exactly what
  the source has: three periods "..." where the source has them (the
  menus), the single character "…" where it has that (the QML windows),
  attached to the word without a space ("Öffnen...", "Durchsuchen…").
  Where the source has a space before it ("Shutdown at ...") the space
  stays.
- **Keys** keep RVP's own names, the ones its menus print, even where
  German keyboards label them differently: Ctrl (not Strg), Shift (not
  Umschalt), Alt, Enter (not Eingabe), Space (not Leertaste), Esc, Tab,
  PgUp, PgDn (not Bild auf/ab), Home (not Pos1), End (not Ende), Del,
  Ins, F1...F12, Num+, the arrow keys' Left, Right, Up and Down inside
  key names and lists of keys ("Ctrl+Left / Right", "Up, Down,
  Mausrad"), and the letters and punctuation keys as in the source. In
  running text about keys in general, German words: "die Pfeiltasten",
  "die Lautstärketasten der Tastatur". The gamepad's buttons (A, B, X,
  Y, Back, Start) and the mouse's X1 and X2 keep their labels; the
  mouse's back and forward buttons are "Zurück" and "Vorwärts".
- **Units.** Seconds "s", minutes "min", hours "h", milliseconds "ms";
  "Sec", "Sec.", "Min" and "ms" in the menus become "s", "s", "min" and
  "ms" ("%1 s zurück", "Vorspann %1 s überspringen"). dB, Hz, kHz, fps,
  px, K, p (1080p), GB, MB, W and % stay. German puts a space between a
  number and its unit, the percent sign included: "%1 %", "%1 ms",
  "%1 Hz", "%1 dB", "%1 GB", also where the source writes "%1%" or
  "%1ms". Degrees stay attached ("%1°", "360°"). Written out in a
  sentence: "Sekunden", "Minuten", "Bilder pro Sekunde". The
  multiplier stays "x" ("%1x", "2x").
- **Numbers.** In prose the decimal separator is a comma ("1,5 s",
  "0,5"); a thousands separator, rarely needed, is a period. Notation
  that is a name or that RVP parses keeps the source's form: channel
  layouts (5.1, 7.1), aspect ratios and sizes (16:9, 2.35:1, %1:%2,
  %1.%2, 3x2, %1x%2), versions, ranges such as 0-255 and 16-235, and
  input formats ("h:mm:ss.s", "hh:mm", "h:mm"). Ordinals are the number
  with a period: "%1nd Device" is "%1. Gerät", "%1st logo" "%1. Logo".
  Bits: "16 Bit", "%1 Bit (empfohlen)", and as part of a compound
  "10-Bit-Ausgabe".
- **Placeholders and agreement.** %1 may stand for a noun of any
  gender, a number or a name, so nothing may agree with it: no article,
  adjective or pronoun that depends on %1. Put it after a colon
  ("Status: %1", "Verwendet: %1"), in quotes as a name („%1“ entfernt),
  or in a passive clause ("%1 wird gemessen…"). Counts that can be one
  or many use the parenthesized plural ("%1 Datei(en)"); where the
  source has separate singular and plural strings, use the matching
  German form in each. Words that RVP inserts into another string
  (status words such as "checking", "downloading") must read right at
  that place: check the receiving string ("RIFE (TensorRT): %1, %2 von
  %3 GB" takes "Prüfen", "Entpacken", "Herunterladen").
- **Values stay as they are.** Example and default values that RVP
  matches or parses keep the source's text: chapter titles to skip
  ("Opening;Ending;3"), language codes ("rus,eng", "en,ru", "en"),
  drive letters and mount points ("V:", "/mnt/media"), the yt-dlp format,
  file patterns ("*.dsf", "*.json"), the default folders RVP really
  uses ("Music/RVP"; "Pictures/RVP in the home folder" is "Pictures/RVP
  im Benutzerordner"), and the playlist's album named "Default" (data,
  shown as „Default“). Whisper's model names (tiny, base, small,
  medium, large) stay; their notes are German ("(nicht
  heruntergeladen)").
- **Language names** are German nouns, capitalized, though the source
  writes them in lowercase: "Albanisch", "Deutsch", "Haitianisches
  Kreolisch", "Nynorsk". The language's own name (the Language
  context) is "Deutsch".
- **Short labels.** German runs about a third longer than English, and
  long compounds are the main risk: a single word cannot wrap, and the
  settings window's labels are 240 pixels wide, its choice lists 300.
  - Drop articles, possessives and pronouns that the English keeps:
    "Show the mouse pointer" is "Mauszeiger anzeigen", "Remember the
    A-B repeat lists" "A-B-Wiederholungslisten merken".
  - Prefer a verb to a noun chain: "Größe ändern" over
    "Größenänderung", "vor dem Skalieren" over "vor der Durchführung
    der Skalierung".
  - Write compounds closed as German does ("Schriftgröße", "Audiospur",
    and the glossary's own long words such as "Abschnittswiederholung"
    or "Audiosynchronisation"). A compound that the translation forms
    itself and that runs past about 22 letters is split at its main
    joint with a hyphen, which Duden allows for readability and which
    lets the line wrap there ("Untertitel-Synchronisation",
    "Wiedergabe-Einstellungen..."), or becomes a phrase when that is
    shorter. Never stack three nouns into one word where a genitive or
    "für" reads better. Names, abbreviations and digits always
    join with a hyphen ("RVP-Fenster", "DVD-Menü", "mpv-Ordner",
    "3D-Videomodus", "%1D-Videomodus", "360°-Video", "AC3-Neucodierung",
    "A-B-Wiederholung", "Blu-ray-Laufwerk").
  - An explanation in parentheses may be shortened when the label stays
    clear; "(Recommended)" is "(empfohlen)", "(Recom'd)" "(empf.)".
    Accepted abbreviations in tight labels: "max.", "min.", "z. B.",
    "empf."; no others ("bzw.", "ggf.", "autom." are not used).
  - Where the source's label repeats its heading ("Enable (Shift+N)"
    under "Normalizer"), the German repeats nothing either:
    "Aktivieren (Shift+N)".
- **Words with two meanings in the source.** A file's extension is
  "Dateiendung" (the association table's "Extension", sorting by
  extension); extending the picture is "Erweitern" / "Erweiterung"
  ("Extend/Crop" "Erweitern/Zuschneiden"). A filter or effect is
  "angewendet", a changed setting "übernommen". "Remember" is "merken",
  "store" and "save" "speichern". The "nav bar" and "navigation bar" of
  PotPlayer's menus are the seek bar ("Zeitleiste"). "Insert" in the
  subtitle browser (a new line) is "Einfügen", as Paste is; they never
  meet in one window. "Default" as a value is "Standard"; "RVP
  Default", the skin's name, stays.
- **Recurring labels**, settled during the translation:
  - "Start" is "Start" everywhere, as a button too; "Stop" is "Stopp".
  - "Up" / "Down" buttons are "Nach oben" / "Nach unten".
  - "Cycle X" is "X durchschalten".
  - "On Screen Control" (PotPlayer's page and item for the skin's bars
    over the video) is "Leisten über dem Video", never
    "Bildschirmsteuerung", which reads as controlling the monitor.
  - "AI" is "KI" ("Whisper-KI-Modell").
  - "Keep Video Frame" is "Videobild ... halten", never "festhalten",
    which reads as freezing the picture.
  - The video's "Display AR" (the aspect ratio the file gives) is
    "Anzeige-Seitenverhältnis", not the screen's.
  - "none" that RVP inserts into other strings is "nicht vorhanden", which
    agrees with any noun.
  - Choices in a list take no article ("Eigene Größe", "Letzte
    Position"); a full sentence after a colon starts with a capital.
  - "Seamless Playback" (joining files without a gap) is "Lückenlose
    Wiedergabe".
  - "X Settings..." in the menus is "X-Einstellungen..." when the
    compound stays short ("Skin-Einstellungen...",
    "Wiedergabe-Einstellungen..."), and "Einstellungen der X..."
    otherwise ("Einstellungen der Videoverarbeitung...").
  - Increase / Decrease are "erhöhen" / "verringern" for values (volume,
    brightness, sync, speed) and "vergrößern" / "verkleinern" for sizes
    and margins.
  - "... step" (a slider's increment) is "Schrittweite"; under a
    "Synchronisation" heading the plain "Schrittweite (< und >)" is enough.
  - The list of recent items is "Verlauf" ("Verlauf speichern und
    anzeigen", "URL-Verlauf").
  - The "Video Capture" page and menu are "Aufnahme"; the "Recording"
    heading on that page is "Aufzeichnung".
  - Screen frame / source frame are "Bildschirmbild" / "Quellbild".
  - Deblock, Deband and Soften are "Deblocking", "Debanding" and
    "Weichzeichnen"; a subtitle's box is "Box".
  - RVP's state folder is "Zustandsordner", the home folder
    "Benutzerordner".
  - "Sync" stands alone only in the page names "Sprache/Sync/Sonstiges"
    and the menu items that open them; elsewhere it is "Synchronisation".
  - "Every" before a time is "Alle" ("Alle (Sekunden)").
  - The desktop entry's GenericName is "Mediaplayer".
- **Genders of the loanwords**, kept the same throughout: der Skin (die
  Skins), die Playlist (die Playlists), das Album (die Alben), der
  Stream, die Engine (die Engines), das Token, der Tooltip, der
  Renderer, der Equalizer, der Codec, der Decoder, der Shader, das
  Plugin, das Skript, der Screenshot, der Screenreader, der Timer, der
  Controller, das Gamepad, die Disc (die Discs), das OSD, die Letterbox,
  das Pass-Through, die URL.
- **Equalizer presets and anaglyph modes.** Genre names take the German
  form where one is usual and stay otherwise ("Klassik", "Pop", "Rock",
  "Club", "Dance", "Techno", "Ska", "Reggae", "Party", "Live"); the
  descriptive ones are German ("Großer Saal", "Volle Bässe", "Sanft").
  Anaglyph colours are German ("Rot/Cyan", "Grün/Magenta", "Gelb/Blau",
  "grau", "farbig", "halbfarbig"); "Dubois" stays.

## Glossary

| English | German | Note |
|---------|--------|------|
| play / pause / stop | abspielen / Pause / stoppen | Commands "Abspielen", "Pause", "Stopp"; the verb for pause is "pausieren". In prose "wiedergeben" for play is fine. Never "anhalten", which reads as both pause and stop. |
| playback | Wiedergabe | The menu and page; compounds "Wiedergabeposition", "Wiedergabe-Einstellungen". |
| close playback | Wiedergabe beenden | Stops and unloads the file. "Exit" (RVP itself) is "Beenden". |
| previous / next | vorheriger / nächster | The ending agrees with the noun: "vorheriges Kapitel", "nächste Datei", "nächster Eintrag". "Zurück" and "vor" are seek directions, not previous and next. |
| seek | spulen | "vorspulen", "zurückspulen", the noun "Spulen" ("Beim Spulen mit der Maus pausieren"). Never "suchen", which is search. |
| jump | springen | Noun "Sprung", "Zeitsprung"; "Zu Zeit/Bild springen...". Windows' jump list is "Sprungliste". |
| frame step | Bild für Bild | The commands are "Nächstes Bild" / "Vorheriges Bild". |
| speed | Geschwindigkeit | The playback rate; not "Rate", not "Tempo". |
| repeat | wiederholen | Noun "Wiederholung"; "Wiedergabe wiederholen". Redo is "Wiederherstellen", never "Wiederholen". |
| A-B repeat / section repeat | A-B-Wiederholung / Abschnittswiederholung | Hyphens between A, B and the noun; the marks are "Punkt A" / "Punkt B". |
| shuffle | Zufallswiedergabe | In a sentence also "in zufälliger Reihenfolge". |
| queue | Warteschlange | "Zur Warteschlange hinzufügen"; the playback queue is just "Warteschlange". |
| resume / remember position | fortsetzen / Position merken | The page "Fortsetzen"; "Wiedergabepositionen merken". |
| position | Position | |
| duration | Dauer | Not "Länge", which is a length in general ("Intro length" is "Länge des Vorspanns"). |
| elapsed / remaining time | verstrichene / verbleibende Zeit | Alone as choices "Verstrichen" / "Verbleibend". |
| chapter | Kapitel | Plural the same. |
| bookmark | Lesezeichen | Plural the same. |
| skip (intro, ending) | überspringen (Vorspann, Abspann) | "Vorspann %1 s überspringen", "Abspann überspringen"; the skip page is "Überspringen". |
| seek bar | Zeitleiste | Also PotPlayer's "nav bar" and "navigation bar". |
| thumbnail | Miniatur | Plural "Miniaturen"; the generated sheet of thumbnails is "Miniaturbild". |
| preview | Vorschau | The picture over the seek bar; "Vorschau-Miniaturen" where the source says thumbnail previews. |
| scene browser | Szenenübersicht | The window of frames across the file. |
| file | Datei | "Datei(en)" for file(s). |
| folder | Ordner | Never "Verzeichnis". |
| URL / address | URL / Adresse | die URL. |
| stream | Stream | Network media and recording ("Stream aufnehmen"). A track inside a file is "Spur" ("Audiospur", "Videospur"), the same word as track. |
| device / capture device | Gerät / Aufnahmegerät | "Webcam/anderes Gerät". |
| screen capture | Bildschirmaufnahme | The desktop as a source; not frame capture. |
| disc | Disc | die Disc, plural "Discs"; never "Disk", which is a hard disk. |
| title | Titel | A disc's title; an entry's title is "Titel" too. |
| angle | Kamerawinkel | "Kamerawinkel-Menü". |
| disc menu | Disc-Menü | "DVD-Menü", "Blu-ray-Menü" where the source names the disc. |
| drag and drop | Ziehen und Ablegen | Dropped files are "abgelegte Dateien". |
| recent files | zuletzt geöffnete Dateien | The list of recent items is "Verlauf" ("URL-Verlauf"). |
| favorites | Favoriten | |
| file association / file type | Dateizuordnung / Dateityp | The page is "Zuordnung". |
| playlist | Playlist | die Playlist; not "Wiedergabeliste" (long, and it repeats "Wiedergabe" in the playback texts). |
| album | Album | Plural "Alben"; a playlist tab, not a music album. Album art is "Albumcover". |
| entry | Eintrag | Plural "Einträge"; also the source's playlist "item". |
| sort | sortieren | "Sortieren nach". |
| add / remove / delete | hinzufügen / entfernen / löschen | A playlist entry is "entfernt", a file on disk "gelöscht": never swap them. Removing a bookmark, an album or a key is "löschen" where the source says delete. |
| recycle bin / trash | Papierkorb / Papierkorb | The word of Windows and of the Linux desktops alike. |
| import / export | importieren / exportieren | |
| window / main window | Fenster / Hauptfenster | |
| fullscreen / windowed | Vollbild / Fenstermodus | "Vollbildmodus" where a mode is meant. |
| maximize / minimize | maximieren / minimieren | |
| always on top / stay on top | immer im Vordergrund / im Vordergrund | The window above others; not "oben". |
| opacity | Deckkraft | Transparency (the opposite) is "Transparenz". |
| skin | Skin | der Skin, plural "Skins". |
| color theme | Farbschema | A skin's colour variant. |
| logo | Logo | |
| title bar | Titelleiste | |
| control panel | Bedienfeld | RVP's F7 window; never "Systemsteuerung", which is Windows' Control Panel. |
| OSD / on-screen display | OSD / Bildschirmanzeige | "OSD-Meldungen" for OSD messages. |
| on-screen controller | Bedienleiste | mpv's OSC and scripts such as uosc; "Bedienleiste von mpv". Not "Controller", which is the gamepad. |
| tooltip | Tooltip | der Tooltip, plural "Tooltips". |
| context menu | Kontextmenü | |
| tray icon / notification area | Tray-Symbol / Infobereich | The tray alone is "Infobereich" ("Taskleiste und Infobereich"). |
| taskbar | Taskleiste | On Linux the dock is "Dock". |
| notification | Benachrichtigung | |
| dialog | Dialog | der Dialog, plural "Dialoge"; "dialog box" is "Dialog" too. |
| settings / preferences | Einstellungen / Einstellungen | The window and the menu item "Einstellungen...". |
| page | Seite | "Seite „Wiedergabe“". |
| preset / configuration | Voreinstellung / Konfiguration | A saved set of settings; equalizer and frame size presets too. The smoothing engines' presets are "Qualitätsstufe". |
| reset / default | zurücksetzen / Standard | Buttons "Zurücksetzen", "Standard"; "(Standard)", "Systemstandard", "mpv-Standard"; a default value is "Standardwert". |
| apply | übernehmen | The button "Übernehmen". A filter is "angewendet". |
| key / hotkey / shortcut | Taste / Tastenkürzel / Tastenkürzel | Key names stay as in RVP's menus (see Style). |
| mouse / wheel / button | Maus / Mausrad / Maustaste | A button on screen is "Schaltfläche"; pointer "Mauszeiger". |
| gesture | Geste | "Mausgeste". |
| touch | Touch | "Touch-Bedienung", "Touch-Layout". |
| controller / gamepad | Controller / Gamepad | The Xbox controller. |
| video / picture / image | Video / Bild / Bild | A still image file is "Bilddatei". |
| frame | Bild | "Einzelbild" where it must stand apart from the picture; a keyframe is "Schlüsselbild". Not "Frame". |
| frame rate / fps | Bildrate / fps | Written out "Bilder pro Sekunde". |
| aspect ratio | Seitenverhältnis | The source's "AR" is written out; "DAR" stays. |
| crop | zuschneiden | "Erweitern/Zuschneiden". |
| zoom / pan | Zoom / verschieben | "zoomen"; Zoom In / Zoom Out are "Hineinzoomen" / "Herauszoomen". |
| rotate / flip / mirror | drehen / spiegeln / spiegeln | "Vertikal spiegeln" (upside down), "Horizontal spiegeln" (left-right); rotation is "Drehung". |
| deinterlace | Deinterlacing | The usual German word; "Deinterlacing (Ctrl+I)". |
| brightness / contrast / saturation / hue / gamma | Helligkeit / Kontrast / Sättigung / Farbton / Gamma | |
| sharpen / denoise | schärfen / entrauschen | Nouns "Schärfen", "Entrauschen" ("3D-Entrauschen"); sound's noise reduction is a different word. |
| scaler / upscaling / resize | Skalierung / Hochskalierung / Größe ändern | Upscaler / Downscaler / Chroma scaler: "Hochskalierung" / "Herunterskalierung" / "Chroma-Skalierung"; the noun of resize is "Größenänderung". |
| pixel shader | Pixel-Shader | der Shader, plural the same. |
| renderer | Renderer | "Video-Renderer", "Audio-Renderer". |
| hardware decoding / decoder | Hardware-Decodierung / Decoder | Spelled with c throughout: "decodieren", "Codierung", "Codec". |
| HDR | HDR | |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360°-Video / Anaglyph / nebeneinander / übereinander | "3D-Video", "360°-Videomodus"; SBS and TAB stay as abbreviations beside the German. |
| bars / letterbox / margin | Balken / Letterbox / Rand | "schwarze Balken"; "Unterer Rand", "Seitenrand". The skin's bars (its controls over the video) are "Leisten", not "Balken". |
| frame capture / screenshot | Bildaufnahme / Screenshot | Saving the current frame; the saved files are "Bilder". Not screen capture. |
| motion smoothing | Bewegungsglättung | |
| frame interpolation | Zwischenbildberechnung | The word German TVs use; "interpolieren" for the verb. |
| engine | Engine | die Engine; also whisper's engines and TensorRT's built engine. |
| calibration | Kalibrierung | "kalibrieren", "Neu kalibrieren…". |
| quality preset (light, medium, high) | Qualitätsstufe (leicht, mittel, hoch) | The presets' labels Fast / Medium / Quality are "Schnell" / "Mittel" / "Qualität"; a lighter step of the ladder is a lower "Stufe" ("herunterstufen"). |
| GPU / graphics card | GPU / Grafikkarte | "die Karte" for short. |
| processor / CPU | Prozessor / CPU | |
| power budget | Leistungsbudget | Electrical power in watts: write "Leistungsaufnahme" where "Leistung" could read as performance. |
| dropped frames | ausgelassene Bilder | Frames not shown in time. |
| refresh rate | Bildwiederholrate | The display's rate in Hz. |
| audio / sound | Audio / Ton | "Ohne Ton"; the Audio menu and page are "Audio". |
| volume / mute | Lautstärke / Stumm | The verb is "stummschalten"; the command "Stumm". |
| master volume | Hauptlautstärke | The system's own volume. |
| equalizer | Equalizer | der Equalizer. |
| normalizer | Normalisierung | "vor der Normalisierung"; TomSteady is the method's name. |
| reverb | Hall | "Halleffekt"; Freeverb's room size "Raumgröße", damping "Dämpfung". |
| noise reduction | Rauschunterdrückung | Of the sound only; the picture is "entrauschen". |
| channel / speaker | Kanal / Lautsprecher | Plural "Kanäle". |
| downmix | Downmix | der Downmix; the verb "heruntermischen". |
| surround / stereo | Surround / Stereo | |
| output device | Ausgabegerät | |
| sample rate / bitrate | Abtastrate / Bitrate | |
| audio delay / audio sync | Audioverzögerung / Audiosynchronisation | "Synchronisation" alone inside the Sync sections. |
| pitch / time stretching | Tonhöhe / Time-Stretching | Pitch shifting is "Tonhöhenverschiebung". |
| audio recording | Audioaufnahme | |
| second audio device | zweites Audiogerät | "2. Gerät" where the source abbreviates. |
| pass-through | Pass-Through | das Pass-Through; "Pass-Through nach AC3-Neucodierung". |
| subtitles / subtitle track | Untertitel / Untertitelspur | Plural the same. |
| external / embedded subtitles | externe / eingebettete Untertitel | |
| font / outline / shadow | Schriftart / Kontur / Schatten | "Schriftgröße", "Konturbreite"; a border is "Rahmen". |
| subtitle position / subtitle delay | Untertitelposition / Untertitelverzögerung | |
| encoding | Codierung | A text file's encoding is "Zeichencodierung" or "Textcodierung"; re-encoding is "Neucodierung". |
| subtitle browser / subtitle editor | Untertitelübersicht / Untertiteleditor | |
| subtitles from audio / speech recognition | Untertitel aus Audio / Spracherkennung | |
| model | Modell | whisper's speech model. |
| language | Sprache | |
| remote control | Fernsteuerung | The remote programs themselves are "Fernbedienungen". |
| token | Token | das Token. |
| port / server | Port / Server | |
| log | Protokoll | "rvp.log" stays a file name. |
| crash | Absturz | "abstürzen". |
| download / install | herunterladen / installieren | |
| installer | Installationsprogramm | |
| accessibility | Barrierefreiheit | |
| text to speech (TTS) / screen reader | Sprachausgabe (TTS) / Screenreader | |
| screen saver | Bildschirmschoner | |
| shut down / sleep / hibernate | Herunterfahren / Standby / Ruhezustand | The computer's power actions. |
| timer | Timer | |
| visualization / spectrum | Visualisierung / Spektrum | |
| status bar / info | Statusleiste / Info | "Informationen" where a sentence needs it. |
| codec / format | Codec / Format | |
| bit depth / 10-bit | Bittiefe / 10 Bit | As part of a compound "10-Bit-Ausgabe". |
| channel layout (5.1, 7.1) | Kanalanordnung (5.1, 7.1) | |
| Auto select (recommended) | Automatisch (empfohlen) | "Auto select" alone is "Automatisch". |
