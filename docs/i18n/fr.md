# French (fr)

RVP's French is the French of France, written so that it reads naturally in
Belgium, Switzerland and Canada too: standard vocabulary, no regional or
colloquial words, the vocabulary French-speaking users know from Windows,
GNOME, KDE and media players. Anglicisms are avoided where a usual French
word exists (`habillage`, `jeton`, `plantage`), and kept where French users
write the English word themselves (`codec`, `pixel shader`, `surround`,
`OSD`, `GPU`). The tone is plain and neutral, as in a desktop program's own
settings; the Russian translation helps with meaning, never with wording.

## Style

- **Form of address:** `vous`, never `tu`. Instructions in help texts are
  in the imperative with `vous` (`Appuyez sur une touche pour « %1 ».`,
  `Faites glisser la vidéo avec la souris pour regarder tout autour.`); no
  `Veuillez`, no `Cliquez ici pour`. Descriptions are in the present
  indicative with the program as subject (`RVP mesure…`, `mpv dessine…`).
  Negation always keeps `ne` (`RVP ne télécharge rien`).
- **Labels.** Commands (menu items, buttons) are infinitives: `Ouvrir un
  dossier...`, `Enregistrer sous…`, `Lire l'élément suivant`, `Fermer`.
  Checkboxes are infinitive phrases (`Mémoriser les positions de lecture`,
  `Ne pas ajouter les fichiers en double`) or noun phrases where the
  English is one. Menu names, page titles and section headings are nouns
  (`Lecture`, `Sous-titres`, `Répétition de passage`). Choices in a list
  are nouns, adverbs or adjectives: `Jamais`, `Toujours`, `En haut à
  gauche`, `Désactivé`; prefer wordings that need no agreement, and an
  adjective agrees with the noun of its label (`Position : Centrée`).
  States: `Activé` / `Désactivé` (also for On / Off,
  Enabled / Disabled). Questions in confirmations are infinitive questions:
  `Mettre %1 fichier(s) à la corbeille ?`. Status texts are past
  participles in the default masculine: `Appliqué.`, `Importé.`, `Annulé.`
- **Usual verbs:** Enable `Activer`, Disable `Désactiver`, Use `Utiliser`,
  Show `Afficher`, Hide `Masquer`, Remember `Mémoriser`, Open `Ouvrir`,
  Load `Charger`, Save `Enregistrer` (never `sauvegarder`, which French
  desktops keep for backups), Record `Enregistrer` (the object tells them
  apart: `Enregistrer l'audio...`), Browse `Parcourir`, Choose `Choisir`,
  Select `Sélectionner` (a track, a stream: `Choisir`), Undo `Annuler`,
  Redo `Rétablir`, Cancel `Annuler`, Close `Fermer`, Exit `Quitter`,
  Restore (a window) `Restaurer`, Restore defaults `Rétablir`, Reload
  `Recharger`, Rescan `Actualiser`, Prompt `Demander`, Prevent `Empêcher`,
  Increase / Decrease `Augmenter` / `Diminuer` (never `Réduire`, which is
  Windows' Minimize), Move Up / Down `Monter` / `Descendre`, Setup
  (a noun) `Réglages`, Set up `Configurer`. "Auto X" becomes `X
  automatiquement` in checkboxes and sentences, `X auto` in short labels
  (`Sélection auto`).
- **Capitalization:** sentence case everywhere, whatever the English
  Title Case: only the first word and names take a capital (`Paramètres de
  lecture...`, `Ouvrir des fichiers...`, `Taille de la fenêtre`). After a
  slash and after a colon the next word is lowercase unless it is a name
  (`Afficher/masquer les sous-titres`, `Chapitre/signet`, `Adoucir :
  rayon`, `Critique : erreurs`). Capitals keep their accents (`À propos de
  RVP`, `État`, `Égaliseur`). Language names are lowercase in French
  (`français`, `anglais`), and the whisper language list keeps the
  source's lowercase. Lowercase source words that the program puts into
  other texts (`never`, `none`, `not set`, `checking`, `downloading`)
  stay lowercase.
- **Spaces before punctuation:** French puts a space before `:`, `;`,
  `?` and `!`, and inside guillemets. In RVP's files this is always the
  no-break space U+00A0 (so a line never begins with the mark), never a
  plain space, never an entity such as `&nbsp;`; one character for all of
  them, not the narrow U+202F, which some interface fonts lack. Examples
  (· marks U+00A0): `Nom de l'album·:`, `Démarrer·;`, `Déplacer vers
  « %3 »·?`, `«·%1·»`. No space after an opening or before a closing
  parenthesis, before `,` or `.`, or around `/`. A colon that is not
  punctuation keeps no space: times `h:mm:ss`, ratios `16:9`, `%1:%2`.
  Elsewhere in this file the examples show that space as a plain one;
  in the translations it is U+00A0, as is the space between a number
  and its unit.
- **Quotation marks:** guillemets `« »` with U+00A0 inside, for every
  quote in the source (`“%1”`, `"Add to"`). A help text that names
  another control, page or menu quotes its French label: `dans le menu
  « Proportions »`, `« Renouveler » crée un nouveau jeton`. A settings
  path keeps `>` without quotes: `(Vidéo > Marge inférieure)`.
- **Apostrophe:** the straight `'` (U+0027), as in the source: users type
  it in the settings search (`Rechercher une option`), and `l'image`
  must match what they type.
- **Ellipsis:** keep exactly what the source has, three periods `...` or
  the single character `…`, attached to the last word: `Préférences...`,
  `Calibrer…`. `Shutdown at ...` keeps its space: `Arrêt à ...`.
- **"&"** becomes `et` (`Basses et aigus`, `Ajouter et modifier`).
- **Units:** a no-break space between a number and its unit: `5 s`,
  `%1 min`, `%1 ms`, `-3 dB`, `%1 Hz`, `48 kHz`, `%1 W`, `%1 %` (French
  puts a space before `%` too: `Augmenter la luminosité de %1 %`).
  Seconds are `s` (never `sec` or `s.`), minutes `min`, hours `h`.
  Sizes are in octets: GB `Go`, MB `Mo`, KB `Ko` (`%2 sur %3 Go`, `16 Mo`).
  fps is `i/s` (images par seconde). `%1-bit` is `%1 bits`. Kept attached
  as in the source: `1080p`, `%3p`, `4K`, `%1x` (a multiplier), `%1°`,
  `%1D`. Time fields keep the formats the program reads (`h:mm:ss.s`,
  `hh:mm`), whose letters suit French anyway.
- **Numbers:** a decimal number the translation writes itself takes a
  comma (`1,5 s`); numbers the program fills in keep its own format.
  Versions, addresses and channel layouts keep their periods (`TensorRT
  10.15.1`, `127.0.0.1`, `5.1`, `7.1`). Ordinals: `1er`, `2e`, `3e`
  (`%1e périphérique`, `{Identique au %1er logo}`); never `2ème`, `2nd`.
  Number sign: `n°` (`Style n° %1`).
- **Agreement and placeholders.** French participles and adjectives agree
  in gender and number, and a placeholder's gender is unknown: put the
  noun first and the placeholder after a colon (`Retiré : %1`,
  `État : %1`), or name the noun (`le moteur %1`, `le fichier %1`).
  Words the program inserts (`checking`, `downloading`) are translated as
  nouns (`vérification`, `téléchargement`), so they fit any sentence.
  Counts keep the source's `(s)` (`%1 fichier(s)`); where the source has
  separate singular and plural strings, French singular covers 0 and 1.
  Never elide before a placeholder (`de %1`, not `d'%1`). Before names:
  `d'`/`l'` before a written vowel (`d'AMD`, `l'API`, `d'AviSynth`), `de`
  before a consonant even when the letter is spoken with a vowel (`de
  RVP`, `de SVP`, `de mpv`, `de NVIDIA`, `de MVTools`), as French writes
  `le RER`. Names of programs and techniques are masculine (`RVP est
  installé`, `le GPU`, `le HDR`), except those that are feminine French
  nouns: `une URL`, `la webcam`, `la 3D`.
- **Recommended:** `(recommandé)`, always this one masculine tag in
  lowercase, whatever the noun (as Windows writes it); `Recom'd` too.
- **Keys** keep RVP's English names in every text, as its menus show
  them (`Shift`, `Enter`, `Esc`, `Space`, `Left / Right`, `Scroll Lock`,
  `X1`), not the French keyboard's `Maj`, `Entrée`, `Échap`. Prose around
  them is French: `les touches fléchées`, `la molette`.
- **Example values** in empty fields: language codes become French
  users' own (`rus,eng` becomes `fre,eng`, `rus` becomes `fre`, `en,ru`
  becomes `fr,en`, `en` stays); chapter names (`Opening;Ending;3`,
  matched against the files' own chapter titles), paths and folder names
  (`Music/RVP`, `V:`) stay.
- **Fragments** that the program joins keep the source's leading and
  trailing spaces and punctuation (`. Allégé : %1.`, a text ending in
  `, `).
- **Keeping short labels short.** French runs about a fifth longer than
  English, and `sous-titres` and `liste de lecture` have no shorter
  standard form, so save elsewhere: prefer short words (`si` rather than
  `lorsque`, `selon` rather than `en fonction de`, `pour` rather than
  `afin de`, `sans` rather than `en l'absence de`, `Taille` rather than
  `Dimensions`); a noun phrase may leave out the article (`Taille de
  fenêtre`, `Synchronisation audio`), a verb phrase keeps it (`Ouvrir le
  dossier`); the accepted short forms are `auto`, `infos`, `max.`, `n°`
  and nothing else (write `minimum` in full: `min` is minutes). Never cut
  grammar into telegraphic French (`Police sous-titres`) to win room.
  Help texts may grow.
- **Spelling:** traditional spelling, as the desktops write it: `boîte`,
  `événement`, `connaître`, `paraître`. `e.g.` is `p. ex.`, `i.e.` is
  `c.-à-d.`.

## Glossary

| English | French | Note |
|---------|--------|------|
| play / pause / stop | lire / pause / arrêter | Commands are verbs: `Lire`, `Arrêter`; pause as a verb is `mettre en pause`. The play/pause toggle is `Lecture/Pause`. |
| playback | lecture | The Playback menu and page: `Lecture`. |
| close playback | fermer le média | Stop and unload what plays (file, disc or stream): `Fermer le média`, `Back ferme le média`; not `fermer la lecture`. |
| previous / next | précédent / suivant | Agree with the noun: `image suivante`, `piste précédente`. |
| seek | se déplacer | In the file; the noun is `déplacement` (`là où arrive un déplacement`). Not `rechercher`, which is search. |
| jump | aller à | `Aller à un temps/une image...`; the noun is `saut` (arrow-key time jumps: `Sauts`). |
| frame step | image par image | `Avancer image par image`; single steps are `Image suivante` / `Image précédente`. |
| speed | vitesse | |
| repeat | répéter | The noun is `répétition`. |
| A-B repeat / section repeat | répétition A-B / répétition de passage | A marked part is a `passage` (`Répéter le passage marqué`). |
| shuffle | lecture aléatoire | `Aléatoire` alone where a list of modes already says `lecture`. |
| queue | file d'attente | |
| resume / remember position | reprise / mémoriser la position | The verb is `reprendre`; `mémoriser`, not `se souvenir de`. |
| position | position | |
| duration | durée | |
| elapsed / remaining time | écoulé / temps restant | In full: `temps écoulé`, `temps restant`. |
| chapter | chapitre | |
| bookmark | signet | |
| skip (intro, ending) | passer (intro, fin) | `Passer l'intro`, `Passer la fin`, `Passer des chapitres`. Not `sauter`: `saut` is the jump's noun. |
| seek bar | barre de progression | Also for the source's "nav bar" and "navigation bar". |
| thumbnail | miniature | |
| preview | aperçu | |
| scene browser | explorateur de scènes | Browse Scenes is `Parcourir les scènes`. |
| file | fichier | |
| folder | dossier | |
| URL / address | URL / adresse | `URL` is feminine: `une URL`. |
| stream | flux | Network media; a stream inside a file is a `piste` (`Choisir la piste audio`). Record Stream is `Enregistrer le flux`. |
| device / capture device | périphérique / périphérique de capture | A webcam is `webcam` (feminine). |
| screen capture | capture du bureau | The desktop as a video source. Never `capture d'écran`, which French users read as a screenshot. |
| disc | disque | |
| title | titre | |
| angle | angle | |
| disc menu | menu du disque | |
| drag and drop | glisser-déposer | |
| recent files | fichiers récents | Recent items: `éléments récents`. |
| favorites | favoris | |
| file association / file type | association de fichiers / type de fichier | |
| playlist | liste de lecture | Help texts may say `la liste` once they have named it. |
| album | album | |
| entry | élément | Also for the source's "item" of the playlist. |
| sort | trier | The noun is `tri`. |
| add / remove / delete | ajouter / retirer / supprimer | Trap: `retirer` takes an entry out of a list (playlist, folders, keys) and leaves the file; `supprimer` destroys the file, bookmark, album or preset itself. |
| recycle bin / trash | corbeille / corbeille | Windows and the Linux desktops both say `Corbeille`. |
| import / export | importer / exporter | |
| window / main window | fenêtre / fenêtre principale | |
| fullscreen / windowed | plein écran / fenêtré | `en plein écran`, `en mode fenêtré`. |
| maximize / minimize | agrandir / réduire | Windows' words; Restore is `Restaurer`. |
| always on top / stay on top | toujours au premier plan / rester au premier plan | |
| opacity | opacité | |
| skin | habillage | Not `skin`, not `thème` (kept for color themes), not `apparence`. Video skin: `habillage vidéo`; skinned windows: `fenêtres habillées`. |
| color theme | thème de couleurs | |
| logo | logo | |
| title bar | barre de titre | |
| control panel | panneau de contrôle | RVP's F7 window. Never `panneau de configuration`, which is Windows' Control Panel. |
| OSD / on-screen display | OSD / affichage à l'écran | `OSD` is masculine; OSD Messages: `Messages à l'écran`. |
| on-screen controller | contrôleur à l'écran | mpv's OSC, uosc and the like, drawn over the video; not the gamepad (`manette`). |
| tooltip | info-bulle | Plural `info-bulles`. |
| context menu | menu contextuel | |
| tray icon / notification area | icône de la zone de notification / zone de notification | "Tray" alone is `zone de notification`. |
| taskbar | barre des tâches | On Linux the dock is `dock`. |
| notification | notification | |
| dialog | boîte de dialogue | `dialogue` alone only where the context is clear. |
| settings / preferences | paramètres / préférences | The settings window is `Paramètres`; a page's items are `options`. |
| page | page | |
| preset / configuration | préréglage / configuration | Also equalizer and frame size presets (`Préréglage %1`); "setup" is `réglages`, not `configuration`. |
| reset / default | réinitialiser / par défaut | The Default button is `Par défaut`; RVP's defaults are `les valeurs par défaut de RVP`. |
| apply | appliquer | |
| key / hotkey / shortcut | touche / raccourci clavier / raccourci | |
| mouse / wheel / button | souris / molette / bouton | |
| gesture | geste | `geste de souris`. |
| touch | tactile | `fonction tactile`, `écran tactile`; the Touch page is `Tactile`. |
| controller / gamepad | manette / manette de jeu | |
| video / picture / image | vidéo / image / image | |
| frame | image | As in `images par seconde`; a field of interlaced video is a `trame`. The video's rectangle ("video frame", "frame size") may be `cadre` where `image` would be read as a frame. |
| frame rate / fps | fréquence d'images / i/s | A rate alone is `fréquence` (`la fréquence de l'écran`). |
| aspect ratio | proportions | Feminine plural: `Conserver les proportions`. The source's "AR" is `proportions` too: French has no such abbreviation. |
| crop | rogner | |
| zoom / pan | zoom / déplacer | Zoom In / Out: `Zoom avant` / `Zoom arrière`; to pan the picture is `déplacer l'image`; a camera pan is a `panoramique`. |
| rotate / flip / mirror | faire pivoter / retourner / miroir | The noun is `rotation`; flips: `Retourner horizontalement` / `verticalement`. |
| deinterlace | désentrelacer | The noun is `désentrelacement`. |
| brightness / contrast / saturation / hue / gamma | luminosité / contraste / saturation / teinte / gamma | |
| sharpen / denoise | netteté / débruitage | Labels are the nouns (`Netteté (Ctrl+R)`, `Débruitage 3D`); the verbs are `accentuer la netteté` / `débruiter`. Sound's noise reduction is `réduction du bruit`. |
| scaler / upscaling / resize | mise à l'échelle / agrandissement / redimensionner | A scaler as a method is `algorithme` where needed (`les algorithmes EWA`); the Resize page is `Redimensionnement`. |
| pixel shader | pixel shader | Masculine, plural `pixel shaders`, as French GPU users write it; `shader` alone in short labels. |
| renderer | rendu | `Rendu vidéo`, `Rendu audio`; not `moteur de rendu` (`moteur` is the engine). |
| hardware decoding / decoder | décodage matériel / décodeur | |
| HDR | HDR | Masculine: `le HDR`. |
| 3D / 360° video / anaglyph / side by side / top and bottom | 3D / vidéo 360° / anaglyphe / côte à côte / haut et bas | |
| bars / letterbox / margin | bandes / bandes noires / marge | Black areas only; the skin's control bars are `barres`. |
| frame capture / screenshot | capture d'image / capture d'écran | |
| motion smoothing | lissage du mouvement | `lissage` alone once the context is set. |
| frame interpolation | interpolation d'images | |
| engine | moteur | |
| calibration | calibrage | The verb is `calibrer`. |
| quality preset (light, medium, high) | préréglage de qualité (léger, moyen, élevé) | The presets shown are `Rapide`, `Moyen`, `Qualité`. |
| GPU / graphics card | GPU / carte graphique | `le GPU`. |
| processor / CPU | processeur / CPU | `le CPU`. |
| power budget | budget de puissance | A card's power is its `puissance` or `consommation` in watts, not `énergie`. |
| dropped frames | images perdues | |
| refresh rate | fréquence de rafraîchissement | |
| audio / sound | audio / son | |
| volume / mute | volume / muet | The Mute command is `Muet`; as a verb, `couper le son`. |
| master volume | volume général | |
| equalizer | égaliseur | |
| normalizer | normalisation | The feature, as French users name it (`normalisation du volume`). |
| reverb | réverbération | |
| noise reduction | réduction du bruit | |
| channel / speaker | canal / haut-parleur | Plurals `canaux`, `haut-parleurs`. |
| downmix | mixage réducteur | In help texts, say what it does: `réduit le 5.1 en stéréo`. |
| surround / stereo | surround / stéréo | |
| output device | périphérique de sortie | |
| sample rate / bitrate | fréquence d'échantillonnage / débit | |
| audio delay / audio sync | décalage audio / synchronisation audio | Trap: never `délai` (a time limit). The verb is `synchroniser`. |
| pitch / time stretching | hauteur / étirement temporel | `hauteur du son` where unclear; not `tonalité` (a musical key). |
| audio recording | enregistrement audio | |
| second audio device | second périphérique audio | |
| pass-through | transmission directe | Sending the encoded sound on unchanged. |
| subtitles / subtitle track | sous-titres / piste de sous-titres | One line is a `sous-titre`. |
| external / embedded subtitles | externes / sous-titres intégrés | In full: `sous-titres externes`. |
| font / outline / shadow | police / contour / ombre | |
| subtitle position / subtitle delay | position des sous-titres / décalage des sous-titres | Not `délai`. |
| encoding | encodage | A text file's `encodage des caractères`. |
| subtitle browser / subtitle editor | explorateur de sous-titres / éditeur de sous-titres | |
| subtitles from audio / speech recognition | sous-titres depuis l'audio / reconnaissance vocale | Making them: `Créer des sous-titres depuis l'audio`. |
| model | modèle | Whisper's model names (`tiny`, `base`...) stay. |
| language | langue | Never `langage` (a programming language). |
| remote control | contrôle à distance | A remote (the phone program or its layout) is a `télécommande`. |
| token | jeton | |
| port / server | port / serveur | |
| log | journal | |
| crash | plantage | |
| download / install | télécharger / installer | |
| installer | programme d'installation | |
| accessibility | accessibilité | |
| text to speech (TTS) / screen reader | synthèse vocale (TTS) / lecteur d'écran | |
| screen saver | écran de veille | |
| shut down / sleep / hibernate | arrêter / mettre en veille / mettre en veille prolongée | Windows' words; nouns `arrêt`, `veille`, `veille prolongée`. Where `arrêter` could be read as stopping playback, say `arrêter l'ordinateur`. |
| timer | minuteur | |
| visualization / spectrum | visualisation / spectre | |
| status bar / info | barre d'état / infos | `infos` also for the Info log level. |
| codec / format | codec / format | |
| bit depth / 10-bit | profondeur de bits / 10 bits | |
| channel layout (5.1, 7.1) | disposition des canaux (5.1, 7.1) | |
| Auto select (recommended) | Automatique (recommandé) | Auto select alone is `Automatique`. |
