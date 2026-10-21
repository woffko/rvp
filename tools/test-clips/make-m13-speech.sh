#!/usr/bin/env bash
# M13's synthetic speech clips for subtitles from audio (goal O), spoken by
# espeak-ng from RVP's own text, never committed:
#   speech.wav       one sentence, about 4 s (tests/speech.rs, m13-all.ps1)
#   speech-long.wav  several paragraphs, about 1.5 min (the host session's
#                    timings, m13-host.ps1)
# espeak-ng comes from the system or from ESPEAK (with ESPEAK_DATA_PATH
# for an unpacked copy, as in ~/rvp-deps/espeak).
#
# usage: tools/test-clips/make-m13-speech.sh OUT_DIR
set -euo pipefail

OUT=${1:?usage: make-m13-speech.sh OUT_DIR}
ESPEAK=${ESPEAK:-espeak-ng}
mkdir -p "$OUT"

"$ESPEAK" -w "$OUT/speech.wav" "Hello world. This is a test of subtitles made from speech."

"$ESPEAK" -s 150 -w "$OUT/speech-long.wav" "\
A media player has a simple task on paper. It opens a file, reads the \
pictures and the sound, and shows them together at the right time. In \
practice every part of that sentence hides a long list of details. Files \
come in many containers, each with its own way of storing streams, \
chapters and attachments. Pictures arrive in different sizes, colour \
spaces and frame rates, and the screen they go to has its own rate and \
its own idea of colour. Sound may have two channels or eight, and the \
speakers in the room may have fewer. \
Subtitles add another layer. Some are pictures taken from a disc, others \
are text with styles, positions and effects. A good player lets the \
viewer move them, make them larger, change their colours, and shift them \
in time when they do not match the voices. When no subtitles exist at \
all, a player can now listen to the sound itself and write them down, \
line by line, while the film plays. \
This clip was made by a speech synthesizer to test exactly that. The \
voice is plain and the words are clear, so a small speech model should \
hear most of them. The test measures how long the work takes on the \
processor and on the graphics card, and checks that the words come back \
as subtitles with sensible times. \
The last paragraph counts slowly from one to ten. One. Two. Three. Four. \
Five. Six. Seven. Eight. Nine. Ten. That is the end of the clip."
ls -la "$OUT"/speech*.wav
