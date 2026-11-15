# Chinese, Traditional (zh_TW)

Traditional Chinese as written in Taiwan: Taiwan's standard character
forms (the Ministry of Education's 標準字體) and Taiwan's own computing
vocabulary, the words that Windows, GNOME and KDE in Taiwan and the
media players people use there have made familiar (檔案, 資料夾, 影片,
音訊, 設定, 預設, 播放清單, 字幕). RVP shows the same text to users in
Hong Kong and Macau when it has no language of theirs; it stays
Taiwan's, with no Cantonese and no Hong Kong-only words or character
forms, since those readers know Taiwan's software terms. The
translation is written for RVP from the English, with the Russian as a
second reference for meaning. It is not a conversion of the Simplified
Chinese translation: the mainland's terms differ (文件, 視頻, 設置,
默認, 質量, 信息...), and converting characters one by one leaves them
in. The tone is that of a desktop program's own settings: plain,
polite, short, and impersonal unless the source speaks to the user.

## Style

Examples are set in 「」, or in backticks where they contain 「」
themselves.

- **Address.** Use 您 where the source speaks to the user and the
  sentence needs a subject ("files that you download" 「您下載的檔案」);
  most strings need none. Never 你. Use 請 only where the text asks the
  user to do something now ("Press a key for “%1” (%2). Esc cancels."
  `請按下要指定給「%1」的按鍵（%2）。按 Esc 取消。`); never in labels,
  and not in help texts that only describe.
- **Help texts** are declarative sentences in the present tense, with
  the program or the setting as the subject, as in the source
  (「RVP 會測量…」, 「此選項只在全螢幕時生效。」). Avoid translationese:
  no 進行 or 對…進行… as a dummy verb, no 被 passive where an active
  verb or a topic works, 「…時」 rather than 「當…的時候」, no 一個 before
  every noun, no chains of 的. A long English sentence may become two
  Chinese ones, and its parts may change order, as long as nothing is
  lost.
- **Questions, notices and progress.** Confirmation questions take
  「要…嗎？」: 「要將 %1 個檔案移至資源回收筒嗎？」,
  `要將「%1」的選項還原為 RVP 的預設值嗎？`. Status notices take 已:
  「已套用。」, 「已取消。」, 「已匯入。」, 「已匯出。」; "Unchanged." is
  「未變更。」. Progress texts take 正在 and keep the source's ellipsis:
  「正在啟動…」, 「正在測量 %1…」. Status words that RVP puts into
  another string ("checking", "unpacking", "downloading" in "RIFE
  (TensorRT): %1, %2 of %3 GB") are the short progressive forms
  「檢查中」, 「解壓縮中」, 「下載中」: check the receiving string.
- **Menu items and buttons** are commands: verb first, then the object,
  short, no 請, as few 的 as possible (「開啟檔案...」, 「另存播放清單...」,
  「新增書籤」, 「重新載入字幕」). The standard desktop words: Open 開啟,
  Save 儲存, Save As 另存新檔 (with an object 另存…: 「另存字幕...」),
  Close 關閉, Exit 結束, Cancel 取消, OK 確定, Apply 套用, Browse 瀏覽,
  Choose 選擇, Select 選取, Select All 全選, Deselect All 取消全選,
  Copy 複製, Paste 貼上, Delete 刪除, Rename 重新命名, Undo 復原,
  Redo 重做, Reset 重設, Restore 還原, Reload 重新載入, Rescan
  重新掃描, Find 尋找, Search 搜尋, Customize 自訂, About 關於, Show
  顯示, Hide 隱藏, Enable 啟用, Disable 停用, Use 使用, Remember 記住,
  Load 載入, Record 錄製, Capture 擷取, Cycle and Toggle 切換, Set 設定,
  Edit 編輯, Insert 插入, Split 分割, Join 合併, Test 測試, Accept 接受.
  Increase / Decrease are 增加 / 減少, and 提高 / 降低 for volume,
  brightness and other levels; a window's or font's size is 放大 / 縮小.
- **Checkboxes** are verb phrases that say what happens when the box is
  checked, without 是否: 「記住播放位置」, 「顯示工具提示」,
  「播放時防止螢幕保護程式啟動」, 「不要加入重複的檔案」.
- **Choices** in lists are short nouns, adjectives or adverbs: 永不
  (Never), 一律 (Always), 播放時, 全螢幕播放時, 自動, 無 (None), 預設,
  自訂, 高品質. On / Off as choices are 開啟 / 關閉; Enabled / Disabled
  as states 已啟用 / 已停用. Check that every choice of one list reads
  right after its label.
- **Page titles, headings, window titles and field labels** are nouns or
  noun phrases (「播放」, 「時間跳轉」, 「區段重複」, 「長寬比」,
  「字型大小」). A label before a field ends with a colon only where the
  source has one, and that colon is full-width 「：」.
- **Taiwan's words, not the mainland's.** 檔案 (not 文件, which is a
  document in Taiwan), 資料夾 (文件夾), 影片 (視頻), 音訊 (音頻), 設定
  (設置), 預設 (默認), 品質 (質量, which is mass), 資訊 (信息), 訊息
  (消息), 網路 (網絡), 程式 (程序, which is a procedure), 軟體 / 硬體
  (軟件 / 硬件), 支援 (支持), 載入 (加載), 執行 (運行), 螢幕 (屏幕),
  滑鼠 (鼠標), 視窗 (窗口), 選單 (菜單), 清單 (列表), 介面 (界面),
  解析度 (分辨率), 位元 (比特), 字型 (字體), 字元 (字符), 文字 (文本),
  自訂 (自定義), 進階 (高級), 一般 (常規), 偵測 (檢測), 最佳化 (優化),
  快取 (緩存), 記憶體 (內存), 執行緒 (線程), 伺服器 (服務器), 連接埠
  (端口), 使用者 (用戶), 副檔名 (擴展名), 匯入 / 匯出 (導入 / 導出),
  開啟 (打開), 結束 (退出), 貼上 (粘貼), 復原 (撤銷), 套用 (應用), 停用
  (禁用), 還原 (恢復), 數位 (數字), 訊號 (信號), 顯示卡 (顯卡), 音效卡
  (聲卡), 喇叭 (揚聲器), 圖示 (圖標), 縮圖 (縮略圖), 剪貼簿 (剪貼板),
  工作列 (任務欄), 延展 (拉伸), 置中 (居中), 填滿 (填充), 擷取 (捕獲,
  採集), 裝置 (設備), 光碟 (光盤), 連結 (鏈接), 線上 (在線), 授權
  (許可證), 可攜式 (便攜), 應用程式 for an app (應用), 執行個體 for a
  program's instance (實例), 偵錯 (調試), 外掛程式 for a plugin (插件),
  篩選 for filtering a list.
- **Character forms** are Taiwan's standard ones: 裡 (not 裏), 線 (綫),
  為 (爲), 啟 (啓), 說 (説), 內 (内), 值 (値), 錄 (録), 眾 (衆), 衛 (衞),
  溫 (温), 群 (羣), 著 (着: 著色器), 卻 (却), 黃 (黄), 台 in everyday
  words (這台電腦). Characters that conversion tools confuse must be the
  right ones: 復原 (undo), 複製 (copy), 重複 (repeat), 覆寫 (overwrite);
  後 (after), 製 (make) and 制 (control), 範圍, 準確, 遊戲, 鐘, 乾, 鬆,
  系統 and 關係.
- **Punctuation.** Chinese text takes full-width punctuation: ，、。；：
  ？！（）「」『』. 、 separates listed items, ， clauses; the source's
  semicolons and colons become ； and ：. A sentence ends with 。 where
  the source ends with a period; a label without one gets none. Dashes
  become ：, ， or parentheses (no ——). Parentheses are full-width in
  Chinese text, also around keys and Latin words: 「去交錯（Ctrl+I）」,
  「自動選擇（建議）」, 「設定起點（A）」. Half-width stays for "/" between
  alternatives (播放/暫停, with spaces where the source has them), "+"
  in key names, "-" in ranges (0-255), ":" in times and ratios (h:mm,
  16:9), "." in numbers, the bracket marks of the labels
  "[Skin defined]", "<Default logo>" and "{Same as %1st logo}"
  (「[由外觀定義]」, 「<預設標誌>」), and everything inside names, paths
  and codes. File-type filters keep their ASCII parentheses with a
  space before them, because Qt's file dialog reads the patterns from
  them: 「PotPlayer 外觀 (*.dsf)」, 「設定 (*.json)」, 「所有檔案 (*)」.
  Menu paths keep the source's " > " between the Chinese labels:
  「影片 > 延伸/裁切 > 移動」. The one source string with a line break
  ("%1\nin %2 seconds") keeps it; no others get one.
- **"..." and "…".** The end of a menu item or button keeps exactly what
  the source has: three periods "..." where it has them (the menus),
  the single character "…" where it has that (the QML windows),
  attached to the last Chinese character with no space, also where the
  source has a space before it ("Shutdown at ..." 「定時關機...」).
  Taiwan's Windows and desktops mark a command that opens a dialog the
  same way. The Chinese ellipsis 「……」 (two characters) marks omitted
  text in prose, which these strings do not have, so it is not used. A
  "..." that means "and so on" in a list becomes 等: 「（http、https、rtmp
  等）」. The "[...]" and "(...)" of the hearing-impaired notes are the
  patterns themselves and stay as written.
- **Quotation marks** are 「」, with 『』 inside them, for both the
  source's “…” and its straight "...". They are the marks of Taiwan's
  official punctuation standard (教育部《重訂標點符號手冊》) and of
  Taiwan's Windows, GNOME and KDE; “” is the mainland's GB standard and
  reads as such. When a text names another label, page or menu, it
  quotes that label's Chinese text exactly: `在檔案右鍵選單中加入「加入至」`,
  `「視窗配合新開啟的影片」開啟時套用（「播放」頁面）。`; a quotation
  inside a quoted text takes 『』.
- **Spacing.** One half-width space between Chinese characters and
  Latin letters, digits or a placeholder, on both sides, the usual
  practice of Taiwan's current software and documentation: 「RVP 的設定」, 「以 PNG 格式儲存
  圖片」, 「快轉 %1 秒」, 「%1 個檔案」, 「3D 影片」, 「按 Shift+Enter
  換行」, 「mpv 的 OSC」. No space between any character and full-width
  punctuation (`RVP。`, `（Ctrl+I）`, `「%1」`), none around "/" unless
  the source has one, none inside "%1%", "%1x", "%1:%2" or "1080p".
  Placeholders always take the spaces of a Latin word, since most stand
  for numbers, names, keys or paths.
- **Units.** Seconds (s, Sec, Sec.) are 秒, minutes (min, Min) 分鐘,
  hours 小時 (「快轉 %1 秒」, 「倒轉 %1 分鐘」, 「略過片頭 %1 秒」). ms
  stays "ms", as Taiwan's audio, video and game settings write it; a
  spelled-out "milliseconds" is 毫秒. dB, Hz, kHz, fps, px, K, p (1080p),
  GB, MB and W stay, with a space after the number: "%1 ms", "%1 Hz",
  "60 fps", also where the source writes "%1ms" or "%1Hz". The percent
  sign, the degree sign and the multiplier x attach to the number
  (「%1%」, 「5%」 also where the source writes "5 %", 「360°」, 「%1x」,
  「1.0x」); "degrees" written out is 度 (「旋轉 %1 度」). "Frames a
  second" is 「每秒影格數」, or fps in a tight label.
- **Numbers.** Arabic numerals for values, sizes, times and counts with a
  unit; Chinese numerals for small counts written out in prose ("one to
  three minutes" 「一到三分鐘」, "twice" 兩倍, "a fifth to spare"
  「保留五分之一的餘裕」). The decimal separator is a point; no thousands
  separator. Notation that is a name or that RVP parses keeps the
  source's form: channel layouts (5.1, 7.1), ratios and sizes (16:9,
  2.35:1, %1:%2, %1.%2, 3x2, %1x%2), versions, ranges (0-255, 16-235),
  input formats (h:mm:ss.s, hh:mm). A range of values is 「480p 到
  2160p」. Ordinals are 第 + number + measure word: "%1nd Device"
  「第 %1 個裝置」, "%1st logo" 「第 %1 個標誌」, "Second Device"
  「第二裝置」. Bits: "%1-bit" 「%1 位元」, "10-bit output" 「10 位元輸出」.
- **Measure words and plurals.** A number before a noun needs a measure
  word, written after the placeholder: 個 by default (檔案, 項目, 裝置,
  影格, 聲道 when counted: "at least %1 chs" 「至少 %1 個聲道」), 張 for
  images, pictures and thumbnails, 行 for lines of text ("Reserve %1
  lines" 「保留 %1 行」), 列 for rows, 欄 for columns, 次 for times and
  repeats, 段 for pieces and sections. Units need none (「%1 秒」,
  「%1 位元」, 「%1 度」), nor does a channel count that names a layout
  (「%4 聲道」). Chinese has no plural: "file(s)" is 檔案, and the
  source's singular and plural strings translate alike ("Reserve %1
  line" and "Reserve %1 lines" are both 「保留 %1 行」).
- **Rows, columns and lines.** In Taiwan 列 is a row and 欄 a column (the
  mainland's 行 and 列 say the opposite); a line of text is 行. "Row
  interleave" 「逐列交錯」, "Column interleave" 「逐欄交錯」, "Columns" /
  "Rows" 「欄數」 / 「列數」. Bars of a window are 列 too (工作列,
  標題列, 狀態列, 進度列), never 欄 or 條.
- **Keys and the mouse.** Keys keep RVP's own names, the ones its menus
  print: Ctrl, Shift, Alt, Enter, Space, Esc, Tab, PgUp, PgDn, Home,
  End, Del, F1...F12, the arrow keys' Left, Right, Up and Down inside key
  names and lists of keys ("Ctrl+Left / Right" stays as written), and
  the letters and punctuation keys as in the source. In running text
  about keys in general, Chinese words: 方向鍵, 媒體鍵, 「鍵盤的音量鍵」.
  The gamepad's A, B, X, Y, Back and Start and the mouse's X1 and X2
  keep their labels; the mouse's back and forward buttons are 上一頁鍵
  and 下一頁鍵, the gamepad's shoulders 肩鍵, its sticks 搖桿, its pad
  方向鍵. Pressing a key is 按 (「按 Enter」); a click is 按一下, a double
  click 按兩下, the left and right buttons 左鍵 and 右鍵; the pointer is
  指標, the cursor 游標, dragging 拖曳.
- **Names, possessives and values.** Names stay as README lists them,
  with the spaces above. "RVP's" is 「RVP 的」, or a compound where it
  stays clear (「PotPlayer 外觀」, 「SVP 濾鏡」); "RVP's own" is
  「RVP 自己的」. Values that RVP matches or parses keep the source's
  text: chapter titles to skip ("Opening;Ending;3"), language codes
  ("rus,eng", "en,ru", "en"), drive letters and mount points ("V:",
  "/mnt/media"), the yt-dlp format, file patterns, the folders RVP really
  uses ("Music/RVP"; "Pictures/RVP in the home folder" is
  「個人資料夾中的 Pictures/RVP」), and the playlist's album named
  "Default" (data, shown as 「Default」). Whisper's model names (tiny,
  base, small, medium, large...) and engine names stay; their notes are
  Chinese ("(not downloaded)" 「（未下載）」, "(not installed)"
  「（未安裝）」).
- **Language names** (the speech languages, which the source writes in
  lowercase) are the names of Taiwan's language lists: 「…文」 where
  Taiwan writes so (英文, 中文, 日文, 韓文, 法文, 德文, 西班牙文, 俄文,
  阿拉伯文, 越南文, 泰文), 「…語」 where that is the usual name (粵語),
  and Taiwan's transliterations of places (義大利文, 克羅埃西亞文,
  斯洛維尼亞文, 喬治亞文, 亞塞拜然文, 波士尼亞文, 威爾斯文, 寮文). The
  language's own name (the Language context) is 「繁體中文」, which
  readers in Taiwan, Hong Kong and Macau all recognise.
- **Words with two meanings in the source.** A file's extension is
  副檔名, extending the picture 延伸 ("Extend/Crop" 「延伸/裁切」). A
  frame in time is 影格, the picture's area (Extend/Crop, Resize, "video
  frame") 畫面. Black bars are 黑邊, the skin's bars 控制列. A saved set
  of all settings is 設定檔, an equalizer's, frame size's or engine's
  preset 預設集. Network media is 串流, a track inside a file 軌.
  Recording video or a stream is 錄製 and recording sound 錄音; what
  the log writes down is 記錄. "Correct" is 校正, "calibrate" 校準. mpv's, FFmpeg's and RVP's
  filters, of the picture and of the sound, are 濾鏡; a frequency filter
  (HPF) is 濾波器; DirectShow's filters are 篩選器. "Default" as a value
  is 預設 or 預設值; "RVP Default", the skin's name, stays. The "nav bar"
  and "navigation bar" of PotPlayer's menus are the seek bar 進度列.
  "Remember" is 記住, "store" and "save" 儲存. The home folder is
  個人資料夾, RVP's state folder 狀態資料夾, portable mode 可攜式模式.
- **Equalizer presets and anaglyph modes.** Genre names take the usual
  Chinese ones (古典, 俱樂部, 舞曲, 派對, 流行, 雷鬼, 搖滾, 抒情搖滾) and
  stay Latin where Taiwan has none in common use (Ska, Techno); the
  descriptive ones are Chinese (現場, 大廳, 筆電, 柔和, 重低音, 重高音,
  重低音與高音). Anaglyph colours are 紅/青, 綠/洋紅, 黃/藍, 紅/藍,
  紅/綠, with 灰階, 彩色 and 半彩色; Dubois stays. SBS and TAB stay as
  abbreviations beside the Chinese.
- **Length.** README's rule counts each Chinese character as two
  letters; Chinese labels are usually shorter than the English, and
  should stay so. Prefer the established two-character words, and drop
  what the English needs and Chinese does not: articles, possessives,
  的, 一個, 這個, 您的, 進行, 功能 ("Enable the touch feature" 「啟用
  觸控」). "(Recommended)" and "(Recom'd)" are 「（建議）」. Help texts
  may run as long as clarity needs.
- **"&"** is 與 in labels (「新增與編輯」, 「重低音與高音」) and 和 or 與
  in prose; it stays "&" only inside names.

## Glossary

| English | Chinese (Traditional) | Note |
|---------|-----------------------|------|
| play / pause / stop | 播放 / 暫停 / 停止 | The commands and buttons; "Play/Pause" is 播放/暫停. |
| playback | 播放 | The Playback menu and page 「播放」; 播放位置, 播放速度, 「播放時」 (while playing). Never 回放, which is a replay. |
| close playback | 關閉播放 | Stops and unloads the file; "Exit" (RVP itself) is 結束. |
| previous / next | 上一個 / 下一個 | With a noun: 上一個項目, 下一個章節, 下一個影格. |
| seek | 跳轉 | Moving the position ("after a start or a seek" 「開始播放或跳轉後」; with the mouse 「拖曳進度」). Step seeks are 快轉 / 倒轉: "%1 Sec Forward" 「快轉 %1 秒」, "%1 Min Backward" 「倒轉 %1 分鐘」. Never 搜尋, which is search. |
| jump | 跳至 | Before a target: "Jump to Specified Time/Frame..." 「跳至指定時間/影格...」, the button "Jump" 「跳至」. The noun ("Time Jump", "exact jumps") is 跳轉, as seek. Windows' jump list is 跳躍清單. |
| frame step | 逐格播放 | The commands are 下一個影格 / 上一個影格. |
| speed | 速度 | The playback rate 播放速度; the value keeps "1.0x". |
| repeat | 重複 | 重複播放, 重複播放清單; never 循環, the mainland's loop. Redo is 重做. |
| A-B repeat / section repeat | A-B 重複 / 區段重複 | Its marks are 起點 (A) and 終點 (B): 「設定起點（A）」. |
| shuffle | 隨機播放 | "Randomly" is 隨機. |
| queue | 播放佇列 | 佇列 is Taiwan's queue (not 隊列): 「加入播放佇列」, 「清除播放佇列」. |
| resume / remember position | 續播 / 記住播放位置 | The page "Resume" is 「續播」; in help texts 「從上次停止處繼續播放」. |
| position | 位置 | 播放位置 where a window's or the subtitles' position could be meant. |
| duration | 長度 | A file's whole length (the playlist's column, 「長度：%1」); not 時長. |
| elapsed / remaining time | 已播放 / 剩餘時間 | Choices 「已播放」, 「剩餘」, 「已播放與剩餘」. |
| chapter | 章節 | "Chapter Menu" 「章節選單」. |
| bookmark | 書籤 | |
| skip (intro, ending) | 略過（片頭、片尾） | 略過, as Taiwan's Windows writes skip (not 跳過): 「略過片頭 %1 秒」, 「略過片尾」, 「略過章節」. |
| seek bar | 進度列 | Also PotPlayer's "nav bar" and "navigation bar". |
| thumbnail | 縮圖 | Never 縮略圖. |
| preview | 預覽 | "thumbnail previews" 「縮圖預覽」. |
| scene browser | 場景瀏覽器 | |
| file | 檔案 | Never 文件, a document in Taiwan. File name 檔名, file extension 副檔名. |
| folder | 資料夾 | Never 文件夾. The home folder is 個人資料夾. |
| URL / address | 網址 / 網址 | Both are the address to open ("Open URL..." 「開啟網址...」). The address the remote control listens on (127.0.0.1) is 位址. |
| stream | 串流 | Network media and its recording (「錄製串流...」). A track inside a file is 軌: 音軌, 影片軌, 字幕軌 ("Cycle Audio Stream" 「切換音軌」, "Language/Stream Selection" 「語言/音軌選擇」). |
| device / capture device | 裝置 / 擷取裝置 | Never 設備. A webcam is 網路攝影機, a capture card 擷取卡. |
| screen capture | 螢幕擷取 | The desktop as a source. 擷取 is Taiwan's capture (not 捕獲, 採集). |
| disc | 光碟 | Its drive is 光碟機; never 光盤. |
| title | 標題 | A DVD's title and an entry's title alike ("Title Menu" 「標題選單」). |
| angle | 視角 | Not 角度, which is rotation's (「旋轉 %1 度」). |
| disc menu | 光碟選單 | 選單 is Taiwan's menu (never 菜單); the DVD's root menu 根選單. |
| drag and drop | 拖放 | Dragging alone is 拖曳. |
| recent files | 最近開啟的檔案 | "recent items" 最近項目. |
| favorites | 我的最愛 | |
| file association / file type | 檔案關聯 / 檔案類型 | The page "Association" 「檔案關聯」. |
| playlist | 播放清單 | 清單 is Taiwan's list (never 列表). |
| album | 專輯 | RVP's (PotPlayer's) named list, one tab of the playlist. "Album art" is a music album's 專輯封面. The album named "Default" is data and stays. |
| entry | 項目 | Also "item". |
| sort | 排序 | "Sort by" 「排序方式」; ascending 遞增, descending 遞減 (「遞增（A 到 Z）」). |
| add / remove / delete | 新增 / 移除 / 刪除 | "Add X" is 新增 X (新增檔案, 新增資料夾, 新增書籤); "add ... to Y" is 加入 Y (加入播放佇列, 加入我的最愛). 移除 takes an entry off a list; 刪除 deletes the file itself, or a bookmark or album. |
| recycle bin / trash | 資源回收筒 / 垃圾桶 | Windows' name / the Linux desktops'. |
| import / export | 匯入 / 匯出 | Never 導入 / 導出. |
| window / main window | 視窗 / 主視窗 | Never 窗口. |
| fullscreen / windowed | 全螢幕 / 視窗模式 | 螢幕 is Taiwan's screen (never 屏幕). |
| maximize / minimize | 最大化 / 最小化 | Back to normal is 還原. |
| always on top / stay on top | 一律最上層顯示 / 最上層顯示 | 最上層顯示 is the state and the checkbox; with "Always" 一律, as everywhere. |
| opacity | 不透明度 | Transparency is 透明度. |
| skin | 外觀 | PotPlayer's .dsf files and the others: "Skins" 「外觀」, "skin modes" 「外觀模式」, "Skin defined" 「由外觀定義」, the video and audio skins 影片外觀 / 音訊外觀. Not 面板 (a panel; RVP's control panel is 控制面板) and not 皮膚 (the mainland's). |
| color theme | 色彩主題 | A colour as a value is 顏色 (「字幕顏色」). |
| logo | 標誌 | The picture shown without a video; never 商標, which is a trademark. |
| title bar | 標題列 | |
| control panel | 控制面板 | RVP's F7 window; Windows' own Control Panel (控制台) is another thing. |
| OSD / on-screen display | OSD / 螢幕顯示 | OSD stays as an abbreviation: the page "OSD Messages" 「OSD 訊息」. |
| on-screen controller | 螢幕控制介面 | mpv's OSC; the Skins page's "Controller" is 「控制介面」, the menu's "On Screen Control" 「螢幕控制」. |
| tooltip | 工具提示 | |
| context menu | 右鍵選單 | |
| tray icon / notification area | 系統匣圖示 / 通知區域 | "Taskbar/Tray icon" 「工作列/系統匣圖示」. |
| taskbar | 工作列 | Where a text means a Linux desktop's dock, 「Dock」. |
| notification | 通知 | |
| dialog | 對話方塊 | "modeless dialog boxes" 「非強制回應對話方塊」. |
| settings / preferences | 設定 / 偏好設定 | Never 設置. |
| page | 頁面 | A page named in a text: `「播放」頁面`. |
| preset / configuration | 設定檔 / 設定檔 | Both English words are RVP's saved sets of all settings: the page "Configuration" 「設定檔」, "Configuration management" 「設定檔管理」, "Manage Presets..." 「管理設定檔...」, "*Default preset" 「*預設設定檔」. An equalizer's, frame size's or engine's preset is 預設集. Never 預設 alone, which is default. |
| reset / default | 重設 / 預設 | 「重設」, 「全部重設」; "Restore to Default" 「還原為預設值」; "Default" as a choice 預設, as a value 預設值. Never 默認. |
| apply | 套用 | Never 應用 (an app is 應用程式). "Applied." 「已套用。」 |
| key / hotkey / shortcut | 按鍵 / 快速鍵 / 快速鍵 | Hotkey and shortcut are one thing; never 快捷鍵. Key names stay as RVP shows them. |
| mouse / wheel / button | 滑鼠 / 滾輪 / 按鍵 | A mouse's button is 按鍵 (左鍵, 右鍵, 中鍵); a button on the screen or the skin is 按鈕. The pointer is 指標, the cursor 游標. |
| gesture | 手勢 | |
| touch | 觸控 | |
| controller / gamepad | 控制器 / 遊戲手把 | The Xbox controller; 手把 is Taiwan's word (not 手柄). mpv's on-screen controller is 控制介面. |
| video / picture / image | 影片 / 畫面 / 圖片 | 影片 for every video: the Video menu and page, 影片檔案, 影片軌, 影片轉譯器. Never 視頻; not 視訊 either (Taiwan's word for video calls and signals), to keep one term. 畫面 is the picture on the screen (also a 3D video's left and right images), 圖片 a still image or picture file. |
| frame | 影格 | One picture in time ("Next frame" 「下一個影格」, keyframe 關鍵影格). The picture's area that Extend/Crop and Resize change is 畫面 ("video frame" 「影片畫面」). Not 幀 or 畫格, to keep one term. |
| frame rate / fps | 影格速率 / fps | "Frames a second" 「每秒影格數」. |
| aspect ratio | 長寬比 | Also the menus' "AR"; DAR 顯示長寬比. Not 外觀比例 (外觀 is the skin). |
| crop | 裁切 | Extending is 延伸 ("Extend/Crop" 「延伸/裁切」). |
| zoom / pan | 縮放 / 平移 | "Zoom In" / "Zoom Out" 「放大」 / 「縮小」. |
| rotate / flip / mirror | 旋轉 / 翻轉 / 鏡像 | "Flip Horizontal" 「水平翻轉」, "Flip Vertical" 「垂直翻轉」. |
| deinterlace | 去交錯 | Interlaced 交錯式; a field 圖場 ("Top field first" 「上圖場優先」). |
| brightness / contrast / saturation / hue / gamma | 亮度 / 對比 / 飽和度 / 色調 / Gamma | 對比 without 度, as Taiwan's displays write it; Gamma stays Latin. |
| sharpen / denoise | 銳利化 / 降噪 | Soften 柔化, deblock 去區塊, deband 去色帶. |
| scaler / upscaling / resize | 縮放演算法 / 放大 / 調整大小 | "Upscaler" 「放大演算法」, "Downscaler" 「縮小演算法」, "Chroma scaler" 「色度縮放演算法」; 演算法, not 算法. |
| pixel shader | 像素著色器 | 著, never 着. Pre-resize / post-resize 縮放前 / 縮放後. |
| renderer | 轉譯器 | 轉譯 is the word for rendering in Taiwan's Windows (not 渲染器): the video renderer 影片轉譯器, the audio renderer 音訊轉譯器. |
| hardware decoding / decoder | 硬體解碼 / 解碼器 | 硬體 / 軟體, never 硬件 / 軟件. |
| HDR | HDR | Also SDR. |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360° 影片 / 互補色立體 / 左右並排 / 上下並排 | The anaglyph colours as in the Style section; SBS and TAB stay beside the Chinese. |
| bars / letterbox / margin | 黑邊 / 上下黑邊 / 邊界 | The black areas; the skin's control bars are 控制列. 邊界 as Taiwan's word processors name margins ("Bottom Margin" 「底部邊界」, "screen margins" 「螢幕邊界」). |
| frame capture / screenshot | 畫面擷取 / 擷圖 | "Save Current Screen Frame" 「儲存目前螢幕畫面」; the page "Video Capture" 「影像擷取」. |
| motion smoothing | 動態平滑 | |
| frame interpolation | 影格插補 | |
| engine | 引擎 | |
| calibration | 校準 | "Calibrate…" 「校準…」. 校正 is "correct" (「校正長寬比」), kept apart. |
| quality preset (light, medium, high) | 品質預設集（輕量、中等、高） | The labels "Fast", "Medium", "Quality" are 快速 / 中等 / 高品質. 品質 is Taiwan's quality (never 質量, which is mass). |
| GPU / graphics card | GPU / 顯示卡 | Never 顯卡. |
| processor / CPU | 處理器 / CPU | |
| power budget | 功耗預算 | |
| dropped frames | 掉格 | Their count is 掉格數; late frames 延遲的影格. |
| refresh rate | 更新率 | 「顯示器更新率」; never 刷新率. |
| audio / sound | 音訊 / 聲音 | 音訊 for the Audio menu and page, audio files and tracks (音訊檔案, 音軌); 聲音 in plain prose ("without sound" 「無聲」). Never 音頻. Audio effects are 音效. |
| volume / mute | 音量 / 靜音 | |
| master volume | 主音量 | The system's own; "wave volume" 波形音量. |
| equalizer | 等化器 | Never 均衡器. |
| normalizer | 音量正規化 | 正規化 is Taiwan's normalize (not 標準化 or 歸一化). |
| reverb | 殘響 | Freeverb stays. |
| noise reduction | 降噪 | The same word as the picture's denoise. |
| channel / speaker | 聲道 / 喇叭 | Never 揚聲器 in labels. LFE stays. |
| downmix | 降混 | |
| surround / stereo | 環繞聲 / 立體聲 | Virtual surround 虛擬環繞聲, mono 單聲道. |
| output device | 輸出裝置 | |
| sample rate / bitrate | 取樣率 / 位元速率 | Never 採樣率 / 比特率 / 碼率. |
| audio delay / audio sync | 音訊延遲 / 音訊同步 | |
| pitch / time stretching | 音高 / 時間伸縮 | Pitch shifting is 音高調整. |
| audio recording | 錄音 | "Record Audio..." 「錄音...」; recording video or a stream is 錄製. |
| second audio device | 第二音訊裝置 | |
| pass-through | 直通 | Sending the encoded sound on: 「AC3 重新編碼後直通」. |
| subtitles / subtitle track | 字幕 / 字幕軌 | |
| external / embedded subtitles | 外掛 / 內嵌字幕 | External subtitles 外掛字幕, external audio 外掛音訊; 內 with 入, not 内. |
| font / outline / shadow | 字型 / 外框 / 陰影 | Never 字體 for a font. Bold 粗體, italic 斜體. |
| subtitle position / subtitle delay | 字幕位置 / 字幕延遲 | Subtitle sync is 字幕同步. |
| encoding | 編碼 | A text file's 文字編碼; a character is 字元, never 字符. |
| subtitle browser / subtitle editor | 字幕瀏覽器 / 字幕編輯器 | |
| subtitles from audio / speech recognition | 從音訊產生字幕 / 語音辨識 | 辨識, not 識別. "Real time" is 即時. |
| model | 模型 | |
| language | 語言 | Language names as the Style section says. |
| remote control | 遙控 | |
| token | 權杖 | Never 令牌. |
| port / server | 連接埠 / 伺服器 | Never 端口 / 服務器. |
| log | 記錄檔 | The page and the file; to log is 記錄. The levels Critical, Warning, Info, Debug are 嚴重, 警告, 資訊, 偵錯. |
| crash | 當機 | RVP ending by an error ("crash logs" 「當機記錄檔」); never 崩潰. |
| download / install | 下載 / 安裝 | |
| installer | 安裝程式 | 程式 is Taiwan's program (never 程序, a procedure). |
| accessibility | 無障礙 | |
| text to speech (TTS) / screen reader | 文字轉語音（TTS） / 螢幕報讀軟體 | The page "TTS" stays 「TTS」. |
| screen saver | 螢幕保護程式 | |
| shut down / sleep / hibernate | 關機 / 睡眠 / 休眠 | |
| timer | 計時器 | "Shutdown Timer" 「定時關機」. |
| visualization / spectrum | 視覺效果 / 頻譜 | |
| status bar / info | 狀態列 / 資訊 | 資訊 is Taiwan's information (never 信息); a message is 訊息. |
| codec / format | 編解碼器 / 格式 | |
| bit depth / 10-bit | 位元深度 / 10 位元 | 位元 for bit, 位元組 for byte: "%1-bit" 「%1 位元」. |
| channel layout (5.1, 7.1) | 聲道配置（5.1、7.1） | |
| Auto select (recommended) | 自動選擇（建議） | "(Recommended)" is 「（建議）」 everywhere; never 推薦. |
