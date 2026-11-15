# Japanese (ja)

Standard modern Japanese as Japanese desktop software writes it, in the
plain vocabulary of a Japanese media player: the kanji terms that players
and the desktops use (再生, 一時停止, 字幕, 音量, 全画面, 設定, 既定,
ごみ箱) and the established katakana loanwords where Japanese users say
them (スキン, プレイリスト, シークバー, チャプター, コーデック, フィルター,
レンダラー). Whole sentences (help texts, messages, questions) are polite
です/ます sentences; labels (menus, buttons, checkboxes, choices, page
titles) are plain noun phrases or a verb's dictionary form, never です/ます.
The user is never addressed by a pronoun. Katakana words ending in -er,
-or and -ar take the long vowel mark (フォルダー, プレーヤー), as the
Cabinet's loanword notation and Japanese Windows write them. Latin letters,
digits and placeholders are half-width and set off from Japanese text by a
half-width space ("RVP の設定", "%1 秒"). Everything is written for RVP;
nothing is taken from another program's Japanese strings.

## Style

In this file "～" only marks where a pattern attaches to a word ("～して
ください"); it never appears in a translation.

- **Politeness.** Help texts, notices, confirmations and every other
  whole sentence use the plain polite form (です/ます): "RVP は何も
  ダウンロードしません。", "ファイルの再生中に読み取ります。". Never the
  である style, never humble or honorific keigo (いたします, させて
  いただきます, ございます, ご～ください): RVP is the subject of plain
  ます verbs ("RVP が測定します"). Never あなた or another pronoun for the
  user: Japanese leaves the subject out ("models that you download" is
  "ダウンロードしたモデル"); where a subject is needed it is "ユーザー".
  "your own keys" is "自分で割り当てたキー".
- **Instructions and questions.** A request to the user is "～してくださ
  い": "字幕を入力して Enter を押してください。入力を始めた位置から表示
  されます。", "もう一度ダウンロードしてください。". "Please" alone adds
  nothing. Confirmation questions are "～しますか?": "%1 個のファイルを
  ごみ箱に移動しますか?", "「%1」のオプションを RVP の既定に戻しますか?".
- **Notices and progress.** A notice with a final period in the source is
  a polite past sentence with "。": "Applied." "適用しました。",
  "Imported." "インポートしました。", "Cancelled." "キャンセルしました。",
  "Unchanged." "変更はありません。", "Not measured yet." "まだ測定してい
  ません。". A fragment without a period (status words, notes in
  parentheses, list entries) is a noun with no period: "(not downloaded)"
  "(未ダウンロード)", "(not installed)" "(未インストール)", "not set"
  "未設定", "%1 done" "%1 完了", "%1 removed" "%1 を削除しました" (an
  event, so a past verb, still without "。" since the source has none).
  Progress texts are a verbal noun with 中: "Starting…" "開始中…",
  "Measuring %1…" "%1 を測定中…". Words RVP inserts into another string
  must read right there: "checking", "unpacking", "downloading" in "RIFE
  (TensorRT): %1, %2 of %3 GB" are "確認中", "展開中", "ダウンロード中".
- **Menu items and buttons** are commands: the object, を, then the verb.
  A Sino-Japanese verbal noun stands without する (追加, 削除, 保存, 表示,
  選択, 適用, 記憶, 再読み込み); a native verb takes its dictionary form
  (開く, 閉じる, 戻す, 隠す, 送る, 読み込む, 切り替える). Never ～します,
  never ～しよう, never ～する after a verbal noun. "Open File(s)..."
  "ファイルを開く...", "Add Bookmark" "ブックマークを追加", "Save playlist
  as..." "プレイリストに名前を付けて保存...", "Next Chapter" "次の
  チャプター", "Reload Subtitle" "字幕を再読み込み". A tight label may join
  the object and the verbal noun without を ("ブックマーク追加"), but the
  same English label is then written so everywhere. The standard desktop
  words: OK "OK", Cancel "キャンセル", Apply "適用", Close "閉じる", Exit
  "終了", Open "開く", Save "保存", Save As "名前を付けて保存", Browse
  "参照", Choose / Select "選択", Rename "名前の変更", Duplicate "複製",
  Copy "コピー", Paste "貼り付け", Insert "挿入", Find "検索", Edit
  "編集", Undo "元に戻す", Redo "やり直し" (never リピート or 繰り返し,
  which are repeat), Reset "リセット", Yes / No "はい" / "いいえ", About
  "バージョン情報" ("About RVP" "RVP のバージョン情報"), Tools "ツール",
  Misc and Other "その他", General "全般", Advanced "詳細", Customize
  "カスタマイズ", Select All "すべて選択", Deselect All "すべて選択解除",
  Up / Down "上へ" / "下へ", Move Up "上へ移動", Start "開始", Test
  "テスト", Show / Hide "表示" / "非表示" (with an object a phrase may use
  隠す: "マウスカーソルを隠す", "自動的に隠す"). No access keys: never
  "(O)" or "(&O)" after a label.
- **Checkboxes** have the same shape and end the same way, with the bare
  verbal noun or a native verb's dictionary form, never with する or ます:
  "再生位置を記憶", "ツールチップを表示", "再生中はマウスカーソルを隠す",
  "重複したファイルを追加しない". A condition comes first with "～とき",
  "～時", "～中は" or "～場合": "プレイリストの終了時に再生を終了",
  "全画面で再生中". Enable / Disable are "有効にする" / "無効にする"
  (有効 and 無効 are not verbal nouns, so にする stays: "スキップ機能を
  有効にする", "既定のホットキーを無効にする"); Use is "使用" ("イコライザー
  を使用"); Turn on / off are "オンにする" / "オフにする".
- **Choices** in lists are short nouns or plain verb forms: On / Off "オン"
  / "オフ", Enabled / Disabled "有効" / "無効", None "なし", Never "しない",
  Always "常に", Auto "自動", Custom "カスタム", Normal "標準", Low /
  Medium / High "低" / "中" / "高" (with a noun: "低品質", "高品質"),
  "While playing" "再生中", "Do not use" "使用しない", "Auto select
  (Recommended)" "自動選択 (推奨)". The choices of one list take one shape.
- **Page titles, headings, window titles and field labels** are nouns or
  noun phrases: "再生", "字幕", "アスペクト比", "フォントサイズ", "Section
  Repeat" "区間リピート". A window titled by a command becomes a noun
  phrase: "Adjust Audio Sync" "音声同期の調整", "Jump to Specified
  Time/Frame" "時間/フレームを指定してジャンプ" (the menu item that opens
  it keeps its "..."). A label before a field ends with a colon only where
  the source has one: "Album name:" "アルバム名:". Japanese has no
  capitals: the source's Title Case means nothing; names keep their own
  case (mpv, libmpv, yt-dlp, whisper.cpp, dav1d).
- **Kanji, kana and loanwords.** Use the word a Japanese user of Windows,
  GNOME or a Japanese media player knows, whichever its origin. Where a
  short kanji term is usual, it wins over katakana (音量, not ボリューム;
  字幕, not サブタイトル; 全画面, not フルスクリーン; 設定, not セッティング;
  品質, not クオリティ; 遅延, not ディレイ; 優先度, not プライオリティ);
  where Japanese says the loanword, use it (スキン, コーデック, フィルター,
  シェーダー, フレーム, チャプター, ブックマーク, ストリーム, トラック,
  プリセット, エンジン, スクリプト, プラグイン). Never coin a katakana word
  where Japanese has a common one (開く, not オープン). Katakana compounds
  are written closed, without spaces or "・" (シークバー, タイトルバー,
  コンテキストメニュー, スクリーンセーバー). No half-width katakana, no
  furigana, no English in parentheses after a Japanese word except names
  and abbreviations the source has.
- **The long vowel mark.** Loanwords from English words ending in -er,
  -or and -ar end in ー: フォルダー, プレーヤー, ブラウザー, エディター,
  フィルター, シェーダー, レンダラー, デコーダー, エンコーダー, スケーラー,
  イコライザー, ノーマライザー, コントローラー, スピーカー, マーカー,
  ポインター, サーバー, ユーザー, コンピューター, プロセッサー, モニター,
  タイマー, ミキサー, バッファー, パラメーター, インストーラー, スライダー.
  Never the clipped forms (フォルダ, プレーヤ, サーバ). Other endings keep
  the established form, without an added ー: キャプチャ, ジェスチャ,
  ピクチャ, エントリ, ライブラリ, メモリ, ディレクトリ, プロパティ,
  カテゴリ; words whose usual form already has one keep it: バッテリー,
  メニュー, カラー, エラー, センター, スペクトラム, and カーソル keeps its
  usual form without ー. "play" inside プレイリスト keeps the established
  イ; the player itself is プレーヤー ("メディアプレーヤー").
- **Kana and okurigana.** Write in kana the words Japanese software writes
  in kana: すべて (not 全て), できる (not 出来る), ください, とき (for
  "when"; 時 only in compounds such as 終了時, 起動時), など, ほか, こと,
  もの, ため, ごと, まで, すでに, いったん, ごみ箱. Okurigana are full, as
  Japanese Windows writes them: 読み込み, 書き込み, 切り替え, 割り当て,
  貼り付け, 並べ替え, 引き伸ばし, 取り込み, 取り消し (not 読込, 切替,
  割当, 貼付).
- **Character widths and spacing.** Latin letters, digits, placeholders
  and the symbols % / : + - ( ) ? are half-width ASCII; never full-width
  Latin (Ｒ, ３) or a full-width space. A half-width space separates
  Japanese text (kana and kanji) from a Latin word, a number or a
  placeholder on either side: "RVP の設定", "mpv の既定", "%1 秒",
  "%1 個のファイル", "DVD メニュー", "A-B リピート", "Web カメラ",
  "%1D ビデオモード", "Shift+N で有効にする", "%1 を測定中…". No space
  next to Japanese punctuation and brackets (、。「」『』): "「%1」を
  削除", "RVP、mpv、SVP". Between Latin words and half-width punctuation
  the spacing is the source's: "Ctrl+Left / Right", "再生/一時停止",
  "%1 / %2".
- **Punctuation.** Sentences use the full-width "、" and "。"; a help
  text's every sentence ends in "。", and labels end without one, as in
  the source. Questions end in a half-width "?" attached to the text, as
  Japanese Windows dialogs write it ("移動しますか?"); avoid "!". Colons
  are half-width with a space after them ("状態: %1", "ソフト: 半径").
  Japanese has no semicolon: a semicolon in a help text becomes "。" (two
  sentences) or "、"; inside values such as "Opening;Ending;3" it stays.
  A dash that sets off a phrase becomes a colon, "、" or parentheses; the
  source's " - " inside menu labels such as "%1/%2/%3 - %4 Stereo" stays.
  "&" becomes "と" ("Luma & Chroma Offset" "輝度と色差のオフセット") or
  "/" in a tight label; "Add & Edit" "追加/編集". "e.g." is "例:",
  "etc." and "such as" "など" ("uosc や ModernZ などのスクリプト"). Lists
  in sentences take "、"; a parenthesized list of Latin names or values
  keeps the source's commas ("(PNG, JPEG, BMP, GIF)"). Ranges are "から
  … まで" in sentences ("480p から 2160p まで"); the source's "0-255"
  and "16-235" stay. Never the wave dash "～" for ranges: two look-alike
  characters exist for it, and fonts draw them differently.
- **Brackets and quotation marks.** Parentheses are half-width "( )" and
  spaced as in English: a space before "(" and after ")" unless
  punctuation or the end of the text is next to them: "自動選択 (推奨)",
  "有効にする (Shift+N)", "ハードウェアデコード (ビデオ) がオン". Quotation
  marks are 「」 (『』 inside them), for the source's “…” and its straight
  "...": "「%1」に PotPlayer のキーを割り当て直しました", "「追加先」".
  When a text names another label, page or menu, it quotes that label's
  Japanese text exactly ("「ウィンドウを新しく開いたビデオに合わせる」が
  オンのとき (「再生」ページ)"); menu paths keep the source's ">" between
  the Japanese labels ("ビデオ > 下の余白"). Square, angle and curly
  brackets stay as in the source, half-width, also around Japanese words
  ("([...] と (...))", "<既定のロゴ>", "[スキンで定義]", "{最初のロゴと
  同じ}"); a word inside them that describes a value is translated
  ("ggml-<モデル>.bin", "mpvSockets/<プロセス ID>").
- **"..." and "…".** The end of a menu item or button keeps exactly what
  the source has: three ASCII periods "..." where the source has them
  (the menus), the single character "…" where it has that (the QML
  windows), attached to the last word with no space ("ファイルを開く...",
  "参照…", "%1 を測定中…"). Where the source has a space before it
  ("Shutdown at ...") the space stays. Japanese prose's doubled "……" is
  not used.
- **Keys** keep RVP's own names, the ones its menus print: Ctrl, Shift,
  Alt, Enter, Space, Esc, Tab, PgUp, PgDn, Home, End, Del, Ins,
  F1...F12, Num+, the arrow keys' Left, Right, Up and Down inside key
  names and lists of keys ("Ctrl+Left / Right"), and the letters and
  punctuation keys as in the source. A key takes で or を with a space:
  "Esc でキャンセルします。", "Enter を押してください". In running text
  about keys in general, Japanese words: "矢印キー", "メディアキー",
  "キーボードの音量キー". The gamepad's buttons (A, B, X, Y, Back, Start)
  and the mouse's X1 and X2 keep their labels; the shoulder buttons are
  "ショルダーボタン", the pad "方向パッド", and the mouse's back and
  forward buttons "「戻る」ボタン" and "「進む」ボタン".
- **Units.** Seconds "秒", minutes "分", hours "時間": "Sec", "Sec.",
  "s", "Min", "min" and "h" after a number become 秒, 分 and 時間 ("%1
  秒戻る", "%1 秒", "%1 分", "%1 時間", "30 秒の区切り"). ms stays "ms" in
  labels ("%1 ms"); where the source spells out "milliseconds" it is
  "ミリ秒" ("ミリ秒単位の音声同期"). dB, Hz, kHz, fps, px, K (4K), p
  (1080p), nits, GB, MB and W stay. A number and a Latin unit are always
  spaced ("%1 ms", "%1 Hz", "%1 dB", "%1 GB", "%1 nits"), also where the
  source writes "%1ms" or "%1Hz"; the percent sign and the degree sign
  attach to the number ("%1%", "100%", "%1°", "360°"), also where the
  source writes "%1 %". Multipliers become 倍 ("%1x video output size"
  "ビデオ出力サイズの %1 倍", "%1x Forward" "%1 倍速で早送り"); sizes and
  grids keep "x" ("%1x%2", "3x2"). Bits are "ビット" ("16 ビット",
  "%1 ビット (推奨)", "10 ビット出力"). "Frames a second" is "1 秒あたりの
  フレーム数" in sentences and "fps" in tight labels.
- **Numbers, counters and placeholders.** Numbers are half-width Arabic
  digits with the source's decimal point ("1.5 秒", "0.5"); kanji numerals
  appear only inside fixed words (一時停止, 一覧, 一部, 一度に). Notation
  that is a name or that RVP parses keeps the source's form: channel
  layouts (5.1, 7.1), aspect ratios and sizes (16:9, 2.35:1, %1:%2,
  %1.%2), versions, ranges, input formats ("h:mm:ss.s", "hh:mm"). Japanese
  has no plural: "(s)" disappears ("Open File(s)..." "ファイルを開く...")
  and the source's separate singular and plural strings get the same
  text ("Reserve %1 line" and "Reserve %1 lines" both "%1 行分を確保").
  Counts take a counter with a space after the number: 個 (files, items,
  things: "%1 個のファイル"), 行 (lines), 列 (columns), 枚 (images,
  pictures), 台 (devices, cards), 回 (times), 本 (fingers: "1 本指"),
  段階 (steps), チャンネル (channels). Ordinals are "～番目" or, for
  devices, "～台目": "%1nd Device" "%1 台目のデバイス", "2nd Audio Renderer"
  "2 台目の音声レンダラー", "{Same as %1st logo}" "{%1 番目のロゴと同じ}";
  "Style #%1" keeps its "#" ("スタイル #%1"). Placeholders stay where the
  sentence needs them; the particle after one must suit every value it
  can take (a name, a number, a page's title): "%1 を", "%1 の", "「%1」",
  or after a colon ("使用中: %1").
- **Values stay as they are.** Example and default values that RVP
  matches or parses keep the source's text: chapter titles to skip
  ("Opening;Ending;3"), language codes ("rus,eng", "en,ru", "en"), drive
  letters and mount points ("V:", "/mnt/media"), the yt-dlp format
  ("bestvideo+bestaudio/best"), file patterns ("*.dsf", "*.json"),
  extension lists ("mkv mp4 ..."), the default folders RVP really uses
  ("Music/RVP"; "Pictures/RVP in the home folder" is "ホームフォルダーの
  Pictures/RVP", "Videos/RVP in the home folder" "ホームフォルダーの
  Videos/RVP"), and the playlist's album named "Default" (data, shown as
  「Default」). Whisper's model names (tiny, base, small, medium, large,
  large-v%1-turbo) stay; their notes are Japanese ("(未ダウンロード)").
- **Language names** are the usual Japanese names with 語, though the
  source writes them in lowercase: "英語", "中国語", "広東語", "アルバニア
  語", "ハイチ・クレオール語" (the one place "・" joins two words of a
  name); a language without a usual Japanese name is its katakana form
  with 語. The language's own name (the Language context) is "日本語".
- **Length.** README's rule counts each Japanese character as two Latin
  letters: a short label may be about a third wider than the English. Kanji
  labels fit easily; katakana runs long, which is the main risk.
  - Prefer the short kanji term where it is usual (see above), and the
    glossary's term over a longer synonym.
  - Drop what the English needs and Japanese does not: articles,
    possessives and pronouns ("Show the mouse pointer" "マウスポインター
    を表示", "Remember the A-B repeat lists" "A-B リピートリストを記憶"),
    and a の between nouns where the compound is usual ("音声同期",
    "字幕同期", "下の余白" but "画面余白").
  - Never する or ます in a label (see Checkboxes); "～するとき" becomes
    "～時" ("再生終了時").
  - An explanation in parentheses may be shortened when the label stays
    clear: "(Recommended)" and "(Recom'd)" are "(推奨)". Japanese uses no
    other abbreviations in labels.
  - Where the source's label repeats its heading ("Enable (Shift+N)"
    under "Normalizer"), the Japanese repeats nothing either: "有効にする
    (Shift+N)".
- **Words with two meanings in the source.** A file's extension is
  "拡張子" (the association table's "Extension", sorting by extension);
  extending the picture is "拡張" ("Extend/Crop" "拡張/トリミング").
  Apply is "適用" for a filter and a setting alike. "Remember" is "記憶",
  "store" and "save" "保存", "keep" "維持" or "保持". The "nav bar" and
  "navigation bar" of PotPlayer's menus are the seek bar ("シークバー").
  "Default" as a value is "既定" ("(既定)", "既定に戻す", "mpv の既定");
  "RVP Default", the skin's name, stays. "Center" / "Centre" is "中央" as
  a position or alignment and "センター" as the speaker and its channel
  ("センターチャンネル"). "Display" is "ディスプレイ" as a monitor and
  "表示" as showing. "Close" is "閉じる" (windows, dialogs, panels), "Exit"
  "終了" (RVP itself: "RVP を終了"), and "Close Playback" "再生を終了" (see
  the glossary). An action bound to a key or gesture is "操作"; the action
  done when playback finishes (shut down, sleep...) is "動作" ("再生終了時の
  動作"). "Card" alone is never "カード": it is "グラフィックスカード" or
  "GPU". The RVP state folder is "RVP の状態フォルダー".
- **Recurring words outside the glossary**, kept the same throughout:
  item 項目, track トラック, mode モード, effect エフェクト, filter
  フィルター, script スクリプト, plugin プラグイン, runtime ランタイム,
  library ライブラリ, home folder ホームフォルダー, mouse pointer
  マウスポインター, cursor カーソル, screen 画面, desktop デスクトップ,
  clipboard クリップボード, webcam Web カメラ, interface language 表示言語,
  priority 優先度, instance インスタンス, focus フォーカス, startup 起動時,
  message メッセージ, warning 警告, error エラー, level レベル, strength
  強さ, threshold しきい値, radius 半径, luma 輝度, chroma 色差, range 範囲,
  tone mapping トーンマッピング, metadata メタデータ, keyframe キーフレーム,
  interlaced インターレース, field フィールド, gain ゲイン, mixer ミキサー,
  headphone ヘッドホン, slideshow スライドショー, waveform 波形,
  spectrogram スペクトログラム, album art アルバムアート, width / height
  幅 / 高さ, horizontal / vertical 水平 / 垂直, stretch 引き伸ばし, send
  to the Recycle Bin ごみ箱に移動.
- **Equalizer presets and anaglyph modes.** Genre names are katakana
  ("クラシック", "ポップ", "ロック", "ソフトロック", "クラブ", "ダンス",
  "テクノ", "スカ", "レゲエ", "パーティー", "ライブ"); the descriptive ones
  are Japanese ("大ホール", "低音強調", "高音強調", "低音と高音を強調",
  "ソフト", "ノート PC", "ラウドネス"). Anaglyph colours are Japanese or
  their usual katakana ("赤/シアン", "緑/マゼンタ", "黄/青", "赤/青",
  "赤/緑"; "グレー", "カラー", "ハーフカラー"); "Dubois" stays.

## Glossary

| English | Japanese | Note |
|---------|----------|------|
| play / pause / stop | 再生 / 一時停止 / 停止 | The commands are these nouns; the toggle is "再生/一時停止". Never shorten 一時停止 to 停止, which is stop. |
| playback | 再生 | The Playback menu and page; "再生位置", "再生速度", "再生中". |
| close playback | 再生を終了 | Stops and unloads the file, disc or stream: "再生を終了", "Back で再生を終了". Exit (RVP itself) is "終了" / "RVP を終了"; "閉じる" is for windows and dialogs. |
| previous / next | 前 / 次 | With a noun "前の…" / "次の…" ("次のチャプター", "前のフレーム"); alone as buttons "前へ" / "次へ". |
| seek | シーク | The noun and the verb ("シーク時に一時停止"); the seek commands are "%1 秒進む" / "%1 秒戻る", fast motion "早送り" / "巻き戻し". Never 検索, which is search. |
| jump | ジャンプ | "時間/フレームを指定してジャンプ...", "次のスライドへジャンプ"; the Time Jump page is "ジャンプ". Windows' jump list is "ジャンプリスト". |
| frame step | コマ送り | Forward "コマ送り", backward "コマ戻し"; the commands are "次のフレーム" / "前のフレーム". コマ appears only in these two words; a frame is otherwise フレーム. |
| speed | 速度 | The playback rate: "再生速度". |
| repeat | リピート | "プレイリストをリピート", "ファイルをリピート", "リピート回数". Redo is "やり直し", never リピート. |
| A-B repeat / section repeat | A-B リピート / 区間リピート | Point A and point B are "A 点" / "B 点". |
| shuffle | シャッフル | "シャッフル再生". |
| queue | 再生キュー | The Playback Queue; "再生キューに追加". |
| resume / remember position | レジューム / 再生位置を記憶 | レジューム is the section heading; in sentences "前回の続きから再生". |
| position | 位置 | The time in the file: "再生位置"; a window's or a picture's position is also 位置. |
| duration | 長さ | A file's whole length: "長さ: %1", "ファイルの長さを検出". Not 再生時間, which also reads as time played. |
| elapsed / remaining time | 経過時間 / 残り時間 | As choices alone "経過時間", "残り時間", "経過時間と残り時間". |
| chapter | チャプター | |
| bookmark | ブックマーク | |
| skip (intro, ending) | スキップ (オープニング、エンディング) | The intro is "オープニング", the ending "エンディング": "オープニングを %1 秒スキップ". The value "Opening;Ending;3" stays. |
| seek bar | シークバー | Also PotPlayer's "nav bar" and "navigation bar". |
| thumbnail | サムネイル | |
| preview | プレビュー | "サムネイルプレビュー" on the seek bar. |
| scene browser | シーンブラウザー | |
| file | ファイル | |
| folder | フォルダー | With the long vowel mark, always. |
| URL / address | URL / アドレス | |
| stream | ストリーム | Both network media and a track inside a file ("音声ストリームを選択"); "Record Stream" is "ストリームを録画". |
| device / capture device | デバイス / キャプチャデバイス | "Web カメラ/その他のデバイスを開く". |
| screen capture | 画面キャプチャ | The desktop as a video source: "画面キャプチャを開く". |
| disc | ディスク | DVD, Blu-ray, audio CD; the drive is "ドライブ". |
| title | タイトル | A DVD's title and an entry's title alike. |
| angle | アングル | |
| disc menu | ディスクメニュー | The disc's own menus: "タイトルメニュー", "ルートメニュー", "チャプターメニュー". |
| drag and drop | ドラッグ&ドロップ | With the ASCII "&". |
| recent files | 最近のファイル | Recent items "最近の項目", recent URLs "最近の URL". |
| favorites | お気に入り | |
| file association / file type | ファイルの関連付け / ファイルの種類 | The Association page is "関連付け". RVP registers itself (登録) and never makes itself the default. |
| playlist | プレイリスト | The established spelling with イ, though the player is プレーヤー. Not 再生リスト. |
| album | アルバム | A tab of the playlist (a named list), not a music album; the album named "Default" stays 「Default」. |
| entry | 項目 | One item of the playlist; the source's "item" is the same word: "選択した項目を削除". |
| sort | 並べ替え | "並べ替え", "フォルダー順に並べ替え", "昇順 (A から Z)". |
| add / remove / delete | 追加 / 削除 / 削除 | Japanese says 削除 for removing an entry and for deleting a file: the object tells them apart. Removing names the entry or the list ("項目を削除", "プレイリストから削除"); deleting the file itself always names the file ("ファイルを削除") or the Recycle Bin ("ごみ箱に移動"). Never write a bare 削除 where a file could be meant. |
| recycle bin / trash | ごみ箱 / ごみ箱 | One word for Windows' and Linux's, in Windows' spelling with hiragana ごみ. "ごみ箱に移動". |
| import / export | インポート / エクスポート | |
| window / main window | ウィンドウ / メインウィンドウ | |
| fullscreen / windowed | 全画面 / ウィンドウ表示 | "全画面 (引き伸ばし)", "全画面で再生中"; not フルスクリーン. |
| maximize / minimize | 最大化 / 最小化 | Restore is "元のサイズに戻す". |
| always on top / stay on top | 常に手前に表示 / 常に手前に表示 | Both the same; the toggle is "常に手前に表示の切り替え". |
| opacity | 不透明度 | Transparency is "透明度". |
| skin | スキン | Video skin and audio skin: "ビデオスキン", "音声スキン". |
| color theme | カラーテーマ | "カスタムカラーテーマ", "HSB カラーテーマ". |
| logo | ロゴ | |
| title bar | タイトルバー | |
| control panel | コントロールパネル | RVP's F7 window, not the Windows Control Panel; the text never needs to say which. |
| OSD / on-screen display | OSD / オンスクリーン表示 | OSD is the usual word ("OSD メッセージ", "OSD フォント"); オンスクリーン表示 where the source spells it out. |
| on-screen controller | オンスクリーンコントローラー | mpv's controls; the Controller choice alone is "コントローラー". Different from the gamepad, ゲームパッド. |
| tooltip | ツールチップ | |
| context menu | コンテキストメニュー | Also the file and folder context menus of Explorer. |
| tray icon / notification area | トレイアイコン / 通知領域 | The tray alone is "トレイ" ("タスクバーとトレイ"). |
| taskbar | タスクバー | Also Linux's dock. |
| notification | 通知 | |
| dialog | ダイアログ | "Dialog Box" (the page) is "ダイアログ"; "modeless dialog boxes" "モードレスダイアログ". |
| settings / preferences | 設定 / 設定 | One word for the F5 window: "Preferences..." is "設定...", like the window's title. |
| page | ページ | |
| preset / configuration | プリセット / 構成 | The Configuration page is "構成"; "Configuration management" "構成の管理". |
| reset / default | リセット / 既定 | Reset "リセット" ("ページをリセット"); default "既定", the value "既定値", "Restore to Default" "既定に戻す". Not デフォルト. |
| apply | 適用 | A filter or effect and a changed setting alike. |
| key / hotkey / shortcut | キー / ホットキー / ショートカット | |
| mouse / wheel / button | マウス / ホイール / ボタン | |
| gesture | ジェスチャ | Without ー, as the desktops write it: "マウスジェスチャ". |
| touch | タッチ | "タッチ機能を有効にする", "1 本指のスワイプ". |
| controller / gamepad | コントローラー / ゲームパッド | "Xbox コントローラー". |
| video / picture / image | ビデオ / 映像 / 画像 | ビデオ for the Video menu and page, the video track and video files ("ビデオファイル"); 映像 for the picture as it shows (3D's left and right images are "左右の映像" too); 画像 for still images, saved frames and logo pictures. Not 動画. |
| frame | フレーム | |
| frame rate / fps | フレームレート / fps | |
| aspect ratio | アスペクト比 | The menus' "AR" is written out: "アスペクト比". |
| crop | トリミング | "アスペクト比でトリミング", "拡張/トリミング". |
| zoom / pan | ズーム / パン | Zoom In / Out are "ズームイン" / "ズームアウト". |
| rotate / flip / mirror | 回転 / 上下反転 / 左右反転 | Translate what happens: the source's "Flip Horizontal" mirrors left-right ("左右反転"), "Flip Vertical" turns upside down ("上下反転"). Never 水平反転 / 垂直反転, which read either way. |
| deinterlace | インターレース解除 | |
| brightness / contrast / saturation / hue / gamma | 明るさ / コントラスト / 彩度 / 色相 / ガンマ | |
| sharpen / denoise | シャープ化 / ノイズ除去 | The video's denoise; the audio's noise reduction is ノイズ低減. |
| scaler / upscaling / resize | スケーラー / アップスケール / サイズ変更 | Upscaler / downscaler "アップスケーラー" / "ダウンスケーラー", chroma scaler "色差スケーラー". |
| pixel shader | ピクセルシェーダー | |
| renderer | レンダラー | Video and audio renderers alike ("音声レンダラー"). |
| hardware decoding / decoder | ハードウェアデコード / デコーダー | Software decoding "ソフトウェアデコード". |
| HDR | HDR | |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360° ビデオ / アナグリフ / サイドバイサイド / トップアンドボトム | The source's abbreviations SBS and TAB stay beside them: "SBS (サイドバイサイド)". |
| bars / letterbox / margin | 黒帯 / レターボックス / 余白 | The black areas around the picture. The skin's bars are "バー" and spectrum bars "スペクトラムバー": not 黒帯. |
| frame capture / screenshot | フレームキャプチャ / スクリーンショット | The Video Capture page is "ビデオキャプチャ"; "Save Current Screen Frame" "現在の画面フレームを保存". |
| motion smoothing | モーションスムージング | Smoothing alone "スムージング"; the checkbox "Smooth motion" is "動きを滑らかにする". |
| frame interpolation | フレーム補間 | |
| engine | エンジン | |
| calibration | 性能測定 | What the calibration does: measures this computer. "Calibrate…" is "性能測定…", "Recalibrate…" "再測定…". Not キャリブレーション, which Japanese users know from monitors' colours. |
| quality preset (light, medium, high) | 品質プリセット (軽量、中、高) | The engines' presets Fast / Medium / Quality are "高速" / "標準" / "高品質". |
| GPU / graphics card | GPU / グラフィックスカード | "the card" is "GPU" in tight places, never a bare カード. |
| processor / CPU | プロセッサー / CPU | |
| power budget | 電力上限 | The share of the card's power RVP may use: "電力上限を超えると、RVP は自身の処理を軽くします". It never changes the card's own power limit. |
| dropped frames | フレーム落ち | Not ドロップフレーム, which is the timecode term. "フレームの 5% 以上が落ちる". |
| refresh rate | リフレッシュレート | The display's rate (Hz). |
| audio / sound | 音声 / 音 | 音声 for the Audio menu and page and in compounds ("音声トラック", "音声エフェクト", "音声デバイス", "音声ファイル"); 音 for the sound in prose ("音なし"). An audio CD is "オーディオ CD", the established name. |
| volume / mute | 音量 / ミュート | |
| master volume | マスター音量 | The system's volume: "システムのマスター音量". |
| equalizer | イコライザー | |
| normalizer | ノーマライザー | |
| reverb | リバーブ | |
| noise reduction | ノイズ低減 | The audio effect; the video's is ノイズ除去. |
| channel / speaker | チャンネル / スピーカー | "左チャンネル", "センターチャンネル", "出力スピーカー". |
| downmix | ダウンミックス | |
| surround / stereo | サラウンド / ステレオ | |
| output device | 出力デバイス | "既定の音声デバイス (推奨)". |
| sample rate / bitrate | サンプルレート / ビットレート | |
| audio delay / audio sync | 音声の遅延 / 音声同期 | "音声同期の調整...", "ミリ秒単位の音声同期 (遅らせると正)". |
| pitch / time stretching | ピッチ / タイムストレッチ | Pitch shifting "ピッチシフト". |
| audio recording | 録音 | Recording audio is 録音, recording video 録画 ("ビデオを録画..."). |
| second audio device | 2 台目の音声デバイス | The page's "Second Device" is "2 台目のデバイス". Never 副音声, which is a TV broadcast's second audio track. |
| pass-through | パススルー | |
| subtitles / subtitle track | 字幕 / 字幕トラック | |
| external / embedded subtitles | 外部 / 内蔵字幕 | External subtitles are "外部字幕", embedded ones "内蔵字幕"; not 埋め込み字幕, which reads as burned in. "Load External Audio..." is "外部音声を読み込む...". |
| font / outline / shadow | フォント / 縁取り / 影 | "縁取りの幅", "影のオフセット". |
| subtitle position / subtitle delay | 字幕の位置 / 字幕の遅延 | Subtitle sync is "字幕同期". |
| encoding | 文字コード | A text file's character set ("文字コード", "UTF-8 以外のファイル"). Encoding video or sound is "エンコード" ("AC3 再エンコード"). |
| subtitle browser / subtitle editor | 字幕ブラウザー / 字幕エディター | |
| subtitles from audio / speech recognition | 音声から字幕 / 音声認識 | "音声から字幕を作成...". |
| model | モデル | |
| language | 言語 | The interface language is "表示言語". |
| remote control | リモート操作 | The page and the feature; a remote program on a phone (Unified Remote's) is "リモコン" ("Unified Remote 用リモコン"). |
| token | トークン | |
| port / server | ポート / サーバー | |
| log | ログ | |
| crash | クラッシュ | |
| download / install | ダウンロード / インストール | |
| installer | インストーラー | |
| accessibility | アクセシビリティ | |
| text to speech (TTS) / screen reader | テキスト読み上げ (TTS) / スクリーンリーダー | Reading aloud is "読み上げ" ("画面のメッセージを読み上げ"). |
| screen saver | スクリーンセーバー | |
| shut down / sleep / hibernate | シャットダウン / スリープ / 休止状態 | |
| timer | タイマー | The Shutdown Timer is "シャットダウンタイマー". |
| visualization / spectrum | ビジュアライザー / スペクトラム | ビジュアライザー for the feature too ("ビジュアライザーの種類"); "スペクトラムバー". |
| status bar / info | ステータスバー / 情報 | "再生情報", "コーデック情報". |
| codec / format | コーデック / 形式 | "PNG 形式で保存", "出力形式". |
| bit depth / 10-bit | ビット深度 / 10 ビット | "10 ビット出力". |
| channel layout (5.1, 7.1) | チャンネル構成 (5.1、7.1) | |
| Auto select (recommended) | 自動選択 (推奨) | The same for every spelling of the source ("(Recommended)", "(recommended)"). |
