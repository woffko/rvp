# Chinese, Simplified (zh_CN)

RVP's Simplified Chinese is the written standard of mainland China:
simplified characters only, and the vocabulary mainland users know from
Windows, GNOME, KDE and media players in zh_CN, not Taiwan's or Hong
Kong's (`文件` not `檔案`, `文件夹` not `資料夾`, `视频` not `影片`, `音频` not
`音訊`, `默认` not `預設`, `设置` not `設定`, `鼠标` not `滑鼠`, `字体` not
`字型`, `程序` not `程式`, `信息` not `資訊`, `网络` not `網路`, `屏幕` not
`螢幕`, `光盘` not `光碟`). The tone is the plain, neutral written register of
a desktop program's own settings: short, exact, no internet slang, no
padding with `进行`, `一下` or `的话`. Established Chinese terms are used
wherever they exist (`播放列表`, `字幕`, `反交错`, `变速不变调`); Latin names,
codec and format names stay as written. The Russian translation helps with
meaning, never with wording. The Traditional Chinese of Taiwan has its own
file (`zh_TW.md`); never mix the two.

## Style

- **Form of address:** no pronoun wherever the sentence works without one,
  which is nearly always. Where a help text needs one (`that you download`,
  `you accept`), it is `您`, never `你`. Help texts describe with RVP or the
  feature as the subject, in the plain present (`RVP 测量各预设的帧率。`,
  `mpv 在完成的画面上绘制字幕。`); no tense particles unless the meaning needs
  them (`已`, `会`). Instructions
  are plain imperatives (`用鼠标拖动视频即可环顾四周。`); `请` only in a prompt
  that asks for an action at this moment (`请为“%1”（%2）按一个键。按 Esc 取消。`),
  never in labels or descriptions. Negation: `不` for what RVP does not do,
  `无法` for what cannot be done (`无法在此运行`), `没有`/`无` for absence.
- **Labels.** Commands (menu items, buttons) are verb + object, as short as
  the meaning allows: `打开文件夹...`, `添加书签`, `另存为…`, `播放下一项`. Checkboxes
  are verb phrases with the condition first, as Chinese orders it:
  `记住视频播放位置`, `播放列表结束时关闭媒体`, `不添加重复的文件`; never `是否` + verb.
  Menu names, page titles and section headings are noun phrases: `播放`,
  `字幕`, `区段循环`, `运动平滑校准`. Choices in a list are short words:
  `从不`, `始终`, `播放时`, `全屏播放时`, `自动`, `无`, `自定义`, `左上`. On / Off as a
  choice is `开` / `关`; Enabled / Disabled are `已启用` / `已禁用`; status texts
  are `已` + verb with a full stop: `已应用。`, `已导入。`, `已导出。`, `已取消。`,
  `未更改。`. Questions in confirmations are `要` + verb + `吗？`:
  `要将 %1 个文件移到回收站吗？`. Chinese has no capitals: the source's Title
  Case means nothing; Latin words keep their own case.
- **Usual verbs:** Enable `启用`, Disable `禁用`, Use `使用`, Show `显示`, Hide
  `隐藏`, Remember `记住`, Open `打开`, Load `加载`, Save `保存`, Save As `另存为`,
  Record `录制`, Browse `浏览`, Choose / Select `选择`, Select All `全选`, Deselect
  All `取消全选`, Invert Selection `反选`, Find `查找`, Search `搜索`, Copy `复制`,
  Paste `粘贴`, Rename `重命名`, Duplicate `创建副本`, Activate (a preset)
  `激活`, Undo `撤销`, Redo `重做`, Cancel `取消`, Close `关闭`, Exit `退出`,
  Restore (a window) `还原`, Restore defaults `恢复默认`, Reload `重新加载`,
  Rescan `重新扫描`, Prompt `询问`, Prevent `阻止`, Move Up / Down (in a list)
  `上移` / `下移`, Set up `设置`, Cycle X `切换X` (`切换宽高比`), Start/Stop X
  `开始/停止X`, Show/Hide X `显示/隐藏X`, Auto X `自动X` (`自动隐藏`, `自动加载`).
  Increase / Decrease take the verb the object needs, and the same object
  always the same verb: `增大` / `减小` for sizes, volume, spacing and
  margins (`增大音量`, `减小字号`); `提高` / `降低` for brightness, contrast,
  saturation, hue and speed (`提高亮度 %1%`, `降低速度`); `增加` / `减少` for
  sync values and counts (`字幕同步增加 %1 毫秒`). Process priorities are
  Windows' words: `高`, `高于正常`, `正常`, `低于正常`, and Idle `低`. A step (an
  increment) is `步长` (`音量步长`); stepping down a smoothing engine is `降级`.
- **Punctuation:** full-width marks in Chinese text: `，` `。` `：` `；` `？`
  `！` `（）` `“”`, and the enumeration comma `、` between listed items
  (`SRT、ASS、WebVTT`; slashes stay where the source has them:
  `前/左/右/后`). A sentence that ends in a Latin word or a file name still
  ends with `。` (`文件移到旁边的 rvp.old.log。`). Half-width ASCII marks stay only where
  they are not Chinese punctuation: times and ratios (`h:mm:ss.s`, `16:9`,
  `%1:%2`), ranges (`0-255`, `16-235`), `/` between alternatives (no spaces:
  `播放/暂停`, `专辑/收藏夹`), `+` in keys and names, `->`, `>` in a settings path
  (`视频 > 底部边距`), and parentheses that belong to a name (`RIFE (TensorRT)`)
  or to a file dialog's filter: Qt reads the pattern from ASCII parentheses
  at the end, so filters keep them with a space before: `所有文件 (*)`,
  `媒体文件 (%1)`, `PotPlayer 皮肤 (*.dsf)`. Everywhere else parentheses are
  full-width, also around keys and Latin words: `去块（Ctrl+H）`, `（推荐）`,
  `关（软件解码）`. No dash `——` in labels: a colon or a comma does its work.
  An example list that ends in `...` ends in `等` instead:
  `地址（http、https、rtmp 等）：`. Never the full-width space U+3000, never
  full-width Latin letters or digits (`ＡＢ１２`).
- **Ellipsis:** keep exactly what the source has, three ASCII periods `...`
  or the single character `…` (U+2026), attached to the last character with
  no space: `打开文件...`, `首选项...`, `另存为…`, `正在测量 %1…`. The Chinese prose
  ellipsis `……` (two U+2026, six dots) is not used: here the mark is the
  interface's sign that a dialog follows or that work goes on, which
  zh_CN desktops write as in English, and `……` would also double the
  label's width for nothing. Where the dots stand for a value the user
  gives (`Shutdown at ...`, `Shutdown in ...`), they keep the source's spaces
  and move to where the value goes in Chinese: `在 ... 关机`, `... 后关机（未指定时间时）`.
- **Spacing:** one rule everywhere: a half-width space between a Chinese
  character and a Latin letter, a digit or a placeholder, on both sides
  (`打开 URL...`, `RVP 的日志`, `%1 个文件`, `最高 %3p`, `用 yt-dlp 打开`, `第 %1 个徽标`);
  no space between any of them and full-width punctuation (`状态：%1`,
  `（Ctrl+H）`, `“%1”`, `RIFE (TensorRT)：%1`); no space inside a Latin or
  number group (`Ctrl+Shift+Enter`, `1080p`, `4K`); spaces between
  placeholders as in the source (`%1 %2`). Chinese words never take spaces
  between them.
- **Quotation marks:** `“”` (U+201C, U+201D) for every quote in the source,
  `‘’` inside them; never corner brackets `「」` (Taiwan, Hong Kong) or book
  marks `《》`. No space inside or around them. A help text that names a
  control, page or menu quotes its Chinese label: `“播放”页面中的“新打开视频时调整窗口”`,
  `“重新生成”会创建新令牌`. The source's straight quotes (`Add "Add to"`) become
  `“”` too: `添加“添加到”`. English `'s` becomes `的`, or nothing where the
  noun reads well without it (`mpv 默认值`, `RVP 状态文件夹`).
- **"&"** becomes `和` (`低音和高音增强`, `添加和编辑`, `路径和名称`).
- **Units:** seconds `秒`, minutes `分钟`, hours `小时`, milliseconds `毫秒`, in
  labels too (`%1 秒`, `%1 分钟`, `后退 %1 秒`, `字幕同步减少 %1 毫秒`; the `ms`
  label beside a field is `毫秒`; `Sec.` and `Min` are the same words). Latin
  symbols stay, with a space after the number: `-3 dB`, `%1 Hz`, `48 kHz`,
  `%1 W`, `%2 GB`, `16 MB`, `px`. Attached, as Chinese writes them: `%1%` and
  `%1 %` both become `%1%` (`提高亮度 %1%`, `%1 W / %3 W（%4%）`), `%1°`,
  `1080p`, `%3p`, `4K`, `%1D`, multipliers `%1x` (`%1x 后退`, not
  `%1x 倍速后退`), sizes `%1x%2`. Frames per second: `fps` in labels and values,
  `每秒 %1 帧` or `帧/秒` in sentences. `%1-bit` is `%1 位` (`32 位（浮点）`).
  Time fields keep the formats the program reads (`h:mm:ss.s`, `hh:mm`).
- **Numbers:** half-width Arabic digits, a decimal point `.`, no thousands
  separators. Numbers the source writes in digits stay digits; numbers it
  spells out become Chinese words (`two letters` → `两个字母`, `twice` → `两倍`,
  `a fifth` → `五分之一`, `one to three minutes` → `一到三分钟`); `两`, not `二`,
  before a measure word. Ordinals are `第` + number: `第 %1 设备` (`%1nd
  Device`), `{与第 %1 个徽标相同}` (`%1st logo`), `第一个徽标`. The number sign
  stays: `样式 #%1（由皮肤定义）`. Versions, addresses and channel layouts keep
  their periods (`TensorRT 10.15.1`, `127.0.0.1`, `5.1`, `7.1`).
- **Measure words and placeholders.** Chinese has no plural: the source's
  `(s)` disappears, and separate singular and plural strings
  (`Reserve %1 line` / `lines`) get the same text. A count placeholder takes
  its measure word: `%1 个文件`, `%1 个项目`, `%1 项`, `%1 帧` (帧 is its own
  measure word), `%1 行`, `%1 列`, `%1 张图片`, `%1 条字幕`, `%1 次`, `%1 个声道`;
  units need none (`%1 秒`). A placeholder that stands for a name or a text
  takes none; name the noun where the sentence needs it (`%1 引擎`, `文件 %1`).
  Word order follows Chinese, and placeholders move with it: `%1 已被“%2”使用。改为分配给“%3”？`,
  `已移除 %1`, `%2 秒后` (`in %2 seconds`).
- **Recommended:** `（推荐）`, always this one tag, full-width parentheses,
  whatever the source's capital (`Recommended`, `recommended`, `Recom'd`).
  `(Default)` is `（默认）`.
- **Keys** keep RVP's English names in every text, as its menus show them:
  `Ctrl`, `Shift`, `Alt`, `Enter`, `Space`, `Esc`, `Tab`, `PgUp`,
  `Left / Right`, `Scroll Lock`, `X1`, `X2`. Prose around them is Chinese:
  `方向键`, `滚轮`, `按 Esc 取消`, `Back 键`. A mouse's buttons are `左键`, `右键`,
  `中键`; X1 and X2 are `侧键` (`X1 是后退键，X2 是前进键。`).
- **Names and Dolby.** Product names stay Latin (`Dolby Pro Logic II`,
  `Dolby Surround`, `RVP Default`); Dolby as a word in a description is
  `杜比` (`虚拟杜比解码器`, `耳机和杜比解码器设置`). Words in angle brackets that
  stand for a value are translated, the rest of a path stays
  (`ggml-<模型>.bin`, `mpvSockets/<进程 ID>`); likewise `<默认徽标>`,
  `[由皮肤定义]`, `{与第一个徽标相同}` keep their brackets.
- **Example values** in empty fields: language codes become Chinese users'
  own, at the source's length (`rus,eng` becomes `chi,eng`, `rus` becomes
  `chi`, `en,ru` becomes `zh,en`, `en` stays); chapter names
  (`Opening;Ending;3`, matched against the files' own chapter titles),
  paths and folder names (`Music/RVP`, `V:`, `/mnt/media`) and
  `bestvideo+bestaudio/best` stay.
- **Words the program inserts** into other texts are written to fit any
  sentence: `checking` `正在校验`, `unpacking` `正在解压`, `downloading` `正在下载`,
  `never` `从不`, `none` `无`, `not set` `未设置`, `ask every time` `每次询问`.
- **Fragments** that the program joins keep their place in the sentence
  with Chinese marks: `. Lightened: %1.` is `。已减轻：%1。`. Line breaks stay
  exactly where the source has them (`%1\n%2 秒后`).
- **Language names** (the whisper list, lowercase in the source) are the
  usual Chinese names: `英语`, `法语`, `日语`, `俄语`, `chinese` `中文`, `cantonese`
  `粤语`. This language's own name is `简体中文`.
- **Length.** A Chinese character counts as two Latin letters. Chinese
  labels are usually shorter than the English: never pad them to fill the
  room. Keep the standard short words (`设置`, not `设置选项`; `时长`, not
  `持续时间`), drop `的` where the phrase reads well without it, and drop
  `进行`, `一下`, `当前的` unless the meaning needs them; but never cut grammar
  into telegraphic Chinese that loses the meaning. Help texts may grow.
- **e.g.** is `例如`, **i.e.** is `即`.

## Glossary

| English | Chinese (Simplified) | Note |
|---------|----------------------|------|
| play / pause / stop | 播放 / 暂停 / 停止 | The toggle is `播放/暂停`. Stop keeps the file loaded; it is not close playback. |
| playback | 播放 | The Playback menu and page: `播放`; `播放设置...`. |
| close playback | 关闭媒体 | Stop and unload what plays (file, disc or stream): `关闭媒体`, `Back 键关闭媒体`. Never `关闭播放器` (that is Exit, `退出`). |
| previous / next | 上一 / 下一 | Always with what follows: `上一个` / `下一个` alone, `上一章`, `下一帧`, `上一项`, `下一个关键帧`. |
| seek | 定位 | Moving the position, as a verb and a noun (`用鼠标定位时暂停`, `下次定位时`). Not `搜索` (search), not `跳转` (jump). |
| jump | 跳转 | `跳转到指定时间/帧...`; the arrow keys' time jumps: `时间跳转`. |
| frame step | 逐帧 | `逐帧播放`; single steps are `下一帧` / `上一帧`. |
| speed | 速度 | `播放速度`; `提高速度` / `降低速度`; values keep `1.0x`. |
| repeat | 循环 | Playback repeat is `循环`, as Chinese players say: `循环播放列表`, `循环当前文件`, `循环次数（0：无限）`. Repeating a frame (smoothing at a cut) is `重复`. |
| A-B repeat / section repeat | A-B 循环 / 区段循环 | A marked part is a `区段` (`循环当前标记区段`); its points are `A 点` / `B 点`. |
| shuffle | 随机播放 | `随机` alone in a list of orders. |
| queue | 队列 | `播放队列`, `添加到播放队列`. |
| resume / remember position | 续播 / 记住播放位置 | The Resume heading is `续播`; not `继续` (unpause) and not `恢复` (defaults). |
| position | 位置 | Also a logo's or subtitle's place on the screen. |
| duration | 时长 | Not `持续时间`. |
| elapsed / remaining time | 已播放 / 剩余时间 | In full `已播放时间` / `剩余时间`; as choices `已播放` / `剩余`. |
| chapter | 章节 | `上一章` / `下一章` in menu commands. |
| bookmark | 书签 | |
| skip (intro, ending) | 跳过（片头、片尾） | `跳过片头`, `跳过片尾`, `跳过章节`; the intro and ending are `片头` / `片尾`. |
| seek bar | 进度条 | Also for the source's "nav bar" and "navigation bar". |
| thumbnail | 缩略图 | |
| preview | 预览 | |
| scene browser | 场景浏览器 | Browse Scenes is `浏览场景`. |
| file | 文件 | |
| folder | 文件夹 | Not `目录` (kept for paths in technical texts only, never in labels). |
| URL / address | URL / 地址 | `打开 URL...`; a remote's or stream's `地址`. |
| stream | 流 | A track inside a file is an `音频流` / `视频流`; network media is a `网络流`; Record Stream is `录制流`; streaming sites are `视频网站`. |
| device / capture device | 设备 / 采集设备 | A capture card is `采集卡`; a webcam `摄像头`. |
| screen capture | 屏幕采集 | The desktop as a video source. Never `截屏` or `截图` (a screenshot) or `录屏` (a recording). |
| disc | 光盘 | |
| title | 标题 | A DVD title, an entry's title, a bookmark's title. |
| angle | 视角 | A DVD's angles and a 360° video's camera views (`视角菜单`). |
| disc menu | 光盘菜单 | |
| drag and drop | 拖放 | |
| recent files | 最近文件 | Recent items: `最近项目`. |
| favorites | 收藏夹 | |
| file association / file type | 文件关联 / 文件类型 | The Association page: `文件关联`. |
| playlist | 播放列表 | Help texts may say `列表` once they have named it. |
| album | 专辑 | RVP's term for a playlist tab, a named list of any media, not only music. Never `相册` (a photo album). |
| entry | 项目 | Also for the source's "item" of the playlist; `项` in compounds (`下一项`, `所选项目`, `%1 项`). |
| sort | 排序 | |
| add / remove / delete | 添加 / 移除 / 删除 | Trap: `移除` takes an entry out of a list (playlist, folders, keys) and leaves the file; `删除` destroys the file, bookmark, album or preset itself. |
| recycle bin / trash | 回收站 / 回收站 | Windows and the Linux desktops both say `回收站`; sending a file there is `移到回收站`. |
| import / export | 导入 / 导出 | |
| window / main window | 窗口 / 主窗口 | |
| fullscreen / windowed | 全屏 / 窗口模式 | |
| maximize / minimize | 最大化 / 最小化 | Windows' words; Restore is `还原`. |
| always on top / stay on top | 始终置顶 / 置顶 | Cycle Stay on Top: `切换置顶`. |
| opacity | 不透明度 | Trap: "transparency" is `透明度`, the opposite scale. |
| skin | 皮肤 | Not `外观` or `主题`. Video skin: `视频皮肤`; the audio skin: `音频皮肤`. |
| color theme | 配色方案 | A skin's color variant; `HSB 配色方案`, `随机配色方案`. |
| logo | 徽标 | The picture shown without a video (`<默认徽标>`, `徽标文件夹`). |
| title bar | 标题栏 | |
| control panel | 控制面板 | RVP's F7 window. Where Windows' own Control Panel is meant, say `Windows 控制面板`. |
| OSD / on-screen display | OSD / 屏幕显示 | OSD Messages: `OSD 消息`; mpv's OSD: `mpv 的 OSD`. |
| on-screen controller | 屏幕控制器 | mpv's OSC, uosc and the like, drawn over the video; not a gamepad (`手柄`). The On Screen Control page is `屏幕控制`. |
| tooltip | 工具提示 | Also the seek bar's tooltip. |
| context menu | 右键菜单 | The word users say; Windows' `上下文菜单` is not used. |
| tray icon / notification area | 托盘图标 / 通知区域 | "Tray" alone is `托盘`. |
| taskbar | 任务栏 | On Linux the dock is `Dock 栏`; Windows' jump list is `跳转列表`. |
| notification | 通知 | |
| dialog | 对话框 | |
| settings / preferences | 设置 / 首选项 | The settings window is `设置`; the Preferences... menu item is `首选项...`; a page's items are `选项`. |
| page | 页面 | Reset Page: `重置页面`. |
| preset / configuration | 预设 / 配置 | Also equalizer and frame size presets (`预设 %1`); the Configuration page: `配置`. |
| reset / default | 重置 / 默认 | The Default button is `默认`; RVP's defaults are `RVP 的默认值`; Restore to Default is `恢复默认`. |
| apply | 应用 | |
| key / hotkey / shortcut | 按键 / 热键 / 快捷键 | The Key… button is `按键…`. |
| mouse / wheel / button | 鼠标 / 滚轮 / 按钮 | A mouse's buttons by name are `左键`, `右键`, `中键`, `侧键`; an on-screen button is `按钮` too. |
| gesture | 手势 | `鼠标手势`. |
| touch | 触控 | The Touch page is `触控`; a touchscreen is `触摸屏`. |
| controller / gamepad | 手柄 / 游戏手柄 | The Xbox controller is `Xbox 手柄`; mpv's on-screen controller is `屏幕控制器`. |
| video / picture / image | 视频 / 画面 / 图像 | The video's picture is `画面`; a still image is `图像`, an image file or photo `图片` (`以 PNG 格式保存图片`, slideshow and cover pictures). |
| frame | 帧 | One picture in time. The video's rectangle ("video frame", "frame size") is `画面` (`画面尺寸`); a window's frame is `边框`. |
| frame rate / fps | 帧率 / fps | A rate alone is `速率` or `帧率` (`显示器的刷新率`, `视频帧率的两倍`). |
| aspect ratio | 宽高比 | Also for the source's "AR". |
| crop | 裁剪 | Extend is `扩展` (`扩展/裁剪`). |
| zoom / pan | 缩放 / 平移 | Zoom In / Out: `放大` / `缩小`. |
| rotate / flip / mirror | 旋转 / 翻转 / 镜像 | Flips: `水平翻转` / `垂直翻转`. Trap: Stretch is `拉伸`. |
| deinterlace | 反交错 | Also the noun (`反交错模式`). |
| brightness / contrast / saturation / hue / gamma | 亮度 / 对比度 / 饱和度 / 色调 / 伽马 | `色调` also for the HSB theme's hue slider. |
| sharpen / denoise | 锐化 / 降噪 | `3D 降噪`, `时域降噪器`. |
| scaler / upscaling / resize | 缩放算法 / 放大 / 调整大小 | Upscaler / Downscaler / Chroma scaler: `放大算法` / `缩小算法` / `色度缩放算法`; the Resize page is `调整大小`. |
| pixel shader | 像素着色器 | `着色器` alone in short labels once the context is set. |
| renderer | 渲染器 | `视频渲染器`, `音频渲染器`. |
| hardware decoding / decoder | 硬件解码 / 解码器 | Software decoding is `软件解码`; not the slang `硬解` / `软解`. |
| HDR | HDR | |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360° 视频 / 互补色 3D / 左右并排 / 上下排列 | Colours of an anaglyph: `红/青`, `绿/品红`, `黄/蓝`, `红/蓝`, `红/绿`; `灰度`, `半彩色`, `彩色`. SBS / TAB keep their letters: `SBS（左右并排）`. |
| bars / letterbox / margin | 黑边 / 上下黑边 / 边距 | Black areas only; the skin's control bars are `控制栏`. |
| frame capture / screenshot | 画面截取 / 截图 | The Video Capture page and menu (saved frames and recordings) is `截图与录制`. |
| motion smoothing | 运动平滑 | The feature and its page; `平滑` alone once the context is set. |
| frame interpolation | 补帧 | The word SVP users know; "interpolate across a cut" is `跨场景补帧`. |
| engine | 引擎 | |
| calibration | 校准 | The verb is `校准`; Recalibrate is `重新校准`. |
| quality preset (light, medium, high) | 质量预设（轻度、中等、高） | The presets shown are `快速`, `中等`, `高质量`; lighter is `更轻` (`更轻的预设`). |
| GPU / graphics card | GPU / 显卡 | |
| processor / CPU | 处理器 / CPU | |
| power budget | 功耗预算 | A card's power in watts is its `功耗`, not `电源` or `能量`. |
| dropped frames | 丢帧 | Late frames: `延迟帧`. |
| refresh rate | 刷新率 | |
| audio / sound | 音频 / 声音 | |
| volume / mute | 音量 / 静音 | |
| master volume | 主音量 | Windows' Wave volume is `波形音量`. |
| equalizer | 均衡器 | |
| normalizer | 音量标准化 | The feature, as Chinese users name it; never `均衡` (the equalizer's word). |
| reverb | 混响 | |
| noise reduction | 降噪 | The sound's, like the video's denoise. |
| channel / speaker | 声道 / 扬声器 | LFE stays `LFE`; centre `中置`, front `前置`, side `侧置`, back `后置`. |
| downmix | 缩混 | In help texts, say what it does: `将 5.1 缩混为立体声`. |
| surround / stereo | 环绕声 / 立体声 | |
| output device | 输出设备 | |
| sample rate / bitrate | 采样率 / 比特率 | |
| audio delay / audio sync | 音频延迟 / 音频同步 | The verb is `同步`. |
| pitch / time stretching | 音调 / 变速不变调 | Pitch shifting is `变调`. Trap: the video's Stretch is `拉伸`. |
| audio recording | 录音 | Record Audio... is `录音...`. |
| second audio device | 第二音频设备 | |
| pass-through | 直通 | Sending the encoded sound on unchanged (`AC3 重新编码后直通`). |
| subtitles / subtitle track | 字幕 / 字幕轨 | One line is `一条字幕`. |
| external / embedded subtitles | 外部 / 内置字幕 | In full `外部字幕`, also `外部音频`. Trap: not `内嵌`, which mainland users read as subtitles burned into the picture. |
| font / outline / shadow | 字体 / 描边 / 阴影 | Font size is `字号`. |
| subtitle position / subtitle delay | 字幕位置 / 字幕延迟 | |
| encoding | 编码 | A text file's `文本编码`. |
| subtitle browser / subtitle editor | 字幕浏览器 / 字幕编辑器 | |
| subtitles from audio / speech recognition | 从音频生成字幕 / 语音识别 | The page and the commands: `从音频生成字幕`, `从音频生成字幕（实时）`. |
| model | 模型 | Whisper's model names (`tiny`, `base`...) stay. |
| language | 语言 | |
| remote control | 远程控制 | A remote (the phone program or its layout) is a `遥控器` (`Unified Remote 遥控器`). |
| token | 令牌 | |
| port / server | 端口 / 服务器 | |
| log | 日志 | Levels: `严重`, `警告`, `信息`, `调试`. |
| crash | 崩溃 | |
| download / install | 下载 / 安装 | |
| installer | 安装程序 | |
| accessibility | 辅助功能 | |
| text to speech (TTS) / screen reader | 文本转语音（TTS） / 屏幕阅读器 | `TTS` alone in the page title. |
| screen saver | 屏幕保护程序 | Windows' word; `屏保` only where a short label has no room. |
| shut down / sleep / hibernate | 关机 / 睡眠 / 休眠 | Windows' words; Shutdown Timer is `定时关机`. |
| timer | 定时器 | |
| visualization / spectrum | 可视化 / 频谱 | Spectrogram `频谱图`, waveform `波形`. |
| status bar / info | 状态栏 / 信息 | Also the Info log level. |
| codec / format | 编解码器 / 格式 | |
| bit depth / 10-bit | 位深度 / 10 位 | |
| channel layout (5.1, 7.1) | 声道布局（5.1、7.1） | |
| Auto select (recommended) | 自动选择（推荐） | Auto select alone is `自动选择`. |
| filter | 滤镜 | Video and audio filters alike: `RVP 自己的滤镜`, `SVP 的滤镜`, `视频变换滤镜`, `滤镜控制`. |
| copy-back | 复制回内存 | Hardware decoding that copies the frames back for filters (`auto-copy（SVP 需要复制回内存）`). |
| step (amount per press) / step down | 步长 / 降级 | A setting's amount per press (`音量步长`, `同步步长`); smoothing taking a lighter preset or engine. |
| luma / chroma | 亮度 / 色度 | Luma shares `亮度` with brightness; the label's context tells them apart (`柔化：亮度`). |
| deblock / deband / soften | 去块 / 去色带 / 柔化 | |
| temporary denoise | 时域降噪 | PotPlayer's "Temporary" and "Temporal" denoise are one filter: `时域降噪`, `时域降噪器：A`. |
| pre-resize / post-resize | 缩放前 / 缩放后 | Pixel shaders before and after scaling: `缩放前像素着色器`. |
| consecutive images | 连续截图 | Their count is `张数`, the period `间隔`. |
| saved frames / storage folder | 截图 / 保存文件夹 | `截图文件夹`, `打开截图保存文件夹`, `设置录音保存文件夹...`. |
| Start (button or time) | 开始 | Both the command (the `开始` button) and a point in time (a subtitle's `开始`, its End `结束`, `开始和结束`). A-B's points are `起点` / `终点`. |
| Move (key, DVD titles) | 转移 | Moving a key to another action is `转移`; DVD Control's Move submenu (the disc's titles) is `转到`; moving a picture or subtitle is `移动` (`左移`, `上移`). |
| cycle | 切换 | `切换宽高比`, `切换字幕`. |
| swap | 交换 | `交换立体声声道`, `交换左右图像`. |
| voice removal / voice emphasis | 人声消除 / 人声增强 | |
| crossfeed | 交叉馈送 | |
| bass / treble | 低音 / 高音 | Equalizer presets: `低音增强`, `高音增强`, `低音和高音增强`. |
| EULA | 最终用户许可协议 | `CUDA 许可协议` where a short label has no room. |
| state folder | 状态文件夹 | RVP's state folder. |
| runtime (VapourSynth, CUDA) | 运行时 | `RVP 的 VapourSynth 运行时`, `CUDA 运行时`. |
| endpoint / pipe / socket / portal | 端点 / 管道 / 套接字 / 门户 | IPC terms; the portal is the desktop's (xdg-desktop-portal). |
| instance | 实例 | `多个实例`, `在新实例中播放`. |
| extension (file) | 扩展名 | A file's extension; Extend (the picture) is `扩展`. |
| shoulders / stick | 肩键 / 摇杆 | The gamepad's buttons and sticks. |
| tone mapping / field order | 色调映射 / 场序 | |
| anamorphic | 变形（anamorphic） | |
| seamless playback | 无缝播放 | PotPlayer's joining of files without a gap. |
| channel control | 频道控制 | The TV channels' menu (not sound channels, `声道`). |
| levels (video) | 色阶 | The picture's black and white points: `自动色阶`, `色阶调整（Ctrl+L）`, `色阶和偏移`. Sound levels stay `电平` (`混音器电平`, `湿声电平`). |
| broadcast effects | 直播效果 | PotPlayer's live-streaming tools (logo, pen, scrolling text). |
| movie subtitles | 视频的文本字幕 | "Movie" is `视频`, never `影片`. |

The rows after "Auto select (recommended)" are terms this translation added.
