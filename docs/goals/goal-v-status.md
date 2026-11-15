# Goal V: interface languages (M19, D20)

Goal V adds twelve interface languages to English and Russian on the
owner's Ubuntu 26.10 boot (`docs/PLAN.md`, M19 and goal V); goal V-W
checks on the Windows VM what reaches Windows ("For goal V-W" below).
The owner's rule (2026-11-15): no language but English, the source text
and the fallback, is written into the code.

| Step | What | State |
|------|------|-------|
| 1 | No language in the code | done |
| 2 | Rules and glossaries (`docs/i18n`) | done |
| 3 | Translation, one agent per language | in progress |
| 4 | Review by back-translation and scripts | in progress |
| 5 | Fit checks and screenshots per language (Xvfb) | in progress |
| 6 | CJK fonts, README, coverage, AGENTS.md | |

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
| `pl` | 59; 1 (a text too long), fixed | PASS | |
| `uk` | 47; 0 | | |
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
| `it` | 45; 1, fixed, then 1 more in the shortened texts, fixed | PASS | |

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
