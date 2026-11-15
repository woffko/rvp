# Spanish (es)

One neutral Spanish for Spain and Latin America alike: spelling,
accents and punctuation as the RAE and the other academies (ASALE) set
them for every country, and words that every Spanish speaker reads
without effort. Where Spain and the Americas use different everyday
words (ordenador and computadora, móvil and celular, pulsar and oprimir),
the translation takes a word both know (equipo, teléfono, presionar) or
the one the RAE and Windows' Spanish use in both regions; the Style
section lists these choices. The register is that of a desktop program:
plain, polite and concise, never colloquial. Translate what the control
does in RVP, reading the comment, the context's other strings and the
Russian; English word order and structures are not kept where Spanish
says it otherwise.

## Style

- **Form of address: usted**, the form every country reads as neutral;
  never tú, vos or vosotros. Most strings need no address at all: say
  what RVP does in the third person ("RVP mide...") or with the
  impersonal se ("Se aplica la próxima vez que se inicie RVP").
  Instructions inside sentences use usted's imperative ("Seleccione una
  acción y agregue una tecla o un gesto del ratón.", "Presione una
  tecla para “%1” (%2). Esc cancela."); "your" is su ("sus propias
  teclas se mantienen"). Questions in dialogs use the infinitive
  ("¿Enviar %1 archivo(s) a la Papelera de reciclaje?") rather than
  "¿Desea...?". No "por favor", no exclamations.
- **Commands** (menu items, buttons, actions) are infinitives in sentence
  case: "Abrir carpeta...", "Agregar marcador", "Restablecer", "Guardar
  como…". Never the imperative ("Abra") or a noun of action
  ("Apertura") for a command.
- **Labels that name a feature stay names**: "Playback" →
  "Reproducción", "Audio Sync" → "Sincronización de audio", "Stay on Top"
  → "Siempre encima". "Enable X" / "Disable X" → "Activar X" /
  "Desactivar X" (not habilitar/deshabilitar).
- **Checkboxes** say what the box turns on, as an infinitive when the
  source starts with a verb ("Show tooltips" → "Mostrar descripciones
  emergentes", "Remember video playback positions" → "Recordar la
  posición de los videos"), otherwise as a noun phrase.
- **Choices** (radio buttons, lists) are nouns or adjectives that agree
  with the noun they qualify, usually the setting's label: "Prioridad
  del proceso" → "Baja", "Normal", "Alta"; "Preajuste" → "Ligero",
  "Medio", "Alto". On / Off as a choice: "Activado" / "Desactivado"
  (they qualify an implicit modo); "None": "Ninguno" or "Ninguna" by the
  noun; "Default": "Predeterminado" or "Predeterminada" by the noun.
  "(Recommended)" agrees too: masculine after an infinitive or a
  masculine noun ("Mantener la proporción (recomendado)"), "(recomendada)"
  after a feminine one.
- **Page titles, window titles and group headings** are noun phrases
  ("Teclas y ratón", "Pantalla completa", "Configuración del logotipo");
  a title that is a command keeps its infinitive ("Agregar marcador").
  Field labels keep the source's final colon ("Nombre del álbum:").
- **Help texts** are full sentences in the present tense, short and
  direct. When one names another option, page, menu item or button, it
  quotes that item's Spanish label exactly as translated, in quotation
  marks: con “Ajustar la ventana al video nuevo” activado.
- **Capitalization: sentence case everywhere.** Only the first word and
  names take a capital: "Abrir archivos...", "Repetición de sección",
  "Modo de pantalla completa"; the English Title Case is not carried
  over. Capitals stay on names (README's list), acronyms (OSD, GPU, URL,
  HDR, DPI), key names, Windows places named as Windows names them
  (Papelera de reciclaje) and RVP Default. Lowercase after a colon,
  after a slash ("Agregar/seleccionar subtítulos"), inside parentheses
  unless a name begins there, and for languages ("inglés"), days,
  months and units. The language list shows this language's own name as
  "Español".
- **Punctuation.** Every question and exclamation opens with ¿ or ¡ where
  it begins, also in the middle of a string: "“%2” ya usa %1.
  ¿Pasar la asignación a “%3”?". No space before a colon, semicolon, question or
  exclamation mark, or inside the marks. A period follows closing
  quotes and parentheses ("...en “Reproducción”."). The source's slash
  pairs keep its spacing ("Reproducir/pausar"); ranges keep the source's
  hyphen ("16-235").
- **Quotation marks** are “ ” (the RAE allows them everywhere, and they
  read as neutral on both sides of the Atlantic), with ‘ ’ inside them;
  never straight " " and not « », which read as Spain's. Where the source
  has straight quotes, Spanish still writes “ ”.
- **"..."**: keep exactly what the source has at the end of a label:
  three periods "..." in the menus, the single character "…" where the
  QML text has it ("Elegir…", "Midiendo %1…"). No space before it, and
  never drop or add one.
- **"&"** is written "y" ("Copiar la ruta y el nombre..."). "y" becomes
  "e" before a word that begins with the sound i ("audio e imagen"), "o"
  becomes "u" before the sound o ("uno u otro"); before a placeholder
  or a number they stay "y" and "o", and "o" between digits takes no
  accent ("1 o 2").
- **Units** are the international symbols, never abbreviated words: s
  (never "seg", "seg." or "sec"), min, h, ms, dB, Hz, kHz, fps, px, GB,
  MB, W. Spanish separates a symbol from its number or placeholder by
  a space, the percent sign included (RAE): "%1 ms", "5 s", "%1 Hz",
  "%1 %", "%2 GB". Attached as in the source: the angle's degree
  ("360°", "%1°"), the multiplier ("%1x"), resolutions ("1080p", "4K")
  and channel layouts ("5.1"). The source's "Sec." and "Sec" become "s";
  "10-bit" is "de 10 bits". Symbols take no period and no plural.
- **Numbers.** RVP prints its numbers with a decimal point ("1.5x"), the
  form the RAE's Ortografía recommends for all countries: numbers written
  in a string use it too. No thousands separator in four-digit numbers
  ("2160p", "1500"); larger ones take a space. Prefer a placeholder after
  a noun to an ordinal ("Audio Renderer (%1nd Device)" → "Renderizador
  de audio (dispositivo %1)"); an ordinal that cannot be avoided is
  "%1.º" (feminine "%1.ª", "1.er" before a masculine noun). "e.g." is
  "p. ej.", "i.e." is "es decir", "#1" stays "#1".
- **Agreement with placeholders.** A placeholder shows no gender or
  number, so nothing in the sentence may have to agree with it. Put the
  noun before it ("la tecla %1", "el archivo %1", "el motor %1"), use a
  label and a colon ("Quitado: %1", "Duración: %1"), or turn the
  sentence so the placeholder is an object of a verb that agrees with
  something else ("“%2” ya usa %1"). The source's "(s)" stays as Spanish's
  "(s)" and every word it governs takes it ("%1 archivo(s)
  seleccionado(s)").
- **Tense.** Prefer the present tense or a participle for what just
  happened ("Aplicado.", "Importado.", "Cancelado."); where a sentence
  needs a past, the compound perfect ("Se ha cancelado"), which reads
  correctly in every country.
- **Names and gender.** Names stay as README lists them; program names
  take no article ("con SVP", "RVP mide"), and are masculine where
  something must agree (el programa). Acronyms take no plural -s ("las
  GPU", "los DVD", "las URL"). The English possessive becomes "de": "RVP's
  own skin" → "la máscara propia de RVP", "mpv's OSC" → "el OSC de mpv".
  Accents are kept on capitals too ("Índice", "Área") and on borrowed
  words: códec, píxel, álbum.
- **Words of one region** are avoided; these are the choices: equipo
  (computer; never ordenador or computadora), teléfono (never móvil or
  celular), ratón (never mouse), presionar a key (never pulsar or
  oprimir), hacer clic (never cliquear, clicar or pinchar), agregar
  (añadir is understood, but one verb is used), quitar (never remover,
  which in Spain means to stir), altavoz (never bocina or parlante),
  video (the form most speakers use, valid everywhere for the RAE;
  never vídeo), icono (the RAE's preferred form; never ícono), archivo (never fichero), descargar (never
  bajar), configuración (never ajustes for the settings), tomar (never
  coger), error (never fallo or falla, which differ by region),
  predeterminado (never por defecto), compatible con or admite (never
  soporta), biblioteca (never librería), guardar (never salvar),
  reproducir a file (never tocar or jugar), activar / desactivar a
  feature (prender and apagar are not for features; apagar is only for
  the computer).
- **Keep short labels short.** Menu items, buttons, checkboxes and
  choices stay about as long as the English, a third longer at most.
  Leave out the articles a label can do without ("Abrir carpeta", not
  "Abrir una carpeta"; sentences keep theirs), prefer an adjective to a
  "de" phrase where Spanish has one ("audio externo"), and take the
  shorter of two equal words ("Mostrar", not "Visualizar"). Inside the
  playlist window and its menus "lista" may stand for "lista de
  reproducción" where nothing else could be meant. Where the source
  abbreviates aspect ratio as "AR", Spanish says "proporción"
  ("Fullscreen (Keep AR)" → "Pantalla completa (mantener proporción)").
  No word is abbreviated ("config.", "sinc.", "info" are not used);
  help texts may grow.
- **Plain text** as the source: keep its line breaks (and add none),
  its placeholders and its key names (Ctrl, Shift, Alt, Enter, Space,
  Esc, Tab, F1...F12, PgUp, PgDn, Home, End), also in sentences
  ("Presione Enter").

## Glossary

| English | Spanish | Note |
|---------|---------|------|
| play / pause / stop | reproducir / pausar / detener | The one toggle "Play/Pause" is "Reproducir/pausar"; the state is "en pausa". Never jugar or tocar. |
| playback | reproducción | The Playback menu and page: "Reproducción". |
| close playback | cerrar la reproducción | Stops and unloads the file. |
| previous / next | anterior / siguiente | After the noun: "capítulo anterior", "pista siguiente". |
| seek | cambiar la posición | Noun: cambio de posición; a short choice may say "Posición". Never buscar (that is search) or desplazarse (that is scrolling). |
| jump | saltar | Noun: salto ("Saltos exactos"). A menu or button that names only the destination may say "Ir a" ("Jump (to)" → "Ir a"). |
| frame step | fotograma a fotograma | "Next frame" → "Fotograma siguiente". |
| speed | velocidad | |
| repeat | repetir | Noun: repetición. |
| A-B repeat / section repeat | repetición A-B / repetición de sección | The points: "punto A", "punto B". |
| shuffle | orden aleatorio | As a short toggle "Aleatorio". |
| queue | cola de reproducción | Short: "cola" where the context says it. |
| resume / remember position | reanudar / recordar la posición | The page "Resume": "Reanudación". |
| position | posición | |
| duration | duración | |
| elapsed / remaining time | tiempo transcurrido / tiempo restante | As short choices: "Transcurrido", "Restante". |
| chapter | capítulo | |
| bookmark | marcador | |
| skip (intro, ending) | omitir (introducción, final) | Never saltar here: that is jump. |
| seek bar | barra de posición | Not barra de búsqueda (search) nor barra de desplazamiento (scrollbar). |
| thumbnail | miniatura | |
| preview | vista previa | Thumbnail previews: "vistas previas en miniatura". |
| scene browser | explorador de escenas | Browse scenes: "explorar escenas". |
| file | archivo | Never fichero. |
| folder | carpeta | The home folder: "carpeta personal". |
| URL / address | URL / dirección | La URL (feminine), plural las URL. |
| stream | flujo | Where the source means one of a file's tracks ("Select Audio Stream", "Preferred stream"), say pista, the track's word; "Record Stream" → "Grabar flujo". |
| device / capture device | dispositivo / dispositivo de captura | |
| screen capture | captura de pantalla | The desktop as a video source; saving the shown frame is frame capture below. |
| disc | disco | |
| title | título | |
| angle | ángulo | |
| disc menu | menú del disco | |
| drag and drop | arrastrar y soltar | |
| recent files | archivos recientes | |
| favorites | favoritos | "Edit Favorites" → "Editar favoritos". |
| file association / file type | asociación de archivos / tipo de archivo | |
| playlist | lista de reproducción | Short "lista" only where nothing else can be meant (see Style). |
| album | álbum | Plural álbumes. |
| entry | elemento | Also for "item". |
| sort | ordenar | Ascending / descending: "ascendente" / "descendente". |
| add / remove / delete | agregar / quitar / eliminar | Quitar takes an entry out of the list; eliminar deletes the file itself. Never remover. |
| recycle bin / trash | Papelera de reciclaje / papelera | Windows' Recycle Bin keeps its capital; the desktop's trash is lowercase. |
| import / export | importar / exportar | |
| window / main window | ventana / ventana principal | |
| fullscreen / windowed | pantalla completa / en ventana | "In fullscreen": "en pantalla completa". |
| maximize / minimize | maximizar / minimizar | The menu's Restore: "restaurar". |
| always on top / stay on top | siempre encima / mantener encima | Over the other windows; not siempre visible, which reads as never hidden. |
| opacity | opacidad | |
| skin | máscara | The word Windows' media player made known; feminine (la máscara). Not skin, whose gender wavers, nor tema, which is the color theme's. |
| color theme | tema de color | |
| logo | logotipo | |
| title bar | barra de título | |
| control panel | panel de control | RVP's F7 window; lowercase, unlike Windows' own. |
| OSD / on-screen display | OSD / mensajes en pantalla | El OSD (masculine). |
| on-screen controller | controles en pantalla | Plural. PotPlayer's "On Screen Control" page is the same. Never controlador, which is a driver. |
| tooltip | descripción emergente | Plural descripciones emergentes. |
| context menu | menú contextual | |
| tray icon / notification area | icono de la bandeja / área de notificación | The tray alone: "bandeja"; el área (feminine noun with el). Icono, not ícono. |
| taskbar | barra de tareas | Also the dock. |
| notification | notificación | |
| dialog | cuadro de diálogo | Short: "diálogo". |
| settings / preferences | configuración / preferencias | "X Settings..." → "Configuración de X..."; never ajustes. |
| page | página | |
| preset / configuration | preajuste / configuración | A saved set of settings is a preajuste (equalizer and frame size presets too); the page that manages them, "Configuration", is "Configuraciones", plural, apart from the settings window. |
| reset / default | restablecer / predeterminado | Agrees: predeterminada. "Restore to Default" → "Restaurar predeterminados". Never por defecto. "RVP Default" is a name. |
| apply | aplicar | |
| key / hotkey / shortcut | tecla / tecla rápida / atajo de teclado | |
| mouse / wheel / button | ratón / rueda / botón | Ratón is the RAE's word and understood in every country. A click: "clic", "doble clic", "botón derecho". |
| gesture | gesto | "gesto del ratón". |
| touch | táctil | The page: "Táctil"; the screen: "pantalla táctil". |
| controller / gamepad | control de juego / gamepad | The Xbox one: "control de Xbox". Not mando (Spain's) nor controlador (a driver). |
| video / picture / image | video / imagen / imagen | Video without an accent, plural videos. Where the picture and a still image must be told apart: "la imagen del video". |
| frame | fotograma | Never cuadro, which is also a dialog's word. Keyframe: "fotograma clave". |
| frame rate / fps | velocidad de fotogramas / fps | |
| aspect ratio | relación de aspecto | Where the source writes AR: "proporción". |
| crop | recortar | Noun: recorte. |
| zoom / pan | zoom / mover | "Zoom In" / "Zoom Out" → "Acercar" / "Alejar"; panning is the menu's "Move Left..." → "Mover a la izquierda". |
| rotate / flip / mirror | girar / voltear / reflejar | "Flip Horizontal" → "Voltear horizontalmente", as the source says. |
| deinterlace | desentrelazar | Noun: desentrelazado; interlaced: entrelazado. |
| brightness / contrast / saturation / hue / gamma | brillo / contraste / saturación / matiz / gamma | Hue is matiz, never tono (that is pitch). |
| sharpen / denoise | enfocar / eliminar ruido | Nouns: enfoque, eliminación de ruido. The sound's noise reduction is reducción de ruido. |
| scaler / upscaling / resize | escalador / ampliación / cambiar el tamaño | Upscaler / downscaler: "escalador de ampliación" / "de reducción"; the noun resizing: "cambio de tamaño". |
| pixel shader | sombreador de píxeles | Shader alone: sombreador. Píxel, plural píxeles. |
| renderer | renderizador | Also "Audio Renderer": "renderizador de audio". |
| hardware decoding / decoder | decodificación por hardware / decodificador | Decodificar, not descodificar; software: "por software". |
| HDR | HDR | |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / video de 360° / anaglifo / lado a lado / arriba y abajo | |
| bars / letterbox / margin | barras / formato buzón / margen | Black bars: "barras negras". |
| frame capture / screenshot | captura de fotograma / captura de imagen | Not captura de pantalla, which is the screen capture source. |
| motion smoothing | suavizado de movimiento | |
| frame interpolation | interpolación de fotogramas | |
| engine | motor | |
| calibration | calibración | |
| quality preset (light, medium, high) | preajuste de calidad (ligero, medio, alto) | Masculine, agreeing with preajuste. |
| GPU / graphics card | GPU / tarjeta gráfica | La GPU (feminine), plural las GPU; the card alone: "la tarjeta". |
| processor / CPU | procesador / CPU | La CPU. |
| power budget | límite de potencia | The share of the card's power RVP allows itself ("por encima del límite"); the card's own settings are never changed. |
| dropped frames | fotogramas perdidos | |
| refresh rate | frecuencia de actualización | |
| audio / sound | audio / sonido | The Audio menu and page: "Audio". |
| volume / mute | volumen / silenciar | Up / down: "subir" / "bajar el volumen"; the muted state: "silencio". |
| master volume | volumen del sistema | "Mute (Master)" → "Silenciar (sistema)". |
| equalizer | ecualizador | |
| normalizer | normalizador | |
| reverb | reverberación | |
| noise reduction | reducción de ruido | |
| channel / speaker | canal / altavoz | Plural altavoces. Never bocina or parlante. |
| downmix | mezcla descendente | The verb: "mezclar en menos canales" ("mezclar en estéreo"). |
| surround / stereo | envolvente / estéreo | "sonido envolvente". |
| output device | dispositivo de salida | |
| sample rate / bitrate | frecuencia de muestreo / tasa de bits | |
| audio delay / audio sync | retardo de audio / sincronización de audio | Sync alone: "sincronización"; the verb: "sincronizar". |
| pitch / time stretching | tono / estiramiento temporal | Tono is the sound's only: hue is matiz. |
| audio recording | grabación de audio | |
| second audio device | segundo dispositivo de audio | |
| pass-through | paso directo | "Use pass-through output": "usar la salida de paso directo". |
| subtitles / subtitle track | subtítulos / pista de subtítulos | One line: "subtítulo". |
| external / embedded subtitles | subtítulos externos / subtítulos integrados | Not incrustados, which reads as burned into the picture. |
| font / outline / shadow | fuente / contorno / sombra | |
| subtitle position / subtitle delay | posición de los subtítulos / retardo de los subtítulos | |
| encoding | codificación | Of a text file: "codificación de texto"; AC3 re-encoding: "recodificación". |
| subtitle browser / subtitle editor | explorador de subtítulos / editor de subtítulos | |
| subtitles from audio / speech recognition | subtítulos desde el audio / reconocimiento de voz | |
| model | modelo | |
| language | idioma | Never lenguaje. |
| remote control | control remoto | Also a remote on a phone ("teléfono"). |
| token | token | El token. |
| port / server | puerto / servidor | |
| log | registro | The file rvp.log keeps its name. |
| crash | cierre inesperado | Never fallo or falla, which differ by region. |
| download / install | descargar / instalar | Noun: descarga. Never bajar. |
| installer | instalador | |
| accessibility | accesibilidad | |
| text to speech (TTS) / screen reader | texto a voz (TTS) / lector de pantalla | The page "TTS" stays "TTS". |
| screen saver | protector de pantalla | |
| shut down / sleep / hibernate | apagar / suspender / hibernar | Apagar is only the computer's; a feature is desactivado. |
| timer | temporizador | |
| visualization / spectrum | visualización / espectro | |
| status bar / info | barra de estado / información | Never "info". |
| codec / format | códec / formato | Plural códecs. |
| bit depth / 10-bit | profundidad de bits / 10 bits | As an adjective: "de 10 bits". |
| channel layout (5.1, 7.1) | disposición de canales (5.1, 7.1) | |
| Auto select (recommended) | Automático (recomendado) | |
