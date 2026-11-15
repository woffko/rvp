# Korean (ko)

Standard Korean as written in South Korea (the 한글 맞춤법 spelling and
word spacing, loanwords spelled by the 외래어 표기법), in the plain
vocabulary of Korean desktop software: the general interface words that
Windows, GNOME and KDE use in Korean (설정, 창, 대화 상자, 작업 표시줄,
휴지통, 글꼴, 찾아보기), the established Sino-Korean words of video
players (재생, 일시 정지, 자막, 구간 반복, 화면 비율), and loanwords where
Korean users say them (스킨, 코덱, 필터, 렌더러, 셰이더, 이퀄라이저,
프레임, 썸네일). Help texts and messages are polite formal sentences in
the 합니다 style; labels are noun phrases, as Korean menus and settings
are. The user is never addressed by a pronoun. Everything is written for
RVP; nothing is taken from another program's Korean strings.

## Style

- **Speech level.** Whole sentences (help texts under a page's options,
  messages, notices, dialog texts) use the formal polite 합니다 style:
  "-ㅂ니다 / -습니다" ("RVP는 측정이 끝나면 각 파일을 알맞은 프리셋으로
  시작합니다."). Never 해요체 ("-해요", "-하세요"), never the plain "-다"
  style, never noun-ended telegraphic sentences ("-함", "-음") where the
  source writes a sentence. RVP and other programs are subjects without
  the honorific "-시-": "RVP가 다운로드합니다", not "다운로드하십니다".
- **Instructions and questions.** Where the source tells the user to do
  something, the imperative is "-십시오": "자막을 입력하고 Enter 키를
  누르십시오.", "다시 다운로드하십시오.". "Please" alone adds nothing; a
  real request is "-해 주십시오". Confirmation questions take
  "-하시겠습니까?": "파일 %1개를 휴지통으로 보내시겠습니까?", "“%1”의
  옵션을 RVP 기본값으로 되돌리시겠습니까?". "you" disappears: Korean
  drops the subject ("that you download into the models folder" is
  "모델 폴더에 직접 다운로드해 넣는"); where a subject is needed, it is
  "사용자".
- **Notices.** A source sentence with a final period becomes a 합니다
  sentence with a period: "Imported." "가져왔습니다.", "Cancelled."
  "취소되었습니다.", "Unchanged." "변경되지 않았습니다.", "Not measured
  yet." "아직 측정하지 않았습니다.". A fragment without a period (status
  words, notes in parentheses, list entries) takes the noun ending "-됨 /
  -음 / 안 됨" and no period: "%1 removed" "%1 제거됨", "(not downloaded)"
  "(다운로드 안 됨)", "(not installed)" "(설치 안 됨)", "%1 done" "%1
  완료". Progress texts are a verbal noun with "중": "Starting…" "시작
  중…", "Measuring %1…" "%1 측정 중…". Words RVP inserts into another
  string must read right there: "checking", "unpacking", "downloading"
  in "RIFE (TensorRT): %1, %2 of %3 GB" are "확인 중", "압축 해제 중",
  "다운로드 중".
- **Menu items and buttons** are commands written as noun phrases: the
  object first, then a Sino-Korean verbal noun without "하기" (추가,
  삭제, 저장, 표시, 사용, 선택, 설정, 적용) or a native verb in the "-기"
  noun form (열기, 닫기, 끝내기, 숨기기, 건너뛰기, 늘리기, 줄이기,
  높이기, 낮추기, 불러오기, 바꾸기). Never "추가하기", "저장하기",
  never a sentence ending ("-합니다", "-하세요"). "Open File(s)..." is
  "파일 열기...", "Add Bookmark" "북마크 추가", "Save playlist as..."
  "재생 목록을 다른 이름으로 저장...", "Next Chapter" "다음 챕터". The
  standard desktop words: OK 확인, Cancel 취소, Apply 적용, Close 닫기,
  Exit 끝내기, Open 열기, Save 저장, Save As 다른 이름으로 저장, Browse
  찾아보기, Choose/Select 선택, Rename 이름 바꾸기, Copy 복사, Paste
  붙여넣기, Cut 잘라내기, Insert 삽입, Find 찾기, Edit 편집, Undo 실행
  취소, Redo 다시 실행 (never 반복, which is repeat), Reset 초기화, Yes
  예, No 아니요, About RVP "RVP 정보", Help 도움말, Tools 도구, Misc 기타,
  General 일반, Advanced 고급, Customize 사용자 지정, Up / Down 위로 /
  아래로, Move Up "위로 이동".
- **Checkboxes** are noun phrases of the same shape, never sentences:
  "도구 설명 표시", "재생 위치 기억", "재생 중 화면 보호기 끄기",
  "건너뛰기 기능 사용". A condition comes first with "-(으)ㄹ 때", "-면"
  or "중": "재생 목록이 끝나면 재생 종료", "전체 화면에서 재생할 때".
  Enable / Disable / Use are "사용" / "사용 안 함" / "사용" ("Enable SVP
  support" "SVP 지원 사용", "Disable %1D Video Mode" "%1D 비디오 모드 사용
  안 함"); Turn on / off are "켜기" / "끄기".
- **Choices** in lists are short nouns, adverbs or "-음" forms: On "켬",
  Off "끔", Enabled "사용", Disabled "사용 안 함", None "없음", Never
  "안 함", Always "항상", Auto "자동", Normal "보통", Low / Medium / High
  "낮음" / "중간" / "높음", Fast / Slow "빠름" / "느림", "While playing"
  "재생 중", "Auto select (Recommended)" "자동 선택(권장)". Check the
  other choices of the same list and give them one shape.
- **Page titles, headings, window titles and field labels** are nouns
  or noun phrases ("재생", "자막", "화면 비율", "글꼴 크기"). A window
  titled by a verb keeps the noun form: "Adjust Audio Sync" "오디오 싱크
  조절", "Jump to Specified Time/Frame" "지정한 시간/프레임으로 이동". A
  label before a field ends with a colon only where the source has one.
  Korean has no capitals: the source's Title Case means nothing; names
  keep their own case (mpv, libmpv, yt-dlp, whisper.cpp, dav1d).
- **Sino-Korean words and loanwords.** Use the word a Korean user of
  Windows or a Korean media player knows, whichever its origin. General
  interface words follow Korean Windows' usage (설정, 창, 대화 상자,
  알림 영역, 작업 표시줄, 휴지통, 끌어서 놓기, 도구 설명, 기본값, 사용자
  지정, 형식, 글꼴, 장치). Media words that Korean players say in
  Sino-Korean stay Sino-Korean (재생 목록, not 플레이리스트; 자막; 음소거;
  화면 비율; 자르기; 회전; 밝기, 대비, 채도, 색조); those that are said
  as loanwords stay loanwords (스킨, not 외피; 코덱; 필터; 렌더러; 디코더;
  셰이더; 프레임; 챕터; 북마크; 스트림; 트랙; 프리셋; 볼륨; 오디오; 비디오).
  Never coin a transliteration where a Korean word is usual (열기, not
  오픈; 설정, not 세팅). Loanwords take their standard spelling: 셰이더
  (not 쉐이더), 캡처 (not 캡쳐), 메시지 (not 메세지), 콘텐츠 (not 컨텐츠),
  컨트롤러, 리모컨, 스크립트, 플러그인, 디렉터리 (where a directory is meant),
  with one common exception: 썸네일 (the standard 섬네일 is rarely seen in
  software). No Hanja, and no English in parentheses after a Korean word
  except names and abbreviations the source has.
- **Word spacing** follows 한글 맞춤법. Particles and endings attach to
  the word before them; dependent nouns and auxiliary verbs are spaced:
  "재생 중", "할 수 있습니다", "사용 안 함", "표시할 때", "한 번", "두 번째".
  The glossary's compound terms keep its spacing everywhere ("재생 목록",
  "일시 정지", "미리 보기", "대화 상자", "작업 표시줄", "화면 보호기",
  "다른 이름으로 저장", "실행 취소", "붙여넣기", "끌어서 놓기", "즐겨찾기").
  A name, abbreviation or placeholder followed by a Korean noun takes a
  space ("mpv 폴더", "RVP 창", "DVD 메뉴", "NVIDIA 카드", "%1 엔진"); a
  particle attaches to it directly ("RVP가", "mpv의", "SVP에서", "%1의").
  Korean counters and units attach to a number or a placeholder ("5초",
  "%1개", "%1줄", "%1비트", "%1번째"), and so do the Latin units (see
  Units). A slash keeps the source's spacing ("재생/일시 정지", "%1 / %2").
- **Punctuation.** Korean uses the source's period, comma, question mark
  and colon; nothing is spaced before ":", "?", "!", ")" or ",", and one
  space follows a colon. Avoid exclamation marks. A semicolon in a help
  text usually becomes two sentences or a comma clause; inside lists of
  values (such as "Opening;Ending;3") it stays. A dash that sets off a
  phrase becomes a colon, a comma or parentheses; the source's " - " in
  menu labels such as "%1/%2/%3 - %4 Stereo" stays. "&" becomes "및" in
  labels and "와/과" in sentences ("Luma & Chroma Offset" "휘도 및 색차
  오프셋"). "e.g." is "예:", "etc." "등" (spaced: "PNG, JPEG 등").
- **Parentheses.** An opening parenthesis attaches to the word before it,
  as Korean writes and Korean Windows does with "(권장)": "자동
  선택(권장)", "16비트(권장)", "독점 모드(장치가 RVP만 재생)", also after
  a Latin word or a placeholder ("small(다운로드 안 됨)", "“%1”(%2)").
  Names that contain a parenthesis keep it as they are written: "RIFE
  (TensorRT)", "RIFE (TensorRT, NVIDIA)". Inside, no space after "(" or
  before ")".
- **Quotation marks** are “…” (and ‘…’ inside them), for the source's
  “…” and for its straight "...": “%1”, “...로 재생”. When a text names
  another label, page or menu, it quotes that label's Korean text exactly
  and adds what it is: "“재생” 페이지", "“고급” 버튼", "“시작” 버튼을
  누르면". A particle after a quote follows the last sound of the quoted
  Korean text (“재생”을, “자막”은, “기타”를). Menu paths keep the
  source's ">" between the Korean labels.
- **"..." and "…".** The end of a menu item or button keeps exactly what
  the source has: three ASCII periods "..." where the source has them
  (the menus), the single character "…" where it has that (the QML
  windows), attached to the word without a space ("열기...",
  "찾아보기…", "캘리브레이션…"). Where the source has a space before it
  ("Shutdown at ...") the space stays. Never replace one form by the
  other, and never use the Korean ellipsis "⋯".
- **Keys** keep RVP's own names, the ones its menus print: Ctrl, Shift,
  Alt, Enter, Space, Esc, Tab, PgUp, PgDn, Home, End, Del, Ins,
  F1...F12, Num+, Left, Right, Up and Down inside key names and lists of
  keys ("Ctrl+Left / Right"), and the letters and punctuation keys as in
  the source; never 스페이스, 엔터 or 한/영-style labels. In a sentence a
  key takes "키" and its particle attaches to that: "Enter 키를
  누르십시오", "Esc 키를 누르면 취소됩니다", "Shift+Enter 키로 새 줄을
  시작합니다". Keys in general are Korean words: "화살표 키", "볼륨 키",
  "미디어 키". The gamepad's buttons (A, B, X, Y, Back, Start) and the
  mouse's X1 and X2 keep their labels; the mouse's back and forward
  buttons are "뒤로" and "앞으로" buttons ("뒤로 버튼", "앞으로 버튼").
  Pointer actions: click 클릭, double click "두 번 클릭", right-click
  "오른쪽 클릭", left button "왼쪽 버튼", drag 끌기, wheel up / down "휠
  위로" / "휠 아래로".
- **Units.** Seconds, minutes and hours are Korean: "초", "분", "시간",
  attached to the number ("%1초", "5분"); "Sec", "Sec." and "Min" in the
  menus become "초" and "분" ("%1 Sec Backward" "%1초 뒤로", "Skip Intro
  %1 Sec" "오프닝 %1초 건너뛰기"). Milliseconds stay "ms" in labels and
  values ("%1ms", the subtitle browser's "ms") and are "밀리초" when a
  sentence spells them out ("Audio sync in milliseconds" "오디오
  싱크(밀리초)").
  dB, Hz, kHz, fps, px, K, p (1080p), GB, MB, W, ° and % stay Latin and
  attach to the number, as Korean software writes them ("%1%", "60Hz",
  "-3dB", "%1GB", "%1W", "%3p", "360°"), also where the source puts a
  space ("%1 %" "%1%", "%1 s" "%1초", "(%4 %)" "(%4%)"). "frames a
  second" in a sentence is "초당 %1프레임" or "초당 프레임 수". A
  multiplier "x" after a number is "배" ("%1x video output size" "비디오
  출력 크기 %1배", "%1x Forward" "%1배속 앞으로"); "x" between sizes stays
  ("%1x%2", "1920x1080", "3x2").
- **Numbers and counters.** The decimal separator is a period, as in the
  source ("1.5초"). Korean has no plural: "file(s)", "item(s)", "line"
  and "lines" are the bare noun, and a count takes a counter after the
  number: things 개 ("파일 %1개", "%1 Images" "이미지 %1개"), lines 줄,
  rows 행, columns 열, times 번 or 회, ordinals 번째 ("%1nd Device"
  "%1번째 장치", "%1st logo" "%1번째 로고"), bits 비트 ("%1-bit" "%1비트",
  "10비트 출력"). Where the source has separate singular and plural
  strings ("Reserve %1 line", "Reserve %1 lines"), both get the same
  Korean. "%1 of %2" turns around: "%2 중 %1"; "%2 of %3 GB" is "%3GB 중
  %2GB" (a unit may be repeated, a placeholder never). Notation RVP
  parses or that is a name keeps the source's form: channel layouts
  (5.1, 7.1), aspect ratios and sizes (16:9, 2.35:1, %1:%2, %1.%2),
  versions, ranges (0-255, 16-235) and input formats ("h:mm:ss.s",
  "hh:mm", "h:mm").
- **Particles after placeholders and Latin names.** The particles 이/가,
  을/를, 은/는, 과/와, 으로/로, 이나/나 depend on the last sound of the word
  before them, and %1 can be anything. In order of preference:
  1. Put a Korean noun or counter after the placeholder and attach the
     particle to it: "%1 키는", "%1 엔진이", "“%3” 동작으로", "파일
     %1개를", "%1초 동안". This is also the way for names whose reading is
     uncertain (Direct3D 11, X11, AV1: "Direct3D 11 렌더러로").
  2. Use a particle that never changes: 의, 에, 에서, 에게, 도, 만, 까지,
     부터, 보다, 처럼 ("%1에 측정했습니다.", "%1의 옵션").
  3. Put the placeholder after a colon or at the end ("상태: %1", "사용
     중: %1", "모델: %1").
  4. Only where nothing else reads well, the paired form with the
     consonant form first: 이(가), 을(를), 은(는), 과(와), (으)로,
     (이)나 ("%1을(를) 제거했습니다").
  After a fixed Latin name, write the one particle that its usual Korean
  reading takes. Ending in a vowel (가, 를, 는, 와, 로): RVP, mpv, libmpv,
  SVP, PotPlayer, BlackSpace, VapourSynth, MVTools, RIFE, TensorRT, CUDA,
  NVIDIA, AMD, Qt, Windows, Linux, Wayland, Direct3D, DirectShow, DVD,
  Blu-ray, GPU, CPU, KDE, VLC, Winamp, MPRIS, PulseAudio, PipeWire,
  Unified Remote, whisper.cpp, yt-dlp, AviSynth, TomSteady, PNG, SRT,
  ASS, WebVTT. Ending in a consonant (이, 을, 은, 과, 으로): FFmpeg,
  Vulkan, GNOME, JPEG, NVDEC. Ending in ㄹ (이, 을, 은, 과, but 로):
  OpenGL, HDR, SDR. After a quoted Korean label the particle follows
  its last syllable.
- **Possessives and the source's "X's".** In labels a name before a noun
  is enough: "mpv's default" "mpv 기본값", "PotPlayer's skins" "PotPlayer
  스킨", "Windows' default" "Windows 기본값", "RVP's own filters" "RVP 자체
  필터". In sentences "의" where the relation needs it, never two "의" in a
  row: "NVIDIA의 약관", but "RVP VapourSynth 런타임의 MVTools", not
  "RVP의 VapourSynth 런타임의 MVTools".
- **Values stay as they are.** Example and default values that RVP
  matches or parses keep the source's text: chapter titles to skip
  ("Opening;Ending;3"), language codes ("rus,eng", "en,ru", "en"),
  drive letters and mount points ("V:", "/mnt/media"), the yt-dlp format
  ("bestvideo+bestaudio/best"), file patterns ("*.dsf", "*.json"), the
  default folders RVP really uses ("Music/RVP"; "Pictures/RVP in the
  home folder" is "홈 폴더의 Pictures/RVP"), settings keys and the
  playlist's album named "Default" (data, shown as “Default”). "RVP
  Default", the skin's name, stays. Whisper's model names (tiny, base,
  small, medium, large, large-v%1-turbo) stay; their notes are Korean
  ("(다운로드 안 됨)").
- **Language names** are the Korean names with "-어" where Korean uses it:
  "아프리칸스어", "아랍어", "독일어", "아이티 크리올어", "히브리어", "영어",
  "중국어", "이디시어", and "뉘노르스크" (no "-어"). The language's own name (the
  Language context) is "한국어".
- **Words with two meanings in the source.**
  - Default: a value is "기본값" (the Default button, "mpv's default"
    "mpv 기본값"); before a noun "기본" ("Default Audio Device" "기본
    오디오 장치", "Default window size" "기본 창 크기").
  - Title: a name or heading (a bookmark's "Title:", the window title)
    is "제목"; a disc's title is "타이틀".
  - Extension: a file's extension is "확장자"; extending the picture is
    "확장" ("Extend/Crop" "확장/자르기").
  - Close, Exit, Close Playback and Shut down are "닫기", "끝내기", "재생
    종료" and "시스템 종료"; "Only exit RVP" is "RVP만 끝내기".
  - Remember is "기억", save and store "저장"; Load "불러오기", Reload
    "다시 불러오기".
  - Picture, image, screen and display: the video's picture is "영상",
    a still image or a picture file "이미지", the area of the monitor
    "화면", the monitor itself "디스플레이" ("The primary display" "주
    디스플레이", "the display's rate" "디스플레이 주사율").
  - Track and stream: "트랙" and "스트림", each for its own English word
    ("Cycle Audio Stream" "오디오 스트림 전환", "subtitle track" "자막 트랙").
  - Cycle (step through the choices) is "전환": "Cycle Aspect Ratio"
    "화면 비율 전환".
  - Move: a position, point or picture moves by "이동" ("Move Point A %1
    Sec. Forward" "A 지점 %1초 앞으로 이동"); a key, an item or a file that
    goes elsewhere by "옮기기" ("Move it to “%3”?" "“%3” 동작으로
    옮기시겠습니까?", the Move button "옮기기").
  - Increase / Decrease: levels (volume, brightness, contrast,
    saturation, hue) "높이기" / "낮추기"; sizes, margins, spacing and
    amounts "늘리기" / "줄이기"; Decrease / Increase of an audio or
    subtitle sync moves it earlier / later, as the Russian reads it:
    "앞당기기" / "늦추기" ("Decrease Subtitle Sync by %1ms" "자막 싱크
    %1ms 앞당기기").
  - Insert in the subtitle browser (a new line) is "삽입", Paste
    "붙여넣기".
  - The "nav bar" and "navigation bar" of PotPlayer's menus are the seek
    bar ("탐색 막대").
  - Sync of audio and subtitles is "싱크"; "동기화" is left for keeping
    data the same in two places, which RVP does not do.
- **Length.** Korean is compact, but a Hangul syllable counts as two
  Latin letters, and a label can still run past the settings window's
  240-pixel labels and 300-pixel choice lists. "Open File(s)" (12) is
  "파일 열기" (9), "Context menu" (12) "컨텍스트 메뉴" (13).
  - Drop what the English needs and Korean does not: articles,
    possessives, pronouns and the subject ("Show the mouse pointer"
    "마우스 포인터 표시", "Remember the A-B repeat lists" "A-B 반복 목록
    기억").
  - Prefer a verbal noun to a native phrase ("표시" over "보여 주기",
    "저장" over "저장해 두기") and a short particle to a long ending.
  - An explanation in parentheses may be shortened when the label stays
    clear: "(Recommended)" and "(Recom'd)" are "(권장)". No other
    abbreviations are made up.
  - Where the source's label repeats its heading ("Enable (Shift+N)"
    under "Normalizer"), the Korean repeats nothing either: "사용(Shift+N)".
- **Plain text.** No markup, no line breaks the source lacks, and the
  line break the source has (such as "%1\nin %2 seconds") stays.

## Glossary

| English | Korean | Note |
|---------|--------|------|
| play / pause / stop | 재생 / 일시 정지 / 정지 | "일시 정지" spaced; "Play/Pause" is "재생/일시 정지". |
| playback | 재생 | The Playback menu and page are "재생". |
| close playback | 재생 종료 | Not "닫기" (a window) nor "끝내기" (Exit). |
| previous / next | 이전 / 다음 | "Previous Chapter" "이전 챕터". |
| seek | 탐색 | Menu commands that move by a time are "%1초 앞으로" / "%1초 뒤로". |
| jump | 이동 | "Jump to Specified Time/Frame" "지정한 시간/프레임으로 이동". |
| frame step | 프레임 단위 이동 | "Next Frame" is "다음 프레임". |
| speed | 재생 속도 | "속도" alone where the page or menu is about playback. |
| repeat | 반복 | Not "다시 실행" (Redo). |
| A-B repeat / section repeat | A-B 반복 / 구간 반복 | |
| shuffle | 무작위 재생 | "무작위" alone as a choice next to "반복". |
| queue | 재생 대기열 | "대기열" where "재생" is already said. |
| resume / remember position | 이어서 재생 / 재생 위치 기억 | |
| position | 재생 위치 | "위치" for subtitles, the logo, windows. |
| duration | 길이 | As Windows' file list names it; "Duration: %1" "길이: %1". |
| elapsed / remaining time | 경과 시간 / 남은 시간 | |
| chapter | 챕터 | |
| bookmark | 북마크 | Not 책갈피, so that one word is used throughout. |
| skip (intro, ending) | 건너뛰기(오프닝, 엔딩) | "Skip Intro %1 Sec" "오프닝 %1초 건너뛰기"; chapter titles to match stay English. |
| seek bar | 탐색 막대 | Also PotPlayer's "nav bar". |
| thumbnail | 썸네일 | Not 미리 보기, which is the preview. |
| preview | 미리 보기 | Spaced. |
| scene browser | 장면 탐색기 | |
| file | 파일 | "File(s)" is "파일". |
| folder | 폴더 | |
| URL / address | URL / 주소 | |
| stream | 스트림 | Network media and a track inside a file alike. |
| device / capture device | 장치 / 캡처 장치 | "캡처", never "캡쳐". |
| screen capture | 화면 캡처 | |
| disc | 디스크 | |
| title | 타이틀 | A disc's title only; a name or heading is "제목". |
| angle | 앵글 | |
| disc menu | 디스크 메뉴 | |
| drag and drop | 끌어서 놓기 | "dropped files" "끌어 놓은 파일"; drag alone "끌기". |
| recent files | 최근 파일 | |
| favorites | 즐겨찾기 | One word, no space. |
| file association / file type | 파일 연결 / 파일 형식 | |
| playlist | 재생 목록 | Never 플레이리스트. |
| album | 앨범 | |
| entry | 항목 | |
| sort | 정렬 | "Sort by" "정렬 기준". |
| add / remove / delete | 추가 / 제거 / 삭제 | "제거" takes an entry off the list and leaves the file; "삭제" deletes it (and is also Delete for albums, bookmarks, lines). |
| recycle bin / trash | 휴지통 / 휴지통 | Windows' and the Linux desktops' word alike. |
| import / export | 가져오기 / 내보내기 | |
| window / main window | 창 / 메인 창 | Not "기본 창", which reads as "default window". |
| fullscreen / windowed | 전체 화면 / 창 모드 | |
| maximize / minimize | 최대화 / 최소화 | "Restore" is "복원". |
| always on top / stay on top | 항상 위 / 항상 위에 표시 | |
| opacity | 불투명도 | Not 투명도 (transparency), the opposite scale. |
| skin | 스킨 | |
| color theme | 색 테마 | "색", not "색상", which is hue in HSB pickers. |
| logo | 로고 | |
| title bar | 제목 표시줄 | |
| control panel | 제어 패널 | Not "제어판", Windows' Control Panel. |
| OSD / on-screen display | OSD / 화면 표시 | "OSD" in labels; "화면 표시" where a sentence explains it. |
| on-screen controller | 화면 컨트롤러 | mpv's OSC and the user's scripts. |
| tooltip | 도구 설명 | Korean Windows' term. |
| context menu | 컨텍스트 메뉴 | "the right-click menu" is the same. |
| tray icon / notification area | 트레이 아이콘 / 알림 영역 | |
| taskbar | 작업 표시줄 | Also for the dock on Linux. |
| notification | 알림 | |
| dialog | 대화 상자 | "modeless dialog boxes" "모덜리스 대화 상자". |
| settings / preferences | 설정 / 환경 설정 | "X Settings..." is "X 설정...". |
| page | 페이지 | "the Playback page" "“재생” 페이지". |
| preset / configuration | 프리셋 / 구성 | Not 사전 설정; "Manage Presets..." "프리셋 관리...". |
| reset / default | 초기화 / 기본값 | Before a noun "기본" ("기본 오디오 장치"). |
| apply | 적용 | A filter or effect is "적용", a setting too. |
| key / hotkey / shortcut | 키 / 단축키 / 단축키 | In a sentence "Enter 키". |
| mouse / wheel / button | 마우스 / 휠 / 버튼 | "단추" is not used. |
| gesture | 제스처 | |
| touch | 터치 | |
| controller / gamepad | 컨트롤러 / 게임패드 | |
| video / picture / image | 비디오 / 영상 / 이미지 | "영상" is the picture on screen; a still or a picture file is "이미지". |
| frame | 프레임 | |
| frame rate / fps | 프레임 속도 / fps | "초당 프레임 수" where a sentence spells it out. |
| aspect ratio | 화면 비율 | "AR" in PotPlayer's menus is also "화면 비율"; "비율" alone in tight compounds. |
| crop | 자르기 | |
| zoom / pan | 확대 / 이동 | Zoom In / Zoom Out are "확대" / "축소"; pan takes the picture as object ("영상 이동") where "이동" could read as a jump. |
| rotate / flip / mirror | 회전 / 상하 반전 / 좌우 반전 | |
| deinterlace | 디인터레이스 | |
| brightness / contrast / saturation / hue / gamma | 밝기 / 대비 / 채도 / 색조 / 감마 | |
| sharpen / denoise | 선명하게 / 노이즈 제거 | "선명도" for the amount; video noise is "노이즈", sound noise "잡음". |
| scaler / upscaling / resize | 스케일러 / 업스케일링 / 크기 조정 | |
| pixel shader | 픽셀 셰이더 | "셰이더", never "쉐이더". |
| renderer | 렌더러 | |
| hardware decoding / decoder | 하드웨어 디코딩 / 디코더 | |
| HDR | HDR | Reads 에이치디알: "HDR을", "HDR로". |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / 360° 비디오 / 애너글리프 / 좌우 분할 / 상하 분할 | SBS and TAB stay as abbreviations. |
| bars / letterbox / margin | 검은 띠 / 레터박스 / 여백 | "screen margins" "화면 여백". |
| frame capture / screenshot | 프레임 캡처 / 스크린샷 | |
| motion smoothing | 모션 스무딩 | The feature; the technique is "프레임 보간". |
| frame interpolation | 프레임 보간 | |
| engine | 엔진 | |
| calibration | 캘리브레이션 | Not "보정" (correction); "Recalibrate…" "다시 캘리브레이션…". |
| quality preset (light, medium, high) | 품질 프리셋(가벼움, 중간, 높음) | The calibration's Fast / Medium / Quality are "빠름" / "중간" / "품질". |
| GPU / graphics card | GPU / 그래픽 카드 | |
| processor / CPU | 프로세서 / CPU | |
| power budget | 전력 한도 | RVP's own limit; the card's power limit, which RVP never changes, is "전력 제한". |
| dropped frames | 누락된 프레임 | "누락 프레임" in counters. |
| refresh rate | 주사율 | The word Korean users know for a monitor's Hz. |
| audio / sound | 오디오 / 소리 | "오디오" in menus and labels, "소리" in sentences. |
| volume / mute | 볼륨 / 음소거 | |
| master volume | 마스터 볼륨 | The system's volume; "System Volume" is "시스템 볼륨". |
| equalizer | 이퀄라이저 | |
| normalizer | 노멀라이저 | "Auto Volume Control" "자동 볼륨 조절". |
| reverb | 리버브 | |
| noise reduction | 잡음 감소 | Sound; the picture's is "노이즈 제거". |
| channel / speaker | 채널 / 스피커 | |
| downmix | 다운믹스 | |
| surround / stereo | 서라운드 / 스테레오 | |
| output device | 출력 장치 | |
| sample rate / bitrate | 샘플 레이트 / 비트레이트 | |
| audio delay / audio sync | 오디오 지연 / 오디오 싱크 | "싱크", not "동기화". |
| pitch / time stretching | 음높이 / 타임 스트레칭 | |
| audio recording | 오디오 녹음 | |
| second audio device | 두 번째 오디오 장치 | "the 2nd device" "두 번째 장치". |
| pass-through | 패스스루 | |
| subtitles / subtitle track | 자막 / 자막 트랙 | |
| external / embedded subtitles | 외부 자막 / 내장 자막 | |
| font / outline / shadow | 글꼴 / 외곽선 / 그림자 | |
| subtitle position / subtitle delay | 자막 위치 / 자막 지연 | |
| encoding | 인코딩 | A text's character set; AC3 encoding too. |
| subtitle browser / subtitle editor | 자막 탐색기 / 자막 편집기 | |
| subtitles from audio / speech recognition | 오디오로 만든 자막 / 음성 인식 | "Create subtitles from audio..." "오디오로 자막 만들기...". |
| model | 모델 | Model names stay English. |
| language | 언어 | |
| remote control | 원격 제어 | A remote on a phone is "리모컨". |
| token | 토큰 | |
| port / server | 포트 / 서버 | |
| log | 로그 | |
| crash | 비정상 종료 | "crash logs" "비정상 종료 로그". |
| download / install | 다운로드 / 설치 | |
| installer | 설치 프로그램 | |
| accessibility | 접근성 | |
| text to speech (TTS) / screen reader | 텍스트 음성 변환(TTS) / 화면 읽기 프로그램 | "TTS" alone as a page title. |
| screen saver | 화면 보호기 | |
| shut down / sleep / hibernate | 시스템 종료 / 절전 / 최대 절전 | "최대 절전 모드" where a mode is meant. |
| timer | 타이머 | "Shutdown Timer" "종료 타이머". |
| visualization / spectrum | 시각화 / 스펙트럼 | |
| status bar / info | 상태 표시줄 / 정보 | |
| codec / format | 코덱 / 형식 | |
| bit depth / 10-bit | 비트 심도 / 10비트 | "%1-bit" "%1비트", attached. |
| channel layout (5.1, 7.1) | 채널 구성(5.1, 7.1) | |
| Auto select (recommended) | 자동 선택(권장) | "Auto" alone is "자동"; "(Recommended)" is always "(권장)", attached. |
