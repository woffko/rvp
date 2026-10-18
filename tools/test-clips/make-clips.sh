#!/usr/bin/env bash
# Generates the synthetic test clips used by the goal C and goal D checks
# (S1, S3, R1 and S5). Everything is made from ffmpeg's built-in sources, so
# no third-party media is needed and the clips can be rebuilt bit for bit.
#
# usage: tools/test-clips/make-clips.sh OUT_DIR
#   FFMPEG  ffmpeg binary to use (default: ffmpeg on PATH)
#
# Clips:
#   video-2160p60-hevc.mkv      S3 performance: 3840x2160, 60 fps, HEVC, 30 s
#   video-1080p23976-h264.mkv   S1 (SVP) and R1 "playing": 1920x1080, 24000/1001 fps, 60 s
#   video-2160p24-hdr10.mkv     M12's HDR checks: HEVC 10-bit HDR10 (PQ, BT.2020,
#                               1000-nit mastering metadata), 3840x2160, 24 fps, 20 s
#   audio-dialogue-action.flac  S5: quiet speech-like passages and loud bursts, 90 s
#   audio-music.flac            S5: music-like chord with pp, crescendo and ff, 90 s
#   audio-steps.flac            S5: level steps and tone bursts for attack and release, 70 s
set -euo pipefail

OUT=${1:?usage: make-clips.sh OUT_DIR}
FFMPEG=${FFMPEG:-ffmpeg}
RATE=48000
mkdir -p "$OUT"
ff() { "$FFMPEG" -hide_banner -nostdin -loglevel error -y "$@"; }

# ffmpeg's sine source has a fixed amplitude of 1/8 (-18 dBFS); the volume
# factors below are relative to that.
# A quiet stereo tone keeps an audio clock in the video clips.
tone="sine=frequency=440:sample_rate=$RATE,volume=0.03,aformat=channel_layouts=stereo"

ff -f lavfi -i "testsrc2=size=3840x2160:rate=60" -f lavfi -i "$tone" -t 30 \
    -c:v libx265 -preset fast -b:v 40M -pix_fmt yuv420p -x265-params log-level=error \
    -c:a flac "$OUT/video-2160p60-hevc.mkv"

ff -f lavfi -i "testsrc2=size=1920x1080:rate=24000/1001" -f lavfi -i "$tone" -t 60 \
    -c:v libx264 -preset medium -crf 18 -pix_fmt yuv420p -c:a flac "$OUT/video-1080p23976-h264.mkv"

# HDR10: the test pattern at 203-nit diffuse white over a grey ramp of PQ
# values from 0 to 1000 nits (PQ 0.7518), drawn once and repeated.
ff -f lavfi -i "testsrc2=size=3840x1620:rate=24" -f lavfi -i "color=c=black:size=3840x540:rate=24" \
    -f lavfi -i "$tone" -t 20 -filter_complex "
[0:v]zscale=tin=bt709:min=bt709:pin=bt709:rin=tv:t=smpte2084:m=bt2020nc:p=bt2020:r=tv:npl=203,format=yuv420p10le[top];
[1:v]format=yuv420p10le,trim=end_frame=1,geq=lum='64+876*0.7518*X/W':cb=512:cr=512,loop=loop=-1:size=1[ramp];
[top][ramp]vstack=inputs=2,setparams=range=tv:color_primaries=bt2020:color_trc=smpte2084:colorspace=bt2020nc[v]" -map "[v]" -map 2:a \
    -c:v libx265 -preset fast -b:v 30M -pix_fmt yuv420p10le \
    -x265-params "log-level=error:hdr10=1:repeat-headers=1:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:master-display=G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(10000000,1):max-cll=1000,400" \
    -color_primaries bt2020 -color_trc smpte2084 -colorspace bt2020nc \
    -c:a flac "$OUT/video-2160p24-hdr10.mkv"

# Dialogue and action, 30 s cycle: 20 s of band-limited noise with a
# syllable-like envelope at a low level, then 10 s of loud broadband noise
# with low-frequency hits.
speech="anoisesrc=color=white:seed=11:sample_rate=$RATE,highpass=f=300,lowpass=f=3400,
volume='0.12*(0.55+0.45*sin(2*PI*4.1*t))*(0.7+0.3*sin(2*PI*0.53*t))*lt(mod(t,30),20)':eval=frame"
action="anoisesrc=color=pink:seed=12:sample_rate=$RATE,
volume='0.9*(0.6+0.4*abs(sin(2*PI*1.7*t)))*gte(mod(t,30),20)':eval=frame"
hits="sine=frequency=55:sample_rate=$RATE,
volume='5*lt(mod(t,0.75),0.15)*gte(mod(t,30),20)':eval=frame"
ff -filter_complex "$speech[s];$action[a];$hits[h];[s][a][h]amix=inputs=3:normalize=0,
aformat=channel_layouts=stereo,alimiter=limit=0.95:level=disabled" -t 90 -c:a flac "$OUT/audio-dialogue-action.flac"

# Music: an A major chord with harmonics and a slow tremolo. Dynamics:
# pp 0-30 s, crescendo 30-60 s, ff 60-70 s, sudden pp 70-80 s, ff 80-90 s.
chord="aevalsrc='(sin(2*PI*220*t)+0.5*sin(2*PI*440*t)+0.8*sin(2*PI*277.18*t)+0.4*sin(2*PI*554.37*t)
+0.7*sin(2*PI*329.63*t)+0.3*sin(2*PI*659.26*t))/4*(0.85+0.15*sin(2*PI*5*t))':s=$RATE"
dynamics="volume='if(lt(t,30),0.02,if(lt(t,60),0.02+0.88*(t-30)/30,if(lt(t,70),0.9,if(lt(t,80),0.02,0.9))))':eval=frame"
ff -filter_complex "$chord,$dynamics,aformat=channel_layouts=stereo" -t 90 -c:a flac "$OUT/audio-music.flac"

# Steps: pink noise at four levels (0-40 s, 10 s each), 1 kHz bursts (1 s
# on, 3 s off) at 40-50 s, silence at 50-60 s, then the lowest noise level
# again at 60-70 s. Mono sources lose 3 dB in the stereo upmix.
steps="anoisesrc=color=pink:seed=13:sample_rate=$RATE,
volume='if(lt(t,10),0.006,if(lt(t,20),0.02,if(lt(t,30),0.06,if(lt(t,40),0.2,if(lt(t,50),0,if(lt(t,60),0,0.006))))))':eval=frame"
bursts="sine=frequency=1000:sample_rate=$RATE,volume='2*between(t,40,50)*lt(mod(t,4),1)':eval=frame"
ff -filter_complex "$steps[n];$bursts[b];[n][b]amix=inputs=2:normalize=0,aformat=channel_layouts=stereo" \
    -t 70 -c:a flac "$OUT/audio-steps.flac"

# Loudness summary (EBU R128) of the audio clips.
for clip in audio-dialogue-action audio-music audio-steps; do
    printf '%s: ' "$clip"
    "$FFMPEG" -hide_banner -nostdin -i "$OUT/$clip.flac" -af ebur128=peak=true -f null - 2>&1 |
        sed -n '/Summary:/,$p' | grep -E 'I:|LRA:|Peak:' | tr -s ' ' | tr '\n' ' '
    echo
done
ls -l "$OUT"
