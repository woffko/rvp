#!/bin/bash
# The audio audit's test tones (docs/audits/audio.md), made with ffmpeg:
#   tone-stereo.flac  1 kHz at 0.95 of full scale, stereo, 48 kHz, 120 s
#                     (above TomSteady's 75 % target: both normalizers
#                     settle by cutting, every block has the same peak)
#   tone-51.wav       5.1 (side), 48 kHz: after 1 s of silence each channel
#                     alone for 2.5 s, then 1 s of silence, in the order
#                     FL FR FC LFE SL SR (the LFE at 50 Hz), 0.95 of full
#                     scale
#   tone-51-ac3.mkv   the same as AC3 at 640 kbit/s
# usage: make-tones.sh FFMPEG OUT_DIR
set -eu
ffmpeg=$1
out=$2
mkdir -p "$out"
"$ffmpeg" -nostdin -loglevel error -y -f lavfi \
    -i "aevalsrc=0.95*sin(2*PI*1000*t)|0.95*sin(2*PI*1000*t):s=48000:d=120" \
    -c:a flac -sample_fmt s32 -bits_per_raw_sample 24 "$out/tone-stereo.flac"
expr=""
for channel in 0 1 2 3 4 5; do
    frequency=1000
    [ "$channel" = 3 ] && frequency=50
    start=$(echo "1+3.5*$channel" | bc)
    end=$(echo "3.5+3.5*$channel" | bc)
    expr="$expr${expr:+|}0.95*sin(2*PI*$frequency*t)*between(t\\,$start\\,$end)"
done
"$ffmpeg" -nostdin -loglevel error -y -f lavfi -i "aevalsrc=$expr:s=48000:d=22:c=5.1(side)" \
    -c:a pcm_s24le "$out/tone-51.wav"
"$ffmpeg" -nostdin -loglevel error -y -i "$out/tone-51.wav" -c:a ac3 -b:a 640k "$out/tone-51-ac3.mkv"
