# Turkish (tr)

Standard Turkish as written in Turkey (Türkiye Türkçesi), spelled by the
Turkish Language Association's (TDK) rules, with every Turkish letter
(ç, ğ, ı, İ, ö, ş, ü) and never an ASCII stand-in. The translation speaks
to the user formally, as the Windows, GNOME and KDE desktops do in
Turkish, but most strings need no address at all: commands are bare verb
stems, the way Turkish desktops write "Kaydet" and "Aç"; headings are
noun phrases; help texts describe what RVP does, as the English does. The
aim is the everyday vocabulary of a Turkish desktop media player
(oynatma listesi, altyazı, ilerleme çubuğu, Geri Dönüşüm Kutusu), Turkish
words where Turkish users know them, established loanwords only where no
Turkish word is usual (video, logo, model, port, kodek, ekolayzer), and
labels short enough for the settings window's 240-pixel labels and
300-pixel choice lists, which agglutinated Turkish words, unable to
wrap, can overflow.

## Style

- **Address.** Formal "siz", always lowercase. Use it only where the
  source speaks to the user ("you download", "press Enter"): the
  imperative with -(y)In ("Bir altyazı yazın ve Enter tuşuna basın"),
  the possessive with -(I)nIz ("kendi tuşlarınız"). Never the singular
  "sen" in a sentence, never the old-fashioned -(y)InIz ("basınız"),
  never "lütfen" in labels; in help texts only where a real request is
  made. Elsewhere write impersonally, as the source does ("RVP ölçer",
  "Kalibrasyon bir ile üç dakika sürer").
- **Help texts** are full sentences. General behaviour takes the aorist
  ("RVP PotPlayer'ın .dpl dosyalarını okur", "Altyazılar videonun altına
  gelir"), or its passive ("... okunur"); never the bureaucratic
  -mAktAdIr. A long English sentence may become two Turkish ones, each
  with its verb last.
- **Notices and progress.** What happened takes the passive past:
  "Uygulandı.", "İptal edildi.", "İçe aktarıldı.", "%1 kaldırıldı",
  "Henüz ölçülmedi."; "Unchanged." is "Değişiklik yok.". What is going on
  takes the passive present continuous: "Başlatılıyor…", "%1 ölçülüyor…",
  "indiriliyor". Confirmation questions take the passive optative with
  "mı/mi/mu/mü", written apart: "%1 dosya Geri Dönüşüm Kutusu'na
  gönderilsin mi?", "Tüm eylemlere yeniden PotPlayer tuşları atansın
  mı?". The answer buttons are commands: "Taşı", "Koru".
- **Menu items and buttons** are commands: the object first, the bare
  verb stem last ("Klasör ekle...", "Yer imi ekle", "Oynatma listesini
  farklı kaydet..."). An object the English leaves general takes no case
  ending ("Dosya aç...", "URL ekle..."), which is also shorter; a
  definite one takes the accusative ("Geçerli kareyi kaydet"). Where the
  English label is a noun ("Playback", "Audio Recording", "Aspect
  Ratio"), Turkish uses a noun ("Oynatma", "Ses kaydı", "En boy oranı").
  The desktops' own words: OK "Tamam", Cancel "İptal", Close "Kapat",
  Apply "Uygula", Save "Kaydet", Save As "Farklı kaydet", Open "Aç",
  Browse "Gözat", Choose "Seç", Exit "Çıkış", About "Hakkında", Tools
  "Araçlar", Misc "Çeşitli", Other "Diğer", Copy "Kopyala", Paste
  "Yapıştır", Undo "Geri al", Redo "Yinele" (never repeat), Select All
  "Tümünü seç", Invert Selection "Seçimi tersine çevir", Find "Bul",
  Rename "Yeniden adlandır", Duplicate "Çoğalt", Show / Hide "Göster" /
  "Gizle", Customize "Özelleştir", Restore "Geri yükle". PotPlayer's
  "Cycle X" (move to the next one) is "X değiştir" ("Ses akışını
  değiştir"), "Select X" "X seç"; Increase / Decrease "artır" / "azalt",
  Zoom In / Out "Yakınlaştır" / "Uzaklaştır", Move Up / Down "Yukarı
  taşı" / "Aşağı taşı", Forward / Backward "ileri" / "geri".
- **Checkboxes** take the same command form, verb last ("Oynatma
  konumlarını hatırla", "Araç ipuçlarını göster", "Oynatırken ekran
  koruyucuyu engelle"). Enable / Disable as commands are "etkinleştir" /
  "devre dışı bırak"; Use / Do not use "kullan" / "kullanma". A checkbox
  whose English is a noun phrase may stay one.
- **Choices** in lists and menus are noun phrases, adjectives or adverbs:
  "Hiçbir zaman", "Her zaman", "Oynatırken", "Yüksek kalite", "Otomatik
  (önerilen)", "Rastgele". On / Off, and Enable / Disable or Enabled /
  Disabled when they are choices or states, are "Açık" / "Kapalı" as
  Windows' switches say ("Kapalı (önerilen)"); "Etkin" / "Devre dışı"
  only where the source contrasts a feature's state in a sentence. None
  is "Yok", Nothing "Hiçbiri", Auto "Otomatik", Custom "Özel", Default
  "Varsayılan". "(Recommended)" and PotPlayer's "(Recom'd)" are always
  "(önerilen)", "(Default)" "(varsayılan)".
- **Titles** of windows, pages, groups, columns and field labels are noun
  phrases, with the verbal noun in -mA where the English is a gerund:
  "Oynatma", "Zaman esnetme", "Sahne tarayıcısı", "Yazı tipi boyutu".
  "Section Repeat" is "Aralık tekrarı", not a command. A window that a
  command opens keeps the command's words without the ellipsis, as the
  source does. A label before a field ends with a colon only where the
  source has one.
- **Capitalization** is sentence case everywhere, menus, buttons, window
  and page titles included, as GNOME, KDE and the Settings app of
  Windows 11 write Turkish: only the first word and names take a capital
  ("Oynatma ayarları...", "İlerleme çubuğunda küçük resim önizlemelerini
  göster"). The English Title Case is never copied. After a slash or a
  colon the next word is lowercase ("Oynat/duraklat", "Eylem: uyku"),
  names apart. Proper names keep their capitals: Windows' "Geri Dönüşüm
  Kutusu", language names ("İngilizce"), and RVP's names (RVP Default).
  Names keep their own case even at the start (mpv, libmpv, yt-dlp,
  whisper.cpp, dav1d).
- **The dotted and dotless i.** Turkish has two letters: i / İ and
  ı / I. A word that begins with i takes İ as its capital ("İptal",
  "İçe aktar", "İleri sar", "İndir", "İlişkilendirme", "İkinci ses
  aygıtı", "İngilizce"), one that begins with ı takes I ("Işık"); never
  "Iptal" or "ıleri". Abbreviations and names keep their Latin letters
  as written (IPC, UIA, Intel, XInput). Never change a string's case with
  a tool that does not know Turkish rules: Python's and Rust's plain
  lower() turn "İ" into "i" with a combining dot and "I" into "i".
  `work.py compare` lowercases that way, so its glossary flag on a word
  that starts with İ or I may be a false alarm: check the word itself.
- **Quotation marks** are “…” (‘…’ inside them), both for the source's
  “…” and for its straight "...". When a text names another label, page
  or menu, it quotes that label's Turkish text exactly and lets a noun
  after the quotation carry the case ending: “Yeni açılan videoya
  pencereyi sığdır” seçeneği açıkken (Oynatma sayfası), “%1” eylemine.
  TDK writes an ending after a closing quotation mark without an
  apostrophe (“Oynatma”da); the noun after it avoids the question. Menu
  paths stay unquoted, with the Turkish labels and the source's ">".
- **Punctuation.** No space before ":", ";", "?", "!" or ")". No comma
  before "ve", "ile", "veya", "ya da" in a list; a comma may follow a
  sentence's subject or an introductory clause where it helps reading,
  as TDK allows. A dash that sets off a phrase is " – " (an en dash with
  spaces). "&" becomes "ve" ("Tam bas ve tiz"), or stays "&" in a tight
  label; "/" stays between alternatives ("Ekle/Düzenle").
- **"..." and "…".** The end of a menu item or button keeps exactly
  what the source has: three periods "..." where the source has them
  (the menus), the single character "…" where it has that (the QML
  windows), attached to the last word without a space ("Aç...",
  "Gözat…"). Turkish word order puts the verb, and so the dots, last.
  Where the source has a space before them ("Shutdown at ...") the space
  stays.
- **Keys** keep RVP's own names, the ones its menus print, even where
  Turkish keyboards label them differently: Ctrl, Shift, Alt, Enter,
  Space, Esc, Tab, PgUp, PgDn, Home, End, Del, Ins, F1...F12, Num+, the
  arrow keys' Left, Right, Up and Down inside key names and lists of
  keys ("Ctrl+Left / Right", "Up, Down, tekerlek"), and the letters and
  punctuation keys as in the source. In running text about keys in
  general, Turkish words: "ok tuşları", "klavyenin ses tuşları". A case
  ending goes on the word "tuşu" after the key's name, never on the name
  ("Enter tuşuna basın", "Esc tuşu iptal eder"); a key combination as a
  subject needs no ending ("Shift+Enter yeni satır başlatır"). The
  gamepad's buttons (A, B, X, Y, Back, Start) and the mouse's X1 and X2
  keep their labels; the mouse's back and forward buttons are "Geri" and
  "İleri".
- **Units.** Seconds "sn", minutes "dk", hours "sa", milliseconds "ms";
  "Sec", "Sec.", "Min" and "ms" in the menus become "sn", "sn", "dk" and
  "ms" ("%1 sn geri", "Açılışta %1 sn atla"). dB, Hz, kHz, fps, px, K, p
  (1080p), GB, MB and W stay. A space separates a number and its unit,
  also where the source writes "%1ms" or "%1Hz": "%1 ms", "%1 Hz",
  "%1 dB", "%1 GB". Degrees stay attached ("%1°", "360°"). Written out in
  a sentence: "saniye", "dakika", "saniyede kare" ("saniyede 60 kare").
  The multiplier stays "x" ("%1x", "2x").
- **The percent sign comes before the number, attached**, as Turkish
  writes it ("%50"). A placeholder's number therefore becomes "%%1": the
  first "%" is the sign, the "%1" after it the placeholder, which Qt's
  arg() and RVP's menu code fill in as usual ("%1 %" and "%1%" are
  "%%1"; "Decrease Brightness by %1%" is "Parlaklığı %%1 azalt"; "(%4 %)"
  is "(%%4)"). A literal percentage in running text is written "yüzde 5",
  never "%5": that would read as placeholder %5 and fail
  `tests/translations.rs`. A case ending after a percentage depends on
  how the number is read ("%50'si", "%40'ı"): never put one after
  "%%1"; put the percentage after a colon or before a noun, or use
  "kadarı" ("video alanının %%1 kadarı").
- **Numbers.** In prose the decimal separator is a comma ("1,5 sn",
  "0,5"); a thousands separator, rarely needed, is a period. Notation
  that is a name or that RVP parses keeps the source's form: channel
  layouts (5.1, 7.1), aspect ratios and sizes (16:9, 2.35:1, %1:%2,
  %1.%2, 3x2, %1x%2), versions, ranges such as 0-255 and 16-235, and
  input formats ("h:mm:ss.s", "hh:mm", "h:mm"). Ordinals are the number
  with a period: "%1nd Device" is "%1. aygıt", "{Same as %1st logo}"
  "{%1. logo ile aynı}", "2nd" "2.". Bits: "16 bit", "%1 bit
  (önerilen)", "10 bit çıkış". After a number, and after "birkaç",
  "her", "kaç", a noun stays singular: "%1 dosya", "%1 satır", "3
  dakika"; the source's "file(s)", "line"/"lines" and "chapter(s)" are
  one Turkish form ("%1 satır ayır" for both "Reserve %1 line" and
  "Reserve %1 lines").
- **Endings after names.** A name takes its endings after an apostrophe,
  in the vowel harmony of how Turkish speakers say it; an abbreviation
  read letter by letter follows its last letter's Turkish name (pe, de,
  re, le, u...). The usual forms: RVP'nin, RVP'yi, RVP'ye, RVP'de,
  RVP'den; mpv'nin, mpv'ye; SVP'nin; PotPlayer'ın, PotPlayer'a,
  PotPlayer'da; Windows'un, Windows'ta; Linux'un, Linux'ta; NVIDIA'nın,
  NVIDIA'dan; FFmpeg'in; VapourSynth'in; MVTools'un; TensorRT'nin;
  CUDA'nın; Vulkan'ın; yt-dlp'nin; DVD'nin, DVD'yi; GPU'nun, GPU'da;
  CPU'nun, CPU'da; HDR'nin, HDR'ye; URL'nin, URL'yi; AMD'nin. Where the
  reading is uncertain (Qt, RIFE, OpenGL, Direct3D, Wayland, whisper.cpp,
  RVP Default, BlackSpace) or the word would grow long, put a common
  noun after the name and give it the ending: "RIFE motoru", "Qt yazı
  tipi", "OpenGL işleyicisi", "Wayland masaüstünde", "BlackSpace
  kaplaması", "whisper.cpp programı". English possessives of names
  mostly become compounds without the genitive, which are shorter and
  need no ending on the name: "RVP's settings" "RVP ayarları",
  "PotPlayer's skins" "PotPlayer kaplamaları", "mpv's default" "mpv
  varsayılanı", "Windows' default" "Windows varsayılanı"; the genitive
  only where ownership matters ("RVP'nin kendi kaplaması"). Common nouns
  never take an apostrophe ("portu", "kodeği").
- **Placeholders and endings.** %1 may stand for a word, a name, a
  number, a date or a path, so its sound is unknown and no ending may be
  attached to it: never "%1'i", "%1'de", nor double forms such as
  "%1'(n)in". Instead:
  - put it after a colon ("Durum: %1", "Kullanılan: %1", "Süre: %1",
    "Ölçüm tarihi: %1.");
  - make it the subject, which takes no ending ("%1 ölçülüyor…",
    "%1 kaldırıldı", "%1 burada çalışmıyor");
  - use a postposition that takes the bare form, written as a separate
    word: "için", "ile", "gibi" ("“%1” (%2) için bir tuşa basın";
    "ile" never as the suffix -la/-le after %1);
  - let a noun after it carry the ending: "%1 tuşu", "“%1” eylemine",
    "“%1” sayfasının seçenekleri", "%1 kartı için", "%1 sürümü",
    "%1 kalibrasyonu";
  - a number before its noun needs no ending ("%1 dosya", "%2 saniye
    içinde").
  An ending after a fixed letter or unit that follows the placeholder
  depends on that letter only and is safe ("%3p'ye kadar": p is "pe"),
  but "en çok %3p" avoids even that. Words that RVP inserts into another
  string (status words such as "checking", "unpacking", "downloading")
  must read right there: "RIFE (TensorRT): %1, %2/%3 GB" takes
  "denetleniyor", "açılıyor", "indiriliyor".
- **Word order with several placeholders.** Turkish puts the verb last
  and the modifier before the modified, so placeholders often move;
  each stays exactly once. "%1 is used by “%2”. Move it to “%3”?" is
  "%1, “%2” eylemine atanmış. “%3” eylemine taşınsın mı?"; "Calibration
  of %1 at 60 frames a second: %2" "%1 kalibrasyonu (saniyede 60 kare):
  %2"; "%1 done, at most %2 left" "%1 tamamlandı, en çok %2 kaldı". "X of
  Y" amounts are written with a slash: "%2 of %3 GB" "%2/%3 GB", "%1 %2 W
  of %3 W (%4 %)" "%1: %2 W / %3 W (%%4)"; "%1 %2 % of its power" "%1:
  güç kullanımı %%2". Line breaks stay where the source has them ("%1\n%2
  saniye içinde").
- **Values stay as they are.** Example and default values that RVP
  matches or parses keep the source's text: chapter titles to skip
  ("Opening;Ending;3"), language codes ("rus,eng", "en,ru", "en"),
  drive letters and mount points ("V:", "/mnt/media"), the yt-dlp format,
  file patterns ("*.dsf", "*.json"), the default folders RVP really uses
  ("Music/RVP"; "Pictures/RVP in the home folder" is "ev klasöründe
  Pictures/RVP"), and the playlist's album named "Default" (data, shown
  as “Default”: "“Default” albümünü hatırla"). Whisper's model names
  (tiny, base, small, medium, large) stay; their notes are Turkish
  ("(indirilmedi)", "(kurulu değil)").
- **Language names** are capitalized, as TDK writes them, in their -cA /
  -çA form ("Arnavutça", "Almanca", "İngilizce", "İspanyolca",
  "Felemenkçe", "Haiti Kreolcesi", "Türkmence"), though the source writes
  them in lowercase; a language without a usual -cA form takes "dili"
  ("Lingala dili"). The language's own name (the Language context) is
  "Türkçe".
- **Short labels.** Turkish runs about a fifth to a third longer than
  English, and one agglutinated word ("gölgelendiricilerini") cannot
  wrap.
  - Drop what the English keeps for grammar: articles ("bir" only where
    it matters), possessives the context gives ("Show the mouse pointer"
    "Fare imlecini göster"), "olarak" ("otomatik olarak" is "otomatik").
  - Use a bare object, a compound instead of a genitive chain
    ("altyazı yazı tipi", not "altyazının yazı tipi"), and endings
    instead of clauses: -ken for "while" ("Oynatırken", "Tam
    ekrandayken", "Simge durumundayken"), -sA for "when/if"
    ("Altyazı varsa", "Giriş daha küçükse"), participles for "that"
    clauses ("Yeni açılan videolar").
  - Prefer the shorter common word: "göster" (not "görüntüle"), "seç",
    "aç", "kapat", "sil", "kullan", "en çok" / "en az".
  - Accepted abbreviations in tight labels: "sn", "dk", "sa", "maks.",
    "örn.", "vb."; no others.
  - An explanation in parentheses may be shortened when the label stays
    clear. Where the source's label repeats nothing of its heading
    ("Enable (Shift+N)" under "Normalizer"), Turkish repeats nothing
    either: "Etkinleştir (Shift+N)".
- **Words with two meanings.** Each Turkish word has one job in RVP:
  - "kaydet" is both save and record: "Save" is "Kaydet", recording is
    "kayıt" ("Ses kaydını başlat/durdur", "Akışı kaydet..."); "store"
    and "keep" are "sakla" ("Yakalanan resimleri oynatılan klasörde
    sakla"); RVP registering itself for file types is "kendini ...
    için kaydeder".
  - "kapat" is Close, turn off and shut down; Exit is "Çıkış" (in
    phrases "çık": "Oynatma bitince RVP'den çık"); the power action
    reads "Bilgisayarı kapat" where Close could be meant.
  - "yükle" is load ("Altyazı yükle..."); install is "kur" (installed
    "kurulu", installation "kurulum").
  - "sürücü" is both a drive and a driver, as on Windows; the context
    tells them apart ("DVD sürücüsü", "ekran kartı sürücüleri").
  - "atla" is skip only; jump is "git". "sar" is seek; "ara" is search
    only.
  - "sıra" is order ("Artan sıra"); the queue is "kuyruk". "Oynatma
    sırasında" (during playback) is "oynatırken", so it never reads as
    an order.
  - "bölüm" is chapter only; a section is "aralık", a part of the
    window "alan", an episode never comes up.
  - "son" is last or recent ("Son dosyalar", "son konumunda"); a file's
    ending is "kapanış".
  - "giriş" is input ("Giriş (kaynak)", "Giriş daha küçükse"), output
    "çıkış"; the intro is "açılış".
  - "geçiş" is the slideshow's transition only.
  - A file's extension is "uzantı"; extending the picture is
    "genişlet" ("Genişlet/Kırp"). "Insert" in the subtitle browser (a
    new line) is "Araya ekle", Add "Ekle".
  - "Frame" is "kare"; a window's or picture's frame "çerçeve". "Track"
    is "parça" ("ses parçası"), "stream" "akış".
  - "Default" as a value is "Varsayılan"; "RVP Default", the skin's
    name, stays.
  - "Control" is "denetim" ("Ses düzeyi denetimi", "Düzey denetimi"),
    except RVP's "kontrol paneli"; to check is "denetle".
  - Other words of RVP's: action "eylem", option "seçenek", status
    "durum", mode "mod", level "düzey", step "adım", item "öğe",
    script "betik" (VapourSynth's and mpv's; an ASS "script" is an ASS
    subtitle file), plugin "eklenti", filter "filtre", instance "örnek"
    ("Birden çok örnek"), focus "odak", mouse pointer and cursor "fare
    imleci", clipboard "pano", keyframe "anahtar kare", marker "işaret",
    luma and chroma "luma" and "kroma", input "giriş", output "çıkış",
    runtime "çalışma zamanı", endpoint "uç nokta", buffer "arabellek",
    thread "iş parçacığı", cut (a scene's) "kesme", desktop
    "masaüstü", home folder "ev klasörü", state folder "durum klasörü",
    licence "lisans", album art "albüm kapağı", slideshow "slayt
    gösterisi", mixer "karıştırıcı", gain "kazanç", bass / treble "bas" /
    "tiz", preamp "ön yükseltme".
- **Equalizer presets and anaglyph modes.** Genre names take the Turkish
  form where one is usual and stay otherwise ("Klasik", "Pop", "Rock",
  "Kulüp", "Dans", "Tekno", "Ska", "Reggae", "Parti", "Canlı", "Soft
  rock"); the descriptive ones are Turkish ("Büyük salon", "Tam bas",
  "Tam tiz", "Tam bas ve tiz", "Yumuşak", "Dizüstü"). Anaglyph colours
  are Turkish ("Kırmızı/camgöbeği", "Yeşil/macenta", "Sarı/mavi",
  "Kırmızı/mavi", "Kırmızı/yeşil"; "gri", "renkli", "yarı renkli");
  "Dubois" stays.

## Glossary

| English | Turkish | Note |
|---------|---------|------|
| play / pause / stop | oynat / duraklat / durdur | Commands "Oynat", "Duraklat", "Durdur"; Play/Pause "Oynat/duraklat". States "oynatılıyor", "duraklatıldı", "durduruldu". Never "yürüt" in some strings and "oynat" in others, and never "durdur" for pause. |
| playback | oynatma | The menu and page "Oynatma"; "Oynatma ayarları...". During playback is "oynatırken", not "oynatma sırasında". |
| close playback | oynatmayı kapat | Stops and unloads the file. Exit (RVP itself) is "Çıkış"; "Kapat" alone is Close. |
| previous / next | önceki / sonraki | "Önceki bölüm", "Sonraki öğeyi oynat". "Geri" and "ileri" are seek directions, not previous and next. |
| seek | sar | "İleri sar", "geri sar"; the noun "sarma" ("Fareyle sararken duraklat"). PotPlayer's Time Jump (the arrow keys' steps) is seeking too: the page "Sarma". Never "ara", which is search. |
| jump | git | With its target in the dative: "Belirtilen zamana/kareye git...", the button and submenu "Git", "Sonraki slayta git". Windows' jump list is its own "atlama listesi". Never "atla", which is skip. |
| frame step | kare kare | The commands are "Sonraki kare" / "Önceki kare". |
| speed | hız | "Oynatma hızı", "Hızı artır". Not "oran" and not "tempo". |
| repeat | tekrarla | The noun "tekrar" ("Tekrar sayısı (0: sonsuz)"). Redo is "Yinele", never repeat. |
| A-B repeat / section repeat | A-B tekrarı / aralık tekrarı | The marks are "A noktası" and "B noktası", the marked part "aralık" ("İşaretli aralığı tekrarla"). Never "bölüm", which is chapter. |
| shuffle | karışık oynat | The noun "karışık oynatma"; "Randomly" is "Rastgele", random order "rastgele sıra". |
| queue | kuyruk | "Oynatma kuyruğu", "Oynatma kuyruğuna ekle". Not "sıra", which is order. |
| resume / remember position | sürdür / konumu hatırla | The page's heading "Sürdürme"; "Kaldığı yerden sürdür"; "Oynatma konumlarını hatırla". Not "devam et" in some strings and "sürdür" in others. |
| position | konum | The time in the file ("oynatma konumu") and a place on the screen ("Logo konumu") alike; "where it was last" is "son konumunda". |
| duration | süre | "Dosya sürelerini algıla". An intro's or ending's length is "uzunluk". |
| elapsed / remaining time | geçen süre / kalan süre | Short choices "Geçen", "Kalan", "Geçen ve kalan". |
| chapter | bölüm | Chapters only: a section of the A-B repeat is "aralık", a part of the window "alan". |
| bookmark | yer imi | Plural "yer imleri"; "Yer imi ekle", "Yer imlerini düzenle...". |
| skip (intro, ending) | atla (açılış, kapanış) | "Açılışı atla", "Kapanışı atla", "Açılış uzunluğu", "Bölümleri atla"; the page "Atlama". Not "giriş" (input) for the intro, not "son" (last, recent) for the ending. |
| seek bar | ilerleme çubuğu | Also PotPlayer's "nav bar" and "navigation bar". Never "arama çubuğu" (search bar) or "kaydırma çubuğu" (scroll bar). |
| thumbnail | küçük resim | Plural "küçük resimler", accusative "küçük resmi"; "Küçük resim oluştur...". |
| preview | önizleme | "Küçük resim önizlemelerini göster". |
| scene browser | sahne tarayıcısı | A browser is "tarayıcı" in both RVP's browsers; to browse is "göz at", the button "Gözat…". |
| file | dosya | Singular after a number: "%1 dosya". |
| folder | klasör | As on Windows and GNOME; not "dizin". |
| URL / address | URL / adres | A bare object before the verb: "URL aç...", "URL ekle..."; with an ending "URL'yi", "URL'ler". |
| stream | akış | Network media and a stream inside a file ("Ses akışı seç", "Akışı kaydet..."). A track is "parça". |
| device / capture device | aygıt / yakalama aygıtı | As Windows and GNOME write it; never "cihaz" beside it. A webcam is "web kamerası". |
| screen capture | ekran yakalama | "Ekran yakalama aç". |
| disc | disk | An optical disc; its drive is "sürücü" ("DVD sürücüsü"). |
| title | başlık | A DVD title ("Başlık menüsü"), and also a file's, a bookmark's or a window's title. |
| angle | açı | "Kamera açıları", "Açı menüsü". |
| disc menu | disk menüsü | |
| drag and drop | sürükle-bırak | The verb "sürükleyip bırak"; dropped files are "bırakılan dosyalar". |
| recent files | son dosyalar | Recent items "son öğeler", recent URLs "son URL'ler". "Son" alone is last; a file's ending is "kapanış". |
| favorites | favoriler | "Favorilere ekle", "Favorileri düzenle...", "Albüm/favoriler". |
| file association / file type | dosya ilişkilendirmesi / dosya türü | The page "İlişkilendirme". RVP registers itself for a type: "kendini kaydeder", never makes itself the default: "varsayılan yapmaz". |
| playlist | oynatma listesi | Long but established; plain "liste" only where the source itself says "list". |
| album | albüm | One of the playlist's tabs; album art is "albüm kapağı". |
| entry | öğe | Also the source's "item"; "Sonraki öğeyi oynat". |
| sort | sırala | "Sort by" is "Sıralama ölçütü"; "Artan sıra (A'dan Z'ye)", "Azalan sıra (Z'den A'ya)". |
| add / remove / delete | ekle / kaldır / sil | "Kaldır" takes entries off a list and leaves the file; "Sil" deletes the file itself, or something RVP keeps (a bookmark, an album, a preset). |
| recycle bin / trash | Geri Dönüşüm Kutusu / çöp kutusu | Windows' name keeps its capitals, its ending after an apostrophe: "Geri Dönüşüm Kutusu'na gönder". Linux's is a common noun: "çöp kutusuna". |
| import / export | içe aktar / dışa aktar | Notices "İçe aktarıldı.", "Dışa aktarıldı.". |
| window / main window | pencere / ana pencere | |
| fullscreen / windowed | tam ekran / pencereli | In fullscreen "tam ekranda", while in fullscreen "tam ekrandayken"; "Tam ekran (başka ekranda)". |
| maximize / minimize | ekranı kapla / simge durumuna küçült | Windows' words; the states "ekranı kaplamış" and "simge durumunda" ("Simge durumundayken duraklat"). "Büyüt" and "küçült" alone change a size. |
| always on top / stay on top | her zaman üstte / üstte kal | "Üstte kal" is the toggle, "Her zaman üstte" the choice. |
| opacity | opaklık | Transparency is "saydamlık" ("Menülerin saydamlığı"). |
| skin | kaplama | "PotPlayer kaplaması (.dsf)", "Kaplama modları", "Kaplama klasörünü aç". Not "tema" (a color theme) or "görünüm" (a view). |
| color theme | renk teması | "Rastgele renk teması", "Özel renk teması". |
| logo | logo | Ending "logosu". |
| title bar | başlık çubuğu | |
| control panel | kontrol paneli | RVP's F7 window. Windows' Control Panel ("Denetim Masası") is never meant. |
| OSD / on-screen display | OSD / ekran üstü gösterim | "OSD mesajları", "OSD yazı tipi"; OSD's ending by its letters: "OSD'nin". |
| on-screen controller | ekran üstü denetim | mpv's OSC and scripts such as uosc; the setting "Controller" is "Denetim". Not "kumanda", which is a remote or a gamepad. |
| tooltip | araç ipucu | Plural "araç ipuçları". |
| context menu | bağlam menüsü | Also the file manager's ("dosya bağlam menüsü"). |
| tray icon / notification area | tepsi simgesi / bildirim alanı | "Tray" alone is "tepsi" ("Görev çubuğu ve tepsi"). |
| taskbar | görev çubuğu | Linux's dock is "rıhtım" where the source names it. |
| notification | bildirim | |
| dialog | iletişim kutusu | Plural "iletişim kutuları"; a "modeless" dialog is "kalıcı olmayan". |
| settings / preferences | ayarlar / tercihler | "Preferences..." is "Tercihler..."; the settings window and every "... Settings..." item "ayarlar". |
| page | sayfa | "Günlük sayfası" (the Log page). |
| preset / configuration | ön ayar / yapılandırma | "Ön ayar %1", "Ekolayzer ön ayarları"; the page that manages presets is "Yapılandırma". |
| reset / default | sıfırla / varsayılan | "Sıfırla", "Tümünü sıfırla", "Sayfayı sıfırla"; the button "Varsayılan"; "RVP varsayılanları". |
| apply | uygula | Notice "Uygulandı."; a filter "uygulanır" too. |
| key / hotkey / shortcut | tuş / kısayol tuşu / kısayol | A case ending goes on "tuşu" after a key's name: "Enter tuşuna basın". |
| mouse / wheel / button | fare / tekerlek / düğme | "Fare tekerleği", "fare düğmesi"; X1 and X2 keep their names. |
| gesture | hareket | Always "fare hareketi" or "dokunma hareketi", so that it never reads as the motion of motion smoothing. |
| touch | dokunma | "Dokunma ayarları"; a touchscreen is "dokunmatik ekran". |
| controller / gamepad | oyun kumandası / oyun kolu | "Xbox oyun kumandası"; mouse emulation is "fare öykünmesi". mpv's on-screen controller is "ekran üstü denetim". |
| video / picture / image | video / görüntü / resim | The video's picture is "görüntü"; a still image or a picture file "resim" ("Resimler (PNG, JPEG, BMP, GIF)"). |
| frame | kare | One picture of the video. A window's or picture's frame is "çerçeve". |
| frame rate / fps | kare hızı / fps | Written out "saniyede kare" ("saniyede 60 kare"). |
| aspect ratio | en boy oranı | Also PotPlayer's "AR" in the menus ("En boy oranını pencereye uydur"). |
| crop | kırp | The noun "kırpma"; Extend/Crop "Genişlet/Kırp". |
| zoom / pan | yakınlaştır / kaydır | Zoom In / Out "Yakınlaştır" / "Uzaklaştır", the noun "yakınlaştırma"; pan "Görüntüyü kaydır". |
| rotate / flip / mirror | döndür / çevir / aynala | Flip Horizontal / Vertical are "Yatay çevir" / "Dikey çevir"; upside down alone is "ters çevir". |
| deinterlace | geçmeli taramayı gider | Interlaced is "geçmeli"; the noun "geçmeli tarama giderme" ("Geçmeli tarama giderme ayarları..."). The same verb as denoise. |
| brightness / contrast / saturation / hue / gamma | parlaklık / kontrast / doygunluk / renk tonu / gama | "Ton" alone only where "renk tonu" does not fit. Luma and chroma are "luma" and "kroma". |
| sharpen / denoise | keskinleştir / gürültü gider | The nouns "keskinleştirme", "gürültü giderme". The sound's noise reduction is "gürültü azaltma". |
| scaler / upscaling / resize | ölçekleyici / büyütme / boyutlandır | Upscaler / Downscaler / Chroma scaler: "Büyütme ölçekleyicisi" / "Küçültme ölçekleyicisi" / "Renk ölçekleyicisi"; resize "boyutlandır", the noun "boyutlandırma" ("Boyutlandırma öncesi gölgelendiriciler"). |
| pixel shader | piksel gölgelendirici | Plural "piksel gölgelendiriciler"; where the source says only "shader", "gölgelendirici" ("Gölgelendirici klasörleri"). mpv's .glsl and .hook files too. |
| renderer | işleyici | "Video işleyici"; PotPlayer's audio renderer is the sound's output, "Ses işleyici". The video output is "video çıkışı". |
| hardware decoding / decoder | donanımsal kod çözme / kod çözücü | Software decoding "yazılımsal kod çözme"; "AV1 kod çözücü". |
| HDR | HDR | Endings by its letters: "HDR'nin", "HDR'ye". |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360° video / anaglif / yan yana / üst-alt | "3D" as written, not "3B": the menus' "%1D" strings keep the D. |
| bars / letterbox / margin | bant / letterbox / kenar boşluğu | Black bars "siyah bantlar"; the skin's control bars are "çubuklar". A margin keeps both words: "Alt kenar boşluğu". |
| frame capture / screenshot | kare yakalama / ekran görüntüsü | "Geçerli ekran karesini kaydet". |
| motion smoothing | hareket yumuşatma | |
| frame interpolation | ara kare üretimi | To interpolate "ara kare üret"; frame blending "kare harmanlama". |
| engine | motor | Smoothing's and whisper's engines, and TensorRT's built engine. |
| calibration | kalibrasyon | The verb "kalibre et": "Kalibre et…", "Yeniden kalibre et…". |
| quality preset (light, medium, high) | kalite ön ayarı (hafif, orta, yüksek) | The calibration's presets Fast / Medium / Quality are "Hızlı" / "Orta" / "Kaliteli". |
| GPU / graphics card | GPU / ekran kartı | "GPU'da", "GPU'nun"; "ekran kartı", the usual Turkish word, not "grafik kartı". |
| processor / CPU | işlemci / CPU | "CPU'da", "işlemcide". |
| power budget | güç bütçesi | |
| dropped frames | düşen kare | The count "düşen kare sayısı", the event "kare düşmesi". Never "atlanan", which is skip. |
| refresh rate | yenileme hızı | The display's rate is "ekranın yenileme hızı". |
| audio / sound | ses / ses | The menu "Ses"; an audio track "ses parçası". |
| volume / mute | ses düzeyi / sessiz | The mute toggles "Sessiz", "Sessiz (ana)". |
| master volume | ana ses düzeyi | PotPlayer's Wave volume is "Wave ses düzeyi" (Wave is the name of Windows' mixer channel). |
| equalizer | ekolayzer | Endings "ekolayzeri", "ekolayzerde". |
| normalizer | normalleştirici | The TomSteady normalizer; "Normalleştirici/Freeverb". |
| reverb | yankılanma | Crystality's echo is "yankı". |
| noise reduction | gürültü azaltma | The video's denoise is "gürültü giderme". |
| channel / speaker | kanal / hoparlör | "Sol kanal", "Çıkış hoparlörleri". |
| downmix | kanal indirgeme | In a sentence "daha az kanala indirir" ("5.1 ve 7.1'i stereoya indirir"). |
| surround / stereo | çevresel / stereo | "Çevresel ses", "Sanal çevresel ses (HRTF)"; Dolby Surround and Pro Logic are names and stay. |
| output device | çıkış aygıtı | |
| sample rate / bitrate | örnekleme hızı / bit hızı | |
| audio delay / audio sync | ses gecikmesi / ses senkronu | The noun "senkron" ("Altyazı senkronu"), the verb "eşle" ("Buraya eşle"); not "eşitle", which is file syncing. |
| pitch / time stretching | perde / zaman esnetme | Pitch shifting "ses perdesi kaydırma". |
| audio recording | ses kaydı | "Kaydet" is also Save: "Ses kaydını başlat/durdur", "Ses kaydet...". |
| second audio device | ikinci ses aygıtı | "2. aygıt" in tight labels. |
| pass-through | doğrudan aktarım | "AC3 ile yeniden kodlayarak doğrudan aktarım". Not "geçiş" (the slideshow's transition). |
| subtitles / subtitle track | altyazı / altyazı parçası | One word, as media players write it (not TDK's "alt yazı"). |
| external / embedded subtitles | harici altyazı / gömülü altyazı | External audio files are "harici ses dosyaları". |
| font / outline / shadow | yazı tipi / dış hat / gölge | A border is "kenarlık"; "Dış hat kalınlığı". |
| subtitle position / subtitle delay | altyazı konumu / altyazı gecikmesi | |
| encoding | kodlama | A text's "metin kodlaması"; the sound's "yeniden kodlama" (AC3) is the other sense. |
| subtitle browser / subtitle editor | altyazı tarayıcısı / altyazı düzenleyicisi | |
| subtitles from audio / speech recognition | sesten altyazı / konuşma tanıma | "Sesten altyazı oluştur...". |
| model | model | whisper's model names (tiny, base...) stay. |
| language | dil | "Arayüz dili", "ses dili", "konuşma dili". |
| remote control | uzaktan kumanda | The remote programs on a phone are "kumanda uygulamaları". |
| token | belirteç | |
| port / server | port / sunucu | "Port" as network users say it; Windows' "bağlantı noktası" is too long for the label. A common noun: "portu", no apostrophe. |
| log | günlük | The page "Günlük", "günlük düzeyi"; the other meaning, daily, is never meant. |
| crash | çökme | "Çökme günlüğü". |
| download / install | indir / kur | Installed "kurulu", installation "kurulum"; "yükle" is load. |
| installer | kurulum programı | Not "yükleyici": "yükle" is load. |
| accessibility | erişilebilirlik | |
| text to speech (TTS) / screen reader | metin okuma (TTS) / ekran okuyucu | |
| screen saver | ekran koruyucu | |
| shut down / sleep / hibernate | kapat / uyku / hazırda beklet | Windows' power words; "Bilgisayarı kapat" where "Kapat" alone would read as Close. |
| timer | zamanlayıcı | The Shutdown Timer is "Kapatma zamanlayıcısı". |
| visualization / spectrum | görselleştirme / spektrum | "Spektrum çubukları", "Dalga biçimi" (waveform). |
| status bar / info | durum çubuğu / bilgi | "Dosya bilgisi", "Kodek bilgisi". |
| codec / format | kodek / biçim | Codec names stay; "Resimleri PNG biçiminde kaydet". |
| bit depth / 10-bit | bit derinliği / 10 bit | "%1 bit", "10 bit çıkış", "32 bit (kayan nokta)". |
| channel layout (5.1, 7.1) | kanal düzeni (5.1, 7.1) | |
| Auto select (recommended) | Otomatik seç (önerilen) | "(Recommended)" is always "(önerilen)". |
