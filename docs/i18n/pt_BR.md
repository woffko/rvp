# Portuguese, Brazil (pt_BR)

RVP's Portuguese is Brazilian Portuguese in the spelling of the 1990
Orthographic Agreement (*ideia*, *frequência*, *autoajuste*), in the
neutral written register of Brazilian desktop software: plain, direct
and never colloquial. The words are the ones a Brazilian user knows from
Windows, GNOME, KDE and the common media players (*arquivo*, *tela*,
*mouse*, *salvar*, *excluir*, *baixar*), but every string is written for
RVP from its meaning, not from another program's translation. Users in
Portugal read this file's language until a European Portuguese exists,
so slang and spoken forms are left out; the vocabulary stays Brazilian.

## Style

- **Address.** The user is *você*, implied by the verb and rarely
  written; never *tu* or *o senhor*. Instructions are in the imperative
  of *você* (*Selecione uma ação e adicione uma tecla.*, *Pressione uma
  tecla para “%1” (%2). Esc cancela.*). No *por favor*, no *Você pode*:
  say what to do or what happens. Write *seu*/*sua* only where the
  source has "your" and the sentence needs it.
- **Commands** (menu items, buttons, tray items, context menus) are
  infinitives: *Abrir pasta...*, *Salvar*, *Reproduzir*, *Pausar*,
  *Excluir arquivos*. PotPlayer's *Cycle X* is *Alternar X*, *Increase*
  / *Decrease* are *Aumentar* / *Diminuir*.
- **Checkboxes and options** are infinitive phrases too: *Lembrar
  posições de reprodução de vídeos*, *Ocultar o ponteiro do mouse sobre o
  vídeo*, *Mostrar dicas*. *Enable* / *Disable* are *Ativar* / *Desativar*,
  *Show* / *Hide* *Mostrar* / *Ocultar*, *Display* *Exibir*, *Use* *Usar*,
  *Remember* *Lembrar*. PotPlayer's *Auto X* is the verb with
  *automaticamente* (*Ocultar automaticamente*), or the noun with
  *automático* where space is tight.
- **Choices** in a list are nouns, adjectives or short phrases that
  agree with the setting they answer: *Decodificação por hardware*:
  *Desligada (software)*; *Prioridade*: *Abaixo do normal*. On / Off as
  values are *Ligado* / *Desligado*; states are *Ativado* / *Desativado*;
  none is *Nenhum* / *Nenhuma*. With no noun to agree with, use the
  masculine.
- **Titles** of windows, pages, menus and groups are noun phrases:
  *Repetição de trecho*, *Galeria de cenas*, *Configurações de toque*.
- **Help texts** are whole sentences in the present tense, with RVP or
  the feature as the subject: *O RVP mede quantos quadros por segundo...*.
  Progress is in the gerund (*Medindo %1…*, *Baixando*, *Verificando*);
  results are past participles (*Aplicado.*, *Importado.*, *Cancelado.*).
- **Questions** before an action ask with the infinitive: *Enviar %1
  arquivo(s) para a Lixeira?*, *Mover para “%3”?*; *Deseja...* only where
  the infinitive reads badly. Repeat the noun rather than attach a
  pronoun to the verb (*movê-la*).
- **Capitalization** is sentence case everywhere, also where PotPlayer's
  English uses Title Case: *Abrir arquivo(s)...*, *Configurações de
  reprodução*, *Proporção personalizada*. Capitals stay on names,
  acronyms and keys (RVP, SVP, OSD, HDR, Ctrl), on *Lixeira* as the
  place, and on the first letter of a label quoted in a sentence.
  Language names, months and days are lowercase (*inglês*, *russo*), so
  whisper's language list stays lowercase as in the source; the
  language's own name is *Português (Brasil)*. After a colon the text
  goes on in lowercase unless a name follows (*Crítico: erros*,
  *Desligado: o RVP não lê a potência da placa.*).
- **Names of controls in a text** are their Portuguese labels, written
  as they show: a menu path as *Vídeo > Margem inferior*, a page by its
  title (*na página Teclas e mouse*), a longer label between quotation
  marks (*a ação de “Ao terminar a reprodução”*).
- **Names and articles.** Names stay as README lists them, and take the
  article of the thing they are: *o RVP*, *o mpv*, *o PotPlayer*, *o SVP*,
  *o FFmpeg*, *o VapourSynth*, *o Windows*, *o Linux*, *o GNOME*, *o
  whisper.cpp* (programs); *a libmpv* (a library); *a NVIDIA*, *a AMD*
  (companies); *o DVD*, *o Blu-ray*; *a skin RVP Default*. The article
  contracts as Portuguese requires: *do RVP*, *no mpv*, *ao PotPlayer*,
  *pelo SVP*. English possessives become *de*: *PotPlayer's skins* =
  *skins do PotPlayer*, *Windows' default* = *padrão do Windows*.
- **Crase.** *A* + a feminine article is *à*: *Adicionar à playlist*,
  *à fila*, *à direita*; but *para a* stays: *Enviar para a Lixeira*.
- **Quotation marks** are “ ” for the source's straight and curly double
  quotes alike, and ‘ ’ inside them. No space before *:* *;* *?* *!*. No
  comma before *e* in a list (*marcadores, capítulos e pontos A-B*).
  *&* is *e*. A slash between alternatives stays without spaces, as in
  the source (*Reproduzir/pausar*).
- **"..."** Keep exactly what the source ends with: three periods where
  it has "...", the single character "…" where it has that, with no
  space before them. Do not add one where the source has none.
- **Units.** Seconds are *s*, minutes *min*, hours *h* (*Sec*, *Sec.* and
  *Min* in PotPlayer's labels become *s* and *min*); *ms*, *dB*, *Hz*,
  *kHz*, *fps*, *px*, *K*, *W*, *MB*, *GB* stay. A unit symbol takes no
  period and stands after a space: *%1 s*, *%1 ms*, *%1 Hz* (also where
  the source writes *%1ms*). *%* keeps the source's spacing. Resolutions
  stay *480p*, *2160p*; speeds *%1x*. *Pixel* is *pixel*, plural
  *pixels*; *%1-bit* is *%1 bits*; *channels* in *%1 chs* is *canais*.
- **Numbers** written in the text use the Brazilian decimal comma and
  thousands period (*0,5*, *1,0x*, *1.000*). Unchanged: version numbers,
  addresses (*127.0.0.1*), channel layouts (*5.1*, *7.1*), ratios
  (*16:9*), and formats the user types into a field (*h:mm:ss.s*,
  *hh:mm*, *h:mm*: the letters are Portuguese already). Numbers in
  placeholders come formatted from RVP; add nothing around them.
- **Ordinals** use the indicator: *%1º dispositivo* (*%1nd Device*),
  *Igual ao %1º logo*; *ª* after a feminine noun.
- **Placeholders and agreement.** A placeholder's value has no gender
  Portuguese can see, so put a name or a number after a colon or in
  quotation marks rather than make an article or adjective agree with
  it: *Removido: %1*, *Status: %1*, *Predefinição %1*. Counts keep the
  source's *(s)*: *%1 arquivo(s)*, *%1 item(ns)*. Units and words that
  follow a placeholder keep their place after it (*%1 de %2 GB*).
- **Keys** keep the names RVP shows in its menus, as in the source:
  Ctrl, Shift, Alt, Enter, Space, Esc, Tab, PgUp, PgDn, Home, End, Left,
  Right, Up, Down, F1...F12 and the letters (*Ctrl+Left / Right*). A
  description of keys is translated: *as setas*, *as teclas de mídia*,
  *a tecla Voltar do teclado*. Mouse buttons X1 and X2 and the
  controller's A, B, X, Y, Back and Start keep their names.
- **Examples in empty fields** (placeholders shown in grey) follow a
  Brazilian user: language codes *por,eng*, *por* and *pt,en*, *en*;
  chapter titles stay *Opening;Ending;3*, since files name their
  chapters so. Paths, file names, settings keys, mpv's and yt-dlp's
  values stay as in the source. A standard folder described in words
  takes the name the desktop shows (*Imagens/RVP na pasta pessoal*,
  *Vídeos/RVP*, *Música/RVP*).
- **Brazilian words, not European ones:** *arquivo* (not *ficheiro*),
  *tela* (*ecrã*), *mouse* (*rato*), *salvar* (*guardar*), *excluir*
  (*eliminar*), *baixar* (*transferir*), *usuário* (*utilizador*),
  *controle* (*controlo*, *comando*), *registro* (*registo*),
  *decodificar* (*descodificar*), *tela cheia* (*ecrã inteiro*), the
  gerund for what goes on (*baixando*, not *a baixar*). *Programa* is a
  desktop program; *aplicativo* only where Windows names its own pages
  (*Aplicativos padrão*). A display (the device) is a *monitor*, the
  screen a *tela*; the desktop as a place is the *área de trabalho*,
  GNOME or KDE an *ambiente gráfico*. A step of an amount is an
  *incremento* (*Incremento de volume*), a step of the GPU budget's or
  the smoothing's ladder a *nível*.
- **Everyday words**, the same in every context: OK, *Cancelar*,
  *Aplicar*, *Fechar*, *Sair* (Exit), *Abrir*, *Salvar*, *Salvar como…*,
  *Procurar…* (Browse), *Pesquisar* (Find, Search), *Padrão*,
  *Personalizado*, *Automático*, *Sempre*, *Nunca*, *Copiar*, *Colar*,
  *Desfazer*, *Refazer*, *Selecionar tudo*, *Editar*, *Renomear*,
  *Limpar*, *Restaurar*, *Subir* / *Descer* (Up / Down buttons of a
  list), *Diversos* (Misc), *Outro* (Other).
- **Short labels.** Portuguese runs longer than English, so labels drop
  what Portuguese can spare: no articles where the label reads well
  without (*Abrir pasta*, not *Abrir a pasta*), no *Clique para*, the
  short word of two (*Usar*, not *Utilizar*; *Ativar*, not *Habilitar*;
  *Mostrar*, not *Visualizar*; *sincronia*, not *sincronização*), and *Configurar X...* for
  PotPlayer's *X Settings...* menu items. Abbreviate only where the source does and the
  control is tight, with a period (*máx.*, *mín.*). Help texts may run
  as long as Portuguese needs.
- **Names of effects, filters, presets and models** that are names stay
  (*FSR*, *NVScaler*, *KrigBilateral*, *adaptive-sharpen*, *Blend*,
  *SuperEQ*, *tiny*, *base.en*); their descriptions are translated
  (*Blend (mistura de quadros)*, *tiny (não baixado)*). Equalizer
  presets named after genres keep the genre's name Brazilians use
  (*Pop*, *Rock*, *Reggae*, *Ska*, *Techno*, *Dance*) and translate the
  rest (*Clássica*, *Festa*, *Ao vivo*, *Graves*).

## Glossary

| English | Portuguese (Brazil) | Note |
|---------|---------------------|------|
| play / pause / stop | reproduzir / pausar / parar | Buttons and commands in the infinitive; the noun of pause is *pausa*. |
| playback | reprodução |  |
| close playback | fechar reprodução | Stops and unloads the file: never *parar* (stop). |
| previous / next | anterior / próximo | *Próximo* agrees with its noun: *próxima legenda*, *próximo capítulo*. |
| seek | navegar | Noun *navegação*. Commands that move by an amount use *avançar* / *voltar* (*Avançar %1 s*). Not *buscar* or *procurar*, which read as searching. |
| jump | saltar | Noun *salto* (*Saltos exatos*); *Jump to...* is *Saltar para...*. Distinct from *pular* (skip). |
| frame step | quadro a quadro | *Próximo quadro*, *Quadro anterior* for the commands. |
| speed | velocidade |  |
| repeat | repetir | Noun *repetição*. |
| A-B repeat / section repeat | repetição A-B / repetição de trecho | The marks are *ponto A* and *ponto B*; a marked section is a *trecho marcado*. |
| shuffle | aleatório | *Reprodução aleatória*; agrees with its noun (*ordem aleatória*). |
| queue | fila | *Fila de reprodução*; *Adicionar à fila* (with crase). |
| resume / remember position | retomar / lembrar posição | The Playback page's heading *Resume* is the noun *Retomada*. |
| position | posição |  |
| duration | duração |  |
| elapsed / remaining time | decorrido / tempo restante | *Tempo decorrido* in full where the noun is needed; *Decorrido e restante* as a choice. |
| chapter | capítulo |  |
| bookmark | marcador | Never *favorito* (that is favorites). A mark drawn on the seek bar (marker) is a *marca*. |
| skip (intro, ending) | pular (abertura, encerramento) | *Pular abertura* is the phrase users know. Because *abertura* is the intro, the Playback page's *Opening* section (opening files) is *Abrir arquivos*. |
| seek bar | barra de navegação | PotPlayer's *nav bar* and *navigation bar* are the same bar and the same term. Not *barra de busca* (a search box). |
| thumbnail | miniatura |  |
| preview | prévia | *Prévias em miniatura* on the seek bar; the screen saver's Preview button is *Prévia* too. |
| scene browser | galeria de cenas | *Browse Scenes by...* is *Ver cenas por...*. Not *navegador* (a web browser). |
| file | arquivo | Never *ficheiro* (Portugal). |
| folder | pasta |  |
| URL / address | URL / endereço | Feminine: *a URL*, plural *URLs*. |
| stream | faixa | A track inside a file is a *faixa* (*Selecionar faixa de áudio*, *Preferir a faixa indicada*). Recording or receiving media as it comes is a *fluxo* (*Gravar fluxo...*); streaming sites are *sites de streaming*. |
| device / capture device | dispositivo / dispositivo de captura | *Webcam* stays. |
| screen capture | captura de tela | The desktop as a video source (*Abrir captura de tela*). *Tela*, never *ecrã*. |
| disc | disco |  |
| title | título | A DVD's title; *Title Menu* is *Menu de títulos*. |
| angle | ângulo | *Ângulo de câmera*. |
| disc menu | menu do disco |  |
| drag and drop | arrastar e soltar |  |
| recent files | arquivos recentes | *Itens recentes* where the source says items. |
| favorites | favoritos |  |
| file association / file type | associação de arquivos / tipo de arquivo |  |
| playlist | playlist | Feminine: *a playlist*, *à playlist*, *na playlist*. The everyday Brazilian word; *lista de reprodução* is too long for the menus and is not used. |
| album | álbum | Masculine; plural *álbuns*. |
| entry | item | Masculine; plural *itens* (no accent). PotPlayer's labels say item for the same thing. |
| sort | ordenar | *Ordenar por*. |
| add / remove / delete | adicionar / remover / excluir | *Remover* takes an entry out of the playlist; *excluir* deletes the file itself. *Clear* is *limpar*. |
| recycle bin / trash | Lixeira / lixeira | Windows' Recycle Bin and the Linux desktop's trash are both the *Lixeira*, capitalized when it names the place (*Enviar para a Lixeira*). |
| import / export | importar / exportar |  |
| window / main window | janela / janela principal |  |
| fullscreen / windowed | tela cheia / em janela | *Em tela cheia* inside a sentence. |
| maximize / minimize | maximizar / minimizar |  |
| always on top / stay on top | sempre no topo / manter no topo | Above the other windows; *Alternar sempre no topo* for the cycle. |
| opacity | opacidade | Transparency is *transparência*. |
| skin | skin | Feminine: *a skin*, plural *skins*; *RVP Default* stays as a name (*a skin RVP Default*). |
| color theme | tema de cores |  |
| logo | logo | Masculine: *o logo*. |
| title bar | barra de título |  |
| control panel | painel de controle | RVP's F7 window, not Windows' Control Panel. *Controle*, never *controlo*. |
| OSD / on-screen display | OSD / exibição na tela | Masculine: *o OSD*; keep *OSD* where the source has it (*Mensagens do OSD*). |
| on-screen controller | controlador na tela | mpv's OSC and scripts such as uosc. PotPlayer's *On Screen Control* page (the skin's bars over the video) is *Controles na tela*. |
| tooltip | dica |  |
| context menu | menu de contexto |  |
| tray icon / notification area | ícone na bandeja / área de notificação | *Bandeja* alone in tight labels (*Barra de tarefas e bandeja*). |
| taskbar | barra de tarefas | On Linux the dock is the *dock*. |
| notification | notificação |  |
| dialog | caixa de diálogo | *Diálogo* alone only where space is tight. |
| settings / preferences | configurações / preferências | The window and its headings say *Configurações*; a menu item *X Settings...* may read *Configurar X...* to stay short. |
| page | página |  |
| preset / configuration | predefinição / perfil | A feature's preset (equalizer, frame size, quality, VapourSynth) is a *predefinição*. A saved set of all settings is a *perfil*: the Configuration page (*Perfis*), its presets, the menu's *Default preset* (*Perfil padrão*, keeping its leading asterisk), *Manage Presets...*, *Auto-Preset* (*Perfil automático*). *Configurações* (plural) stays the settings window. |
| reset / default | redefinir / padrão | *Padrão* does not agree after a noun: *valores padrão*, *(padrão)*. *Restore to Default* is *Restaurar padrão*. |
| apply | aplicar |  |
| key / hotkey / shortcut | tecla / tecla de atalho / atalho |  |
| mouse / wheel / button | mouse / roda / botão | *Mouse* is masculine, never *rato*; *roda do mouse*. |
| gesture | gesto |  |
| touch | toque | A touchscreen is a *tela sensível ao toque*. |
| controller / gamepad | controle / gamepad | *Controle do Xbox*; never *comando* (Portugal). Its buttons A, B, X, Y, Back and Start keep their names. |
| video / picture / image | vídeo / imagem / imagem | The video's picture and a still image are both *imagem*. |
| frame | quadro | Never *fotograma* (Portugal). PotPlayer's *Frame Size* is *Tamanho do quadro*. |
| frame rate / fps | taxa de quadros / fps | *Quadros por segundo* in prose (*frames a second*). |
| aspect ratio | proporção | PotPlayer's abbreviation AR is written out as *proporção*; DAR stays. |
| crop | recortar | Noun *recorte*. *Cortar* reads as cutting a part of the time. |
| zoom / pan | zoom / deslocar | *Aumentar zoom* / *Diminuir zoom* for Zoom In / Zoom Out. |
| rotate / flip / mirror | girar / inverter / espelhar | Noun *rotação*. *Flip Vertical* is *Inverter verticalmente*. |
| deinterlace | desentrelaçar | Noun *desentrelaçamento*. |
| brightness / contrast / saturation / hue / gamma | brilho / contraste / saturação / matiz / gama | *Matiz* is masculine (*o matiz*). |
| sharpen / denoise | nitidez / remoção de ruído | Commands *aumentar a nitidez*, *remover ruído*; the sound's noise reduction is *redução de ruído*. |
| scaler / upscaling / resize | filtro de escala / ampliação / redimensionar | Upscaler / Downscaler / Chroma scaler: *Filtro de ampliação* / *Filtro de redução* / *Filtro de escala do croma*. Not *escalonador* (a scheduler). Noun *redimensionamento*. |
| pixel shader | pixel shader | Masculine: *o pixel shader*, plural *pixel shaders*; *shader* alone where the source says shader. |
| renderer | renderizador | PotPlayer's *Audio Renderer* (where the sound plays) is *Saída de áudio*. |
| hardware decoding / decoder | decodificação por hardware / decodificador | Brazilian *decodificar*, never *descodificar*; *decodificação por software*. |
| HDR | HDR |  |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / vídeo 360° / anáglifo / lado a lado / em cima e embaixo | SBS and TAB stay; *embaixo* is one word. |
| bars / letterbox / margin | barras / letterbox / margem | Black bars are *barras pretas*; the skin's control bars are *barras* too. |
| frame capture / screenshot | captura de quadro / imagem da tela | *Save Current Screen Frame* is *Salvar quadro da tela*. *Captura de tela* is kept for screen capture. |
| motion smoothing | suavização de movimento | *Smooth motion* is *Suavizar movimento*. |
| frame interpolation | interpolação de quadros |  |
| engine | mecanismo | For the smoothing, whisper and TTS engines alike. |
| calibration | calibração | Verb *calibrar*; *Recalibrar…*. |
| quality preset (light, medium, high) | predefinição de qualidade (leve, média, alta) | Feminine, after *predefinição*: *Rápida* / *Média* / *Qualidade* for Fast / Medium / Quality. |
| GPU / graphics card | GPU / placa de vídeo | Feminine: *a GPU*. Never *placa gráfica* (Portugal). |
| processor / CPU | processador / CPU | Feminine: *a CPU*. |
| power budget | limite de energia | The share of the card's power RVP may use. |
| dropped frames | quadros perdidos |  |
| refresh rate | taxa de atualização |  |
| audio / sound | áudio / som |  |
| volume / mute | volume / mudo | The command *Mute* is *Mudo*; the verb is *silenciar*. |
| master volume | volume geral | The system's own volume; *Mute (Master)* is *Mudo (geral)*. |
| equalizer | equalizador |  |
| normalizer | normalizador |  |
| reverb | reverberação |  |
| noise reduction | redução de ruído |  |
| channel / speaker | canal / alto-falante | Plural *alto-falantes*; never *altifalante* (Portugal). |
| downmix | downmix | Masculine; *mixar para menos canais* where a verb reads better. |
| surround / stereo | surround / estéreo |  |
| output device | dispositivo de saída |  |
| sample rate / bitrate | taxa de amostragem / taxa de bits |  |
| audio delay / audio sync | atraso do áudio / sincronia do áudio | Verb *sincronizar*; *sincronia* is shorter than *sincronização* and means the same. |
| pitch / time stretching | tom / alongamento de tempo | *Mudança de tom* for pitch shifting. |
| audio recording | gravação de áudio |  |
| second audio device | segundo dispositivo de áudio | *2º dispositivo* in tight labels. |
| pass-through | passthrough | One word, as receivers and TVs sold in Brazil write it. |
| subtitles / subtitle track | legendas / faixa de legenda | One line is a *legenda*. |
| external / embedded subtitles | externas / legendas embutidas | *Legendas externas* in full where the noun is needed. |
| font / outline / shadow | fonte / contorno / sombra |  |
| subtitle position / subtitle delay | posição da legenda / atraso da legenda |  |
| encoding | codificação | *Codificação do texto*. |
| subtitle browser / subtitle editor | lista de legendas / editor de legendas | *Browse/Edit…* is *Ver/editar…*. |
| subtitles from audio / speech recognition | legendas do áudio / reconhecimento de fala | *Criar legendas a partir do áudio* for the commands. |
| model | modelo | whisper's model names (tiny, base.en...) stay. |
| language | idioma | Language names are lowercase: *inglês*, *russo*. |
| remote control | controle remoto |  |
| token | token | Masculine. |
| port / server | porta / servidor |  |
| log | log | Masculine. Not *registro*, which is Windows' Registry. |
| crash | falha | *Travar* is to hang, not to crash. |
| download / install | baixar / instalar | The noun stays *download* (*um download de %1 GB*). |
| installer | instalador |  |
| accessibility | acessibilidade |  |
| text to speech (TTS) / screen reader | conversão de texto em fala (TTS) / leitor de tela | The page title TTS may stay *TTS*. |
| screen saver | proteção de tela |  |
| shut down / sleep / hibernate | desligar / suspender / hibernar |  |
| timer | temporizador |  |
| visualization / spectrum | visualização / espectro |  |
| status bar / info | barra de status / informações |  |
| codec / format | codec / formato | *Codec* is masculine, plural *codecs*. |
| bit depth / 10-bit | profundidade de bits / 10 bits | *%1-bit* is *%1 bits*. |
| channel layout (5.1, 7.1) | disposição dos canais (5.1, 7.1) | 5.1 and 7.1 keep their period. |
| Auto select (recommended) | Seleção automática (recomendada) | *Recomendado* agrees with the choice's noun. |
