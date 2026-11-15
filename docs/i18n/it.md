# Italian (it)

RVP's Italian is standard Italian as written in Italy, which also serves
Italian speakers in Switzerland. It speaks to the user the way current
Italian desktops do (Windows 10 and 11, macOS, Android): informally, with
"tu", plainly and briefly, without bureaucratic formulas. Loanwords stay
where Italian users of media players use them every day (file, playlist,
skin, preset, font, video, audio, codec, renderer); everything else is
plain Italian. Labels use Italian sentence case, and each glossary term
below is used the same way in every string.

## Style

- **Form of address.** "Tu" everywhere, never "Lei" or "voi". Instructions
  are tu imperatives: «Seleziona un'azione, poi aggiungi un tasto o un
  gesto del mouse.», «Riavvia RVP per applicarlo.». Questions ask the user
  directly: «Vuoi spostare %1 file nel Cestino?». Never the infinitive for
  instructions ("Selezionare..."), never «per favore» or «si prega di».
  What RVP does is said in the third person present: «RVP misura...»,
  «Il registro conserva...».
- **Commands** (menu items, buttons, context-menu entries) are tu
  imperatives: «Apri file...», «Aggiungi segnalibro», «Salva con nome…»,
  «Reimposta pagina», «Sfoglia…». English labels that are already nouns
  stay nouns («Velocità», «Sottotitoli»).
- **Checkboxes** say what the option does, as a tu imperative phrase:
  «Ricorda la posizione di riproduzione dei video», «Mostra le anteprime
  sulla barra di avanzamento». PotPlayer's "Use X" and "Enable X" become
  «Usa X» and «Attiva X» unless the bare verb of X reads better.
- **Choices** (radio buttons, combo-box items, submenu choices) are nouns,
  adjectives or short phrases that agree with the setting they belong to:
  «Mai», «Sempre», «Durante la riproduzione»; under «Priorità del
  processo» «Normale», «Alta»; under a preset «Veloce», «Medio». When the
  noun is not visible, write a phrase that needs no agreement.
- **Titles** (settings pages, section headings, dialog and window titles,
  column headers) are nouns or noun phrases: «Riproduzione», «Salto
  temporale», «Dimensioni finestra», «Calibrazione del movimento fluido».
  A title that is a command in English stays a command: «Regola sincronia
  audio».
- **Status texts and help texts** are full sentences with articles and
  normal grammar; they end with a period exactly where the source does.
  Enable and Disable are «Attiva» and «Disattiva» (not «Abilita»); the
  states On and Off are «Attivato» and «Disattivato», agreeing with the
  setting's noun when it is known («Decodifica hardware: disattivata»).
- **Capitalization.** Sentence case everywhere: only the first word and
  proper names take a capital. English Title Case goes: "Open Folder" is
  «Apri cartella», "Adjust Audio Sync" is «Regola sincronia audio». After
  a colon, after "/" and inside parentheses, lowercase («Album/preferiti»,
  «Calibrazione: annullata»), except names. Language names, days and
  months are lowercase («inglese», «russo»); the interface language's own
  name, a label by itself, is «Italiano». Names of places in the system
  keep their capital: «Cestino».
- **Naming RVP's own labels in a text.** When a help text names a page,
  menu, command or button, use its Italian label exactly, in quotation
  marks: "Record Stream saves the file as it comes" is «Registra flusso»
  salva il file così com'è. Menu paths keep ">": «Video > Margine
  inferiore».
- **Quotation marks** are Italian guillemets « », with no spaces inside:
  «Riproduci con». Quotes inside quotes are “ ”. Replace the source's
  "..." and “...” with « ».
- **Other punctuation.** No space before : ; ? ! (unlike the source's
  PotPlayer labels such as "Action : Hibernate", which become «Azione:
  iberna»). The apostrophe is the straight ' as in the source. Keep the
  source's "/", parentheses and hyphens; add no dashes the source lacks.
  "&" becomes «e» (see README). "e.g." is «ad es.», "etc." is «ecc.».
- **The "..." rule.** Copy the ending exactly as the source writes it:
  three periods where it has three periods («Apri file...»), the single
  "…" character where it has "…" («Sfoglia…», «Misurazione di %1…»), and
  no space before it unless the source has one ("Shutdown at ..." keeps
  its space).
- **Units.** s (secondi; also for PotPlayer's "Sec" and "Sec."), min, h,
  ms, dB, Hz, kHz, fps, px, GB, MB, W, K and the resolutions (1080p) are
  written as in the source. Keep the source's spacing around a
  placeholder and its unit («%1 %», «%1%»); where the Italian text writes
  a unit itself, put a space between number and unit («30 s», «2 GB»,
  «-3 dB») and none before % or °. "Seconds" written out is «secondi»,
  "pixels" is «pixel» (invariable).
- **Numbers.** Numbers the translation writes in Italian prose use the
  decimal comma («1,5 secondi»). Everything the program parses or shows in
  a fixed form stays as written: time formats («h:mm:ss.s», «hh:mm»),
  speeds («1.0x»), channel layouts («5.1», «7.1»), ranges such as
  «0-255» and «16-235», ratios («16:9»), versions. Bit depths are written
  without a hyphen: «10 bit», «%1 bit». Spell out small ordinals in fixed
  text («secondo dispositivo»); with a placeholder, avoid the ordinal
  («dispositivo %1») or use «%1º» (masculine) and «%1ª» (feminine).
- **Folder paths and file names** stay as in the source ("Music/RVP",
  "~/Pictures/RVP", "ggml-<model>.bin"): they name folders on disk.
- **Keys** keep the names RVP shows (README), and so do Left, Right, Up and
  Down inside a key combination, as RVP's menus and the Keys and Mouse page
  show them («Ctrl+Left / Right»). In prose the arrow keys are «i tasti
  freccia», the media keys «i tasti multimediali».
- **Keeping short labels short.** Italian runs about a fifth longer than
  English, so: use the imperative («Mostra», not «Mostrare» or «Mostra di»);
  drop articles and "di" chains in short labels where Italian UI does
  («Apri cartella», «Dimensioni finestra», «Font sottotitoli»), but keep
  them in sentences; prefer the glossary's short loanwords (file,
  playlist, skin, preset, font, info, auto); keep the source's "/" instead
  of spelling out «o»; leave out words the context already gives (the
  page's name, "RVP's"). Do not abbreviate words («impost.», «sinc.»):
  only units and «max», «info», «auto» may be short.
- **Gender and agreement.** Every adjective, participle and article must
  agree, and a placeholder hides its noun, so put the noun in front of
  it: «il tasto %1», «il file %1», «il motore %1», «la scheda %1»; or move
  the placeholder after a colon: «Rimosso: %1», «Misurazione: %1».
  Status participles with no clear subject are masculine singular
  («Applicato.», «Annullato.», «Importato.»); with one they agree
  («Calibrazione: annullata»). "(Recommended)" is «(consigliato)»
  agreeing with the label's noun («Selezione automatica (consigliata)»),
  masculine after a verb phrase. "(s)" plurals disappear with
  invariable nouns («Apri file», «Elimina file») and otherwise take the
  plural or a rephrase, never «immagine/i». Where the source has
  separate singular and plural strings, write each in its own number.
- **Loanwords' gender and number.** Invariable in the plural: il file,
  la playlist, la skin, l'album, il video, l'audio, il preset, il font,
  il codec, il decoder, il renderer, il driver, il plugin, il server, il
  token, il timer, il bitrate, il downmix, il pass-through, il controller,
  il gamepad, il menu (no accent), il desktop, lo script, lo shader (and
  il pixel shader), lo screenshot, lo zoom, l'editor, il runtime, la
  webcam, la GPU, la CPU, l'URL (masculine), l'OSD (masculine), l'HDR.
  Italian plurals for Italian words: il logo, i loghi.
- **Articles and elision.** No article before program names: «di RVP»,
  «con SVP», «da mpv», never «dell'RVP». Elide before vowels as Italian
  requires: «l'audio», «dell'album», «all'avvio», «un'immagine»,
  «un'azione», but «un elemento», «un album» (masculine «un» takes no
  apostrophe). Never elide before a placeholder («il file %1», not
  «l'%1»). Write «qual è» (no apostrophe) and «un po'».
- **Accents.** Capital E with an accent is «È», never «E'». Acute on
  «perché», «poiché», «affinché», «né», «sé»; grave on «è», «più»,
  «può», «già», «così», «cioè», «là», «lì». Yes is «Sì», never «Si».
- **Recurring words**, one way everywhere: OK «OK», Cancel «Annulla»,
  Close «Chiudi», Exit «Esci», Save «Salva», Save As «Salva con nome»,
  Open «Apri», Browse «Sfoglia», Choose «Scegli», Select «Seleziona»,
  Select All «Seleziona tutto», Deselect All «Deseleziona tutto», Copy
  «Copia», Paste «Incolla», Rename «Rinomina», Find and Search «Cerca»,
  Undo «Annulla», Redo «Rifai» (not «Ripeti», which is repeat), Restore
  «Ripristina», Customize «Personalizza», Custom «Personalizzato»
  (agreeing), Set up «Configura», Options «Opzioni», None «Nessuno»
  (agreeing), Never «Mai», Always «Sempre», Other «Altro», Misc «Varie»,
  Auto «Auto», player «lettore» («Lettore multimediale»), display and
  screen «schermo», monitor «monitor», home folder «cartella home»,
  click «clic» («clic sinistro», «doppio clic»).

## Glossary

The Italian cell holds the term as the checks look for it (`work.py
compare`); the note gives its other forms and traps.

| English | Italian | Note |
|---------|---------|------|
| play / pause / stop | riproduci / pausa / interrompi | Commands «Riproduci», «Pausa», «Interrompi»; the toggle «Riproduci/pausa». Not «stop» or «ferma». |
| playback | riproduzione | The menu and pages «Riproduzione»; "while playing" is «durante la riproduzione». |
| close playback | chiudi riproduzione | «Chiudi riproduzione» unloads the file; «Interrompi» (stop) keeps it loaded. |
| previous / next | precedente / successivo | Agrees with its noun: «Capitolo successivo», «Traccia successiva»; bare buttons «Precedente», «Successivo». |
| seek | spostamento | Verb «spostarsi» («spostati nel file»); commands «Avanti di %1 s», «Indietro di %1 s». Never «ricerca» (search) or «scorrimento» (scrolling). |
| jump | vai a | Going to a given point: «Vai a tempo/fotogramma...», the button «Vai». The arrow keys' steps are «salti»: Time Jump «Salto temporale», Exact jumps «Salti precisi». Windows' jump list stays «Jump List». |
| frame step | fotogramma per fotogramma | The commands are «Fotogramma successivo» and «Fotogramma precedente». |
| speed | velocità | The rate stays as the program writes it («1.0x»). |
| repeat | ripeti | Noun «ripetizione» («Ripetizione playlist»). «Ripeti» is never redo («Rifai»). |
| A-B repeat / section repeat | ripetizione A-B / ripetizione sezione | «punto A», «punto B»; a marked section is a «sezione» («Sezione marcata (A-B)»). |
| shuffle | casuale | The mode is «Riproduzione casuale»; a short choice «Casuale»; "randomly" «in ordine casuale». |
| queue | coda | «Coda di riproduzione», «Aggiungi alla coda». |
| resume / remember position | ripresa / ricorda posizione | «Ripresa» names the feature; the verb is «riprendere». «Ripresa» also means a camera shot: use it only about playback. Checkboxes «Ricorda la posizione di riproduzione». |
| position | posizione | Also the place of a window or logo («Posizione finestra»). |
| duration | durata | |
| elapsed / remaining time | trascorso / tempo rimanente | Agree: «Tempo trascorso», «Fotogrammi trascorsi», «Trascorso e rimanente». Not «residuo». |
| chapter | capitolo | Plural «capitoli». |
| bookmark | segnalibro | Plural «segnalibri». |
| skip (intro, ending) | salta | «Salta l'intro», «Salta il finale», «Salta i capitoli». The feature, its page and menu are «Salto automatico», kept apart from Time Jump's «Salto temporale». |
| seek bar | barra di avanzamento | Also PotPlayer's "navigation bar" and "nav bar". |
| thumbnail | miniatura | Plural «miniature»; the thumbnail sheet is «immagine di miniature». |
| preview | anteprima | Seek-bar previews: «anteprime in miniatura». |
| scene browser | esplora scene | Commands «Esplora le scene per intervalli di tempo...». |
| file | file | Masculine, invariable: «il file», «i file», «3 file»; never «files». |
| folder | cartella | |
| URL / address | URL / indirizzo | «l'URL», «gli URL» (masculine, invariable). |
| stream | flusso | Network media; a track inside a file is «traccia» («Seleziona traccia audio»). «Siti di streaming» for streaming sites. |
| device / capture device | dispositivo / dispositivo di acquisizione | «webcam» (feminine), «scheda di acquisizione». |
| screen capture | acquisizione schermo | The desktop as a video source; a still picture is a «screenshot». |
| disc | disco | The drive is «unità» («unità DVD», «unità Blu-ray»). |
| title | titolo | A DVD's title («Menu titoli»); also a bookmark's or entry's title. |
| angle | angolazione | «Menu angolazioni». |
| disc menu | menu del disco | |
| drag and drop | trascinamento | Verb «trascina e rilascia»; «file trascinati». |
| recent files | file recenti | «Elementi recenti», «URL recenti». |
| favorites | preferiti | Masculine plural: «Aggiungi ai preferiti». |
| file association / file type | associazione file / tipo di file | The page «Associazioni»; extensions «estensioni». |
| playlist | playlist | Feminine, invariable: «la playlist», «le playlist». |
| album | album | Masculine, invariable: «l'album», «gli album». |
| entry | elemento | Also for "item". An item of a menu is a «voce» («Voci del menu contestuale»). |
| sort | ordina | Noun «ordinamento»; «Ordina per». |
| add / remove / delete | aggiungi / rimuovi / elimina | «Rimuovi» takes entries out of a list; «Elimina» deletes the file itself (or a saved item such as a bookmark). |
| recycle bin / trash | Cestino / Cestino | The place's name, capitalized, on Windows and Linux: «Sposta nel Cestino». |
| import / export | importa / esporta | |
| window / main window | finestra / finestra principale | |
| fullscreen / windowed | schermo intero / in finestra | «A schermo intero», «Modalità schermo intero». |
| maximize / minimize | ingrandisci / riduci a icona | Restore is «Ripristina». «Ingrandisci» is never zoom («Aumenta zoom»). |
| always on top / stay on top | sempre in primo piano / in primo piano | |
| opacity | opacità | Transparency is «trasparenza». |
| skin | skin | Feminine, invariable: «la skin», «le skin», «modalità skin». |
| color theme | tema di colore | Plural «temi di colore». |
| logo | logo | Plural «loghi». |
| title bar | barra del titolo | |
| control panel | pannello di controllo | RVP's F7 window; where it could be taken for Windows' Control Panel, «pannello di controllo di RVP». |
| OSD / on-screen display | OSD / messaggi a schermo | «l'OSD» (masculine); «Messaggi OSD»; in prose «messaggi a schermo». |
| on-screen controller | comandi a schermo | mpv's OSC: «Comandi a schermo di mpv»; the setting "Controller" alone is «Comandi». PotPlayer's "On Screen Control" page (the skin's bars over the video) is «Controlli sul video». |
| tooltip | suggerimento | Plural «suggerimenti». |
| context menu | menu contestuale | «menu» has no accent and is invariable. |
| tray icon / notification area | icona nell'area di notifica / area di notifica | "Tray" in short labels is also «area di notifica». |
| taskbar | barra delle applicazioni | Linux's dock is the «dock». |
| notification | notifica | |
| dialog | finestra di dialogo | Plural «finestre di dialogo»; «finestra» alone when clear. |
| settings / preferences | impostazioni / preferenze | The F5 window is «Impostazioni»; the menu item "Preferences..." is «Preferenze...»; "options" are «opzioni». |
| page | pagina | |
| preset / configuration | preset / configurazione | «il preset», «i preset» (masculine, invariable); «Preset %1». |
| reset / default | reimposta / predefinito | «Reimposta» puts values back; «Ripristina» is restore. «predefinito» agrees: «valori predefiniti», «playlist predefinita»; the button "Default" is «Predefinito»; "Windows' default" «Predefinito di Windows». |
| apply | applica | |
| key / hotkey / shortcut | tasto / tasto rapido / scorciatoia | Key combinations keep RVP's key names («Ctrl+Left»). |
| mouse / wheel / button | mouse / rotellina / pulsante | «pulsante» is also a button on screen; «clic sinistro». |
| gesture | gesto | «gesto del mouse», plural «gesti». |
| touch | tocco | The screen is a «touchscreen». |
| controller / gamepad | controller / gamepad | Masculine: «controller Xbox». |
| video / picture / image | video / immagine / immagine | «il video», «i video» (masculine, invariable). |
| frame | fotogramma | Plural «fotogrammi»; never «frame» except in names. |
| frame rate / fps | frequenza dei fotogrammi / fps | In prose «fotogrammi al secondo». |
| aspect ratio | proporzioni | Feminine plural: «Mantieni le proporzioni»; PotPlayer's AR and DAR are «proporzioni» too. |
| crop | ritaglia | Noun «ritaglio». |
| zoom / pan | zoom / sposta | «Aumenta zoom», «Riduci zoom»; moving the picture «Sposta a sinistra». A camera pan inside the video is a «panoramica». |
| rotate / flip / mirror | ruota / capovolgi / specchia | Noun «rotazione». Flip Horizontal and Flip Vertical are «Capovolgi orizzontalmente» and «Capovolgi verticalmente». |
| deinterlace | deinterlaccia | Noun «deinterlacciamento». |
| brightness / contrast / saturation / hue / gamma | luminosità / contrasto / saturazione / tonalità / gamma | «tonalità» is hue only; pitch is «tono». Luminance is «luminanza». |
| sharpen / denoise | nitidezza / riduzione rumore | Filter names «Nitidezza», «Riduzione rumore 3D». |
| scaler / upscaling / resize | filtro di scala / ingrandimento / ridimensiona | Upscaler «Ingrandimento», downscaler «Riduzione», chroma scaler «Scala crominanza»; noun «ridimensionamento». «Ingrandimento» is not «Ingrandisci» (maximize). |
| pixel shader | pixel shader | Masculine, invariable: «i pixel shader», «gli shader». |
| renderer | renderer | Masculine: «il renderer video»; PotPlayer's "Audio Renderer" (the sound's output) is «Renderer audio». |
| hardware decoding / decoder | decodifica hardware / decoder | «decodifica software»; «decoder» masculine, invariable. |
| HDR | HDR | «l'HDR» (masculine); «video HDR», «schermo HDR». |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / video a 360° / anaglifo / fianco a fianco / sopra e sotto | Anaglyph colours «rosso/ciano», «verde/magenta», «giallo/blu»; «grigio», «a colori», «a mezzi colori», «Dubois». |
| bars / letterbox / margin | bande / letterbox / margine | Black bars «bande nere»; the skin's bars of controls are «barre». |
| frame capture / screenshot | acquisizione fotogramma / screenshot | «lo screenshot», «gli screenshot». |
| motion smoothing | movimento fluido | The feature's name: «Movimento fluido»; a smoothed frame is a «fotogramma interpolato». |
| frame interpolation | interpolazione dei fotogrammi | |
| engine | motore | Masculine: «il motore MVTools». |
| calibration | calibrazione | Verb «calibra»; «Ricalibra…». |
| quality preset (light, medium, high) | preset di qualità (leggero, medio, alto) | RVP's presets Fast, Medium, Quality are «Veloce», «Medio», «Qualità» (masculine, agreeing with preset). |
| GPU / graphics card | GPU / scheda video | «la GPU» (feminine); "card" alone is «scheda». |
| processor / CPU | processore / CPU | «la CPU» (feminine). |
| power budget | budget di potenza | «potenza» is power in watts; never «energia» or «alimentazione». |
| dropped frames | fotogrammi persi | |
| refresh rate | frequenza di aggiornamento | The display's rate is «la frequenza dello schermo». |
| audio / sound | audio / audio | Italian UI says «audio» for both; «suono» only for a sound as heard (a tone, an effect). |
| volume / mute | volume / disattiva audio | The state is «audio disattivato». |
| master volume | volume principale | PotPlayer's "Wave" volume is «volume wave». |
| equalizer | equalizzatore | |
| normalizer | normalizzatore | |
| reverb | riverbero | |
| noise reduction | riduzione del rumore | The audio page and filter; «riduzione rumore» in short labels. |
| channel / speaker | canale / altoparlante | «canale sinistro», «altoparlanti virtuali». |
| downmix | downmix | Masculine; the mixer is «il mixer». |
| surround / stereo | surround / stereo | In the mixer, "Surround" are the rear speakers: «Surround». |
| output device | dispositivo di uscita | |
| sample rate / bitrate | frequenza di campionamento / bitrate | «il bitrate» (masculine). |
| audio delay / audio sync | ritardo audio / sincronia audio | «sincronia» for every sync (audio, subtitles, the second device); verb «sincronizza». |
| pitch / time stretching | tono / time stretching | «tono», never «tonalità» (hue). «il time stretching» (masculine). |
| audio recording | registrazione audio | |
| second audio device | secondo dispositivo audio | |
| pass-through | pass-through | Masculine; sending the encoded sound unchanged to a receiver. |
| subtitles / subtitle track | sottotitoli / traccia sottotitoli | One line is a «sottotitolo». |
| external / embedded subtitles | esterni / sottotitoli incorporati | Agrees: «sottotitoli esterni», «audio esterno». |
| font / outline / shadow | font / contorno / ombra | «il font» (masculine, invariable); bold «grassetto», italic «corsivo». |
| subtitle position / subtitle delay | posizione sottotitoli / ritardo sottotitoli | |
| encoding | codifica | Text: «codifica dei caratteri»; re-encoding «ricodifica». |
| subtitle browser / subtitle editor | esplora sottotitoli / editor sottotitoli | «l'editor» (masculine). |
| subtitles from audio / speech recognition | sottotitoli dall'audio / riconoscimento vocale | |
| model | modello | |
| language | lingua | Never «linguaggio». |
| remote control | controllo remoto | A remote (the app or device) is a «telecomando». |
| token | token | Masculine. |
| port / server | porta / server | |
| log | registro | The page and the file; "logged" «registrato». |
| crash | arresto anomalo | |
| download / install | scarica / installa | The noun is «il download». |
| installer | programma di installazione | |
| accessibility | accessibilità | |
| text to speech (TTS) / screen reader | sintesi vocale (TTS) / lettore di schermo | |
| screen saver | salvaschermo | |
| shut down / sleep / hibernate | arresta / sospendi / iberna | Nouns «arresto», «sospensione», «ibernazione». |
| timer | timer | |
| visualization / spectrum | visualizzazione / spettro | |
| status bar / info | barra di stato / info | «info» or «informazioni». |
| codec / format | codec / formato | |
| bit depth / 10-bit | profondità di bit / 10 bit | No hyphen: «%1 bit». |
| channel layout (5.1, 7.1) | configurazione dei canali (5.1, 7.1) | |
| Auto select (recommended) | Selezione automatica (consigliata) | "Auto select" alone is «Selezione automatica». |
