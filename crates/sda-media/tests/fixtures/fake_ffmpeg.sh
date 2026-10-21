#!/bin/sh
# Stands in for ffmpeg in tests. Reads the arguments Lumen passes and
# answers like ffmpeg would, without decoding anything.
#
#   fake_ffmpeg.sh -i FILE                        info on stderr, exit 1
#   fake_ffmpeg.sh ... -f rawvideo ... -s WxH ... -   N RGBA frames on stdout
#   fake_ffmpeg.sh ... -f f32le ... -              N seconds of stereo PCM
#
# FAKE_FRAMES sets the frame count, default 6. FAKE_SECONDS the audio
# length, default 2. FAKE_FAIL=1 exits 1 with an error line at once.

if [ "$FAKE_FAIL" = "1" ]; then
  echo "clip.mp4: Invalid data found when processing input" >&2
  exit 1
fi

size=""
fmt=""
prev=""
for a in "$@"; do
  case "$prev" in
    -s) size="$a" ;;
    -f) fmt="$a" ;;
  esac
  prev="$a"
done

if [ -z "$fmt" ]; then
  cat >&2 <<INFO
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':
  Duration: 00:00:02.00, start: 0.000000, bitrate: 100 kb/s
  Stream #0:0: Video: h264, yuv420p, 160x90, 30 fps
  Stream #0:1: Audio: aac, 48000 Hz, stereo, fltp
At least one output file must be specified
INFO
  exit 1
fi

if [ "$fmt" = "rawvideo" ]; then
  w=${size%x*}
  h=${size#*x}
  n=${FAKE_FRAMES:-6}
  i=0
  while [ "$i" -lt "$n" ]; do
    # One frame of solid color, the red channel counts the frame.
    head -c $((w * h * 4)) /dev/zero | tr '\0' "$(printf '\\%03o' $((i + 1)))"
    i=$((i + 1))
  done
  exit 0
fi

if [ "$fmt" = "f32le" ]; then
  n=${FAKE_SECONDS:-2}
  # BSD head rejects a zero count, so zero seconds writes nothing.
  if [ "$n" -gt 0 ]; then
    head -c $((48000 * 2 * 4 * n)) /dev/zero
  fi
  exit 0
fi
echo "fake_ffmpeg: unknown output $fmt" >&2
exit 2
