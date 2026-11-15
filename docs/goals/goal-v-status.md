# Goal V: interface languages (M19, D20)

Goal V adds twelve interface languages to English and Russian on the
owner's Ubuntu 26.10 boot (`docs/PLAN.md`, M19 and goal V); goal V-W
checks on the Windows VM what reaches Windows ("For goal V-W" below).
The owner's rule (2026-11-15): no language but English, the source text
and the fallback, is written into the code.

| Step | What | State |
|------|------|-------|
| 1 | No language in the code | done |
| 2 | Rules and glossaries (`docs/i18n`) | |
| 3 | Translation, one agent per language | |
| 4 | Review by back-translation and scripts | |
| 5 | Fit checks and screenshots per language (Xvfb) | |
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
