# Goal V: interface languages (M19, D20)

Goal V adds twelve interface languages to English and Russian on the
owner's Ubuntu 26.10 boot (`docs/PLAN.md`, M19 and goal V); goal V-W
checks on the Windows VM what reaches Windows ("For goal V-W" below).
The owner's rule (2026-11-15): no language but English, the source text
and the fallback, is written into the code.

State (2026-11-16): done on this boot. RVP has fourteen interface
languages: English and Russian, and the twelve of goal V (`pt_BR`, `de`,
`es`, `fr`, `it`, `pl`, `uk`, `tr`, `zh_CN`, `zh_TW`, `ja`, `ko`), each
translated, reviewed through a blind back-translation, fixed and looked
at again, fitting every settings text under Xvfb, and each language's
screenshot tour looked through. RVP starts in the system's language.
Goal V-W checks on the Windows VM what the last section lists.

| Step | What | State |
|------|------|-------|
| 1 | No language in the code | done |
| 2 | Rules and glossaries (`docs/i18n`) | done |
| 3 | Translation, one agent per language | done |
| 4 | Review by back-translation and scripts | done |
| 5 | Fit checks and screenshots per language (Xvfb) | done |
| 6 | CJK fonts, README, coverage, AGENTS.md | done |

## Step 1: no language in the code

Russian was written in nine places; each now takes the translation
files found:

- **Build** (`shell/build.rs`): every `i18n/<part>_<code>.ts` is grouped
  by its code and compiled with lrelease into `qml_<code>.qm`; parts are
  `qml` (lupdate's), `menu` (the "Menu" context) and the new `packaging`
  (the desktop entry's GenericName and Comment, the installer's
  component).
- **The window** (`shell/cpp/shell.cpp`): `Tools.interface_languages()`
  lists the `.qm` files built in, each named by its own translation of
  "English" (context "Language", marked in `Settings.qml` with
  `QT_TRANSLATE_NOOP`); `Tools.system_language()` follows QTranslator's
  rule over the system's UI languages (each, then with its last part
  dropped), with English counted as present, then a translation of the
  same language for another region (same script first). `Main.qml`'s
  `applyLanguage` uses both; a code RVP lacks shows English.
- **The settings and the menu**: the General page's list is "The
  system's", English and the files' languages by their own names.
  `general::LANGUAGES` is gone: `general.language` is `auto`, `en` or any
  code of a language's shape (`general::is_language_code`), and the
  window gives the app its languages at start (`rvp_interface_languages`,
  `App::set_interface_languages`) for Misc > Interface Language (Auto
  select, English, then the translations). The `language-russian` action
  is gone; `language-auto` and `language-english` stay.
- **Checks**: `tests/translations.rs` reads every `.ts` file in
  `shell/i18n` and fails a string some language lacks (the union of all
  languages' messages), a lost placeholder, a language without its own
  name, a menu label without a translation and a desktop entry that
  differs from the files. M14's language steps take `--language=CODE`
  (default the first translation built in) and run alone with
  `--selftest=language`; they now also check that the system's language
  took at the start (`m14_system_language`, `--expect-system-language`).
- **Packages**: `packaging/linux/rvp.desktop` carries `GenericName[code]`
  and `Comment[code]` written by `tools/i18n/desktop.py` from the
  packaging files (the template of the packages' entry and of the
  user's, so both have them; the AUR recipe copies it). The installer's
  languages (`languages.nsh`, written by `package.py`'s `nsis_languages`)
  are each found among NSIS's own language files by the Windows locale
  of their LANGID (`locale.windows_locale`): no table. Checked against
  NSIS 3.10's 134 files: all twelve planned languages and Russian find
  theirs (PortugueseBR, SimpChinese and TradChinese by region).
- **`tools/i18n/tsfile.py`** reads and writes `.ts` files as lupdate lays
  them out (control characters as `<byte value="xNN" />`).

Checks (2026-11-15, this boot, Xvfb): `cargo test -p rvp-app --test
translations` (6 tests) and `general::tests` (the codes' shape; the
language menu and its command with a made-up translation `xx`);
`--selftest=language --language=ru` PASS (`m14_language_loads` "ru
Русский", `m14_language_fits` 0 overflows); the system's language under
five locales: `LANGUAGE=ru` ru, `en_US:ru` en (English first wins),
`de` en and `pt_PT` en (no such translation yet), `uk:ru` ru;
`--selftest=m14` with both files: 0 failures; the coverage reports
rewritten (`RVP_BLESS=1`).

CI at d785012 (step 1): every job green; `package-windows` built the
installer with `languages.nsh` from the LANGID match, and
`package-linux`'s Ubuntu 26.10 container check passed (the .deb
installed, rvp and its 177 libraries resolve, whisper.cpp and the
VapourSynth runtime present, the self-tests of the .deb, with the gtk3
theme, of the AppImage, M11 and M16 all PASS).

## Step 2: rules and glossaries

`docs/i18n/README.md` holds the process, the rules every language
follows (meaning for this program, no other program's strings, names
and key names kept, placeholders, "...", "&" literal, units, length,
consistency) and the glossary's 144 English terms with their meaning in
RVP. Each translator agent wrote its language's file first; each
glossary has all 144 rows in order (checked by script, 217 term pairs by
`work.py`'s parser). The main choices:

| Language | Address and labels | Notes |
|----------|--------------------|-------|
| `es` | usted; infinitive commands; sentence case | one neutral Spanish (equipo, presionar, video), ¿ ¡, “ ” |
| `pt_BR` | implied você; infinitive commands | 1990 spelling, “ ”, decimal comma in prose |
| `fr` | vous; infinitive commands | U+00A0 before : ; ? ! and inside « », habillage for skin |
| `de` | Sie; object first, infinitive last | „…“, Ctrl not Strg (RVP's key names), hyphen at long compounds' joints |
| `it` | tu; imperative commands | « », loanwords file/playlist/skin with their gender |
| `pl` | second person singular imperative; perfective commands, imperfective checkboxes | no noun after a number (three plural forms), „…” |
| `uk` | ви; perfective infinitives | тека, типовий, ’ (U+2019), Cyrillic units |
| `tr` | siz only where the source speaks to the user; verb last | "%1 %" becomes "%%1"; nothing attached to %N |
| `zh_CN` | 您 only when needed; verb + object | full-width punctuation, one space between Chinese and Latin |
| `zh_TW` | Taiwan's own terms, not a conversion | 「」, 設定檔 against 預設集 for presets |
| `ja` | です/ます in sentences, object + を + verb in labels | フォルダー (ー after -er/-or), half-width space before Latin |
| `ko` | 합니다체, noun-phrase labels | particles never attached to %N where avoidable, 이(가) last |

Every file keeps file-dialog filters' parentheses ASCII (Qt reads the
patterns from them) and RVP's English key names inside combinations.

## Steps 3 to 5: the languages

Each language: its translator agent's chunks (`out-NN.json`), a blind
back-translation and review by another agent (`issues.json`), the
translator's fixes, the reviewer's second look, then
`--selftest=language` under Xvfb and the screenshot tour looked
through. Fifteen submenu titles that no check had seen (below) were
translated by every language afterwards and reviewed with the fixes.

| Language | Review: issues, then left after the second look | Fit | Committed |
|----------|------------------------------------------------|-----|-----------|
| `pt_BR` | 137; 0, then 4 in the titles and shortened texts, fixed | PASS | yes |
| `de` | 51; 0 | PASS | yes |
| `es` | 37; 1, fixed | PASS | yes |
| `fr` | 64; 4, fixed | PASS | yes |
| `it` | 45; 1, fixed, then 1 more in the shortened texts, fixed | PASS | yes |
| `pl` | 59; 1 (a text too long), fixed | PASS | yes |
| `uk` | 47; 0, then 1 choice shortened for its box | PASS | yes |
| `zh_CN` | 36; 0 | PASS | yes |
| `tr` | 88; 0, then 1 choice shortened for its box | PASS | yes |
| `zh_TW` | 47; 1, fixed | PASS | yes |
| `ja` | 15; 0 | PASS, after the Keys page's scope column (below) | yes |
| `ko` | 50; 0 | PASS | yes |

The last round (2026-11-16, all commits in, Xvfb): `tools/i18n/check.sh`
with both tour videos for every language, 13 of 13 PASS (0 texts that
do not fit) and six shots each, looked through: the settings pages, the
Keys and Mouse page, the menu, the control panel and the player.
Chinese, Japanese and Korean show in Noto Sans CJK with no missing
glyphs (no boxes, no fallback gaps), but Japanese and Simplified
Chinese had Traditional Chinese's shapes (below). The pictures also
showed a cut the check had missed, the Keys and Mouse page's key names
under the row's buttons in every language (below); after both fixes the
check runs 13 of 13 PASS again, Japanese's and Simplified Chinese's
tours show their own shapes, and the README's two settings pictures
were taken again in English (`tour.sh` at 1920x1080).

## Step 6: fonts, README, coverage, AGENTS.md

- **CJK fonts**: the .deb recommends `fonts-noto-cjk` (`package.py`'s
  Recommends, beside GTK's) and the AUR recipe lists `noto-fonts-cjk`
  among its optional dependencies; the AppImage uses the system's fonts
  as before. Windows has its own (Microsoft YaHei, JhengHei, Yu Gothic,
  Malgun Gothic), which Qt finds by its fallbacks: goal V-W looks at
  them.
- **README**: the interface in fourteen languages, named, RVP starting
  in the system's language, and a note that AI agents wrote and
  cross-checked the translations, with corrections from native speakers
  welcome.
- **Coverage** (`RVP_BLESS=1`): the Interface Language reasons say that
  RVP lists English and its own translations by their own names and
  shows English for a language it lacks, and that RVP's built-in
  language is English where PotPlayer's is Korean
  (`docs/coverage/menus.md`).
- **AGENTS.md**'s M14 section: the translation process (`work.py`,
  `check.sh`), the rule that a settings row must shrink to its page, the
  CJK fonts, and the filled menu labels' one form per kind of value.

## Found on the way

- **Test launches reached the owner's RVP** (2026-11-15, about 19:00):
  five launches of the language check had their arguments after `--`, so
  they were no self-tests and handed 25 bogus "files" (`/tmp/LANG=...`,
  `/tmp/--selftest=language`...) to the owner's running RVP, which added
  them to its Default album. Nothing else changed; the owner removes them
  (or, with RVP closed and the owner's word, they are taken out of
  `playlists.json`). Test launches now get their own `XDG_RUNTIME_DIR`
  and session bus (`dbus-run-session`; `tools/screenshots/tour.sh` too),
  the session's helper refuses anything but a self-test, and AGENTS.md
  says so. Checked: an isolated run added nothing.

- **Strings no language could translate.** Before step 3 every visible
  English text of the QML files goes through `qsTr`: 66 strings were
  plain literals or English tails glued to a translated start (file and
  folder dialogs' titles and filters, the playlist's dialogs, the audio
  device page's labels, the power budget's and the time units' formats,
  "found"/"missing" on the smoothing page, the SVP and VapourSynth help
  texts, the stereo downmix note; the French translator found the last).
  File patterns stand aside as `%1`. lupdate had also read the ASS fade
  tag `\fad` in a help text as a form feed, so that text never matched
  its translation (the Russian one was cut at "("): the tag is `%1`
  now. Russian has all 66 (1331 QML strings), and still fits.

- **An exit crash of `tests/m12.rs`** (2026-11-15, once): all 14 tests
  passed, then the test process ended with SIGSEGV; four reruns were
  clean. The changes of the moment were QML and translation files only,
  which the test does not load. Left open: libmpv's teardown at the
  process's exit is the likely place.

- **The owner's crash log of 07:23 is explained** (goal U's open item):
  apport's report (`/var/crash/_opt_rvp_bin_rvp.1000.crash`) names the
  command, `/usr/bin/rvp --crash-test=native`: the kit's m8 step crashed
  the installed RVP on purpose to check its crash log, and with
  `RVP_STATE_DIR` unset there the log went to the owner's state folder.
  `m8_crash` now gives each crash a state folder of its own (and no
  longer removes whatever folder the variable names).

- **M14's fit check measured too early.** It walked each settings page
  right after `showPage`, before the page's layout ran: fields still had
  their implicit widths, so hints in empty fields counted as cut (the
  first Spanish, French and Portuguese runs listed 8 to 14 such), while a
  real cut went unseen (Russian's "Открыть текущий/последний элемент
  заново" on the Keys and Mouse page, elided). The check now shows one
  page a tick and measures it 150 ms later. With it, the empty fields'
  hints fit, and the Keys and Mouse page's action names take a second
  line instead of being elided; `Field` also spans the page's width.

- **The settings pages were wider than the window in long languages.**
  The page stack took the widest page's width, and a row that cannot
  shrink (a checkbox's or heading's unwrapped text, a slider row with a
  long value) pushed every row of its page past the window's right edge,
  cutting texts and buttons; Russian's subtitle sync and presets pages
  already did. The screenshots showed it, the fit check did not. Now
  checkboxes and headings wrap, a slider gives its room to a long value
  (120 to 280 px), choices are 360 px, and `Field` spans the page; the
  fit check also fails a row wider than its page and a choice whose text
  its box clips. Russian's two time stretching choices became shorter.
- **Fifteen submenu titles untranslated in every language** ("Filters",
  "Video Renderer", "Audio Output" and others under DVD Control, TV,
  Subtitles): they show only while a file plays, so the menu test never
  saw them. Russian has them (appended to `menu_ru.ts`), and
  `tests/translations.rs` now also fails a menu message one language has
  and another lacks. The Filters submenu's lines ("Source: ...", "Video
  decoder: ...") are formatted in Rust with the codecs' names and stay
  English, as RVP's other Rust-formatted texts.
- **Menu labels filled with a value fell back to English** in some of
  their forms: the audio sync step of a second or more ("1.5 Sec", where
  PotPlayer's items name it in ms), a skip of a minute or more ("Skip
  Intro 1 Min 30 Sec"), a custom or Extend/Crop ratio written as a number
  or with "/" ("Custom (2.35)"), and the shutdown timer once set
  ("Shutdown at 23:30"; "In" named the time as typed, "90" too). The
  audio step is always in ms now, ratios show as `W:H` or the number, the
  timer's "In" label is `h:mm` (`h:mm:ss`), and every menu file has the
  other forms (ten messages a language), derived from its reviewed
  translation of the same item; the time's word order was written by
  hand for Turkish, Traditional Chinese and Japanese. The menu test now
  walks the menus with such settings and with each timer set.
- **The Keys and Mouse page's scope column** was 60 px, which Japanese's
  "Window" (ウィンドウ) overflows on every row; it takes the longer of
  its two words in the language shown.
- **The Keys and Mouse page's key names lay under the row's buttons**
  in every language, English's README picture too ("MediaPlayPause"
  under Key...): a `RowLayout` shrinks every item without a minimum
  width, and the keys' `Flow` had shrunk to almost nothing (Russian's
  and German's to no width at all). The action's name and the keys now
  give room down to minimums of their own, the buttons keep their
  texts' widths, and a key's button never reaches past its column. M14's
  fit check now also fails a `Flow`'s item past its right edge: run on
  the old page, it listed 15 in Russian and German; on the new one, 0 in
  every language.
- **Japanese and Simplified Chinese in Traditional Chinese's shapes**:
  the ideographic comma (、) stood in the middle of the line, as
  Traditional Chinese writes it, where Japanese and Simplified Chinese
  put it low. Han characters take their shapes from a language, and
  fontconfig orders Noto Sans CJK's variants by `FC_LANG` or the locale,
  here the owner's `ru_RU`. The tour under `FC_LANG=ja` showed
  Japanese's comma. RVP now sets `FC_LANG` from a language chosen in its
  settings before Qt starts (`player::font_language`, Linux; the user's
  own `FC_LANG` wins, "auto" is the locale's language already); the
  Japanese and Simplified Chinese tours without it in the environment
  show their own shapes. Windows picks its CJK fonts by the system's
  language: goal V-W looks at a language other than the system's.
- **Traditional Chinese's "Filters"**: the translator's fix made the
  submenu 篩選器 (a filter of a list); the reviewer's second look kept
  濾鏡, since RVP's Filters submenu lists mpv's filter chain, and the
  glossary says so.

## For goal V-W

The Windows VM is not reachable from this boot. These changes reach
Windows:

| Change | Step | What it changes on Windows | VM check |
|--------|------|----------------------------|----------|
| `build.rs` compiles every language's files | 1 | the bundle's `.qm` resources | the bundle builds; `--selftest=language` for each language |
| `Tools.interface_languages`, `Tools.system_language`, `applyLanguage` | 1 | the language at the first start (Windows' UI languages) | `--selftest=language --expect-system-language=en` on the VM (English Windows) |
| The General page's language list; Misc > Interface Language from the files; `language-russian` gone | 1 | the settings window, the menu | `m14-all.ps1` |
| M14's language steps (`--language`, `--selftest=language`, `m14_system_language`) | 1 | `SelfTestM14.qml` | `m14-all.ps1` |
| The installer's languages from the files by LANGID | 1 | `languages.nsh`; Russian's component text as before | `m8-all.ps1` (install and uninstall) |
| `tests/translations.rs` reads `shell/i18n` | 1 | none (headless) | CI's Windows checks |
| Every visible QML text through `qsTr` | after 1 | dialogs' titles and filters, the audio device page, formats | `m14-all.ps1` (the texts fit), the file dialogs by hand |
| M14's fit check measures each page after its layout and fails rows wider than the page and clipped choices; checkboxes, headings and action names wrap, sliders give room to long values, choices are 360 px, `Field` spans the page | 5 | `SelfTestM14.qml`, `Settings.qml`, `KeysAndMouse.qml` | `m14-all.ps1`, and the settings window looked at in a long language |
| Twelve new languages (`shell/i18n/*_<code>.ts`) | 3 | the bundle's `.qm` files, the settings window and menus in each | `--selftest=m14 --language=CODE` for every language; the settings window and menus looked at, Chinese, Japanese and Korean in Windows' own fonts |
| The installer in each language (`languages.nsh` from the packaging files) | 3 | the installer's texts and RVP's components | `m8-all.ps1` (install and uninstall) with the installer in each language |
| The Keys and Mouse page's scope column takes its words' width; the key names keep their column (minimum widths); M14's fit check fails a `Flow`'s item past its edge | 5 | `KeysAndMouse.qml`, `SelfTestM14.qml` | `m14-all.ps1` in Japanese and Russian, and the Keys and Mouse page looked at |
| Han characters' shapes follow the system's language on Windows (Qt's fallback fonts); `FC_LANG` is Linux's | 5 | none: `player::font_language` is Linux's | Japanese and Simplified Chinese on the English VM: whether 、 and 。 sit low (Yu Gothic, Microsoft YaHei) or take another language's shapes; record which |
| Filled menu labels: the audio sync step in ms, ratios as `W:H` or the number, the timer's "In" label as `h:mm`, their forms in every menu file | 5 | `app.rs`, `picture.rs`, `PlaybackDialogs.qml`, the menu files | `tests/translations.rs` built with `cargo xwin test --no-run`; the timer dialog's "In" with "90" shows "1:30" |
