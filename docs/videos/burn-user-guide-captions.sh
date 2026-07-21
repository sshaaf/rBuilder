#!/usr/bin/env bash
# Burn docs/videos/user-guide-cli.srt onto the no-captions recording.
# Leaves user-guide-cli-no-captions.mp4 untouched for comparison.
#
# Usage (from repo root):
#   ./docs/videos/burn-user-guide-captions.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VID="$ROOT/docs/videos"
SRC="$VID/user-guide-cli-no-captions.mp4"
SRT="$VID/user-guide-cli.srt"
OUT="$VID/user-guide-cli.mp4"

if [[ ! -f "$SRC" ]]; then
  echo "error: missing $SRC — record first: ./docs/videos/record-user-guide-cli.sh" >&2
  exit 1
fi
if [[ ! -f "$SRT" ]]; then
  echo "error: missing $SRT" >&2
  exit 1
fi

FFMPEG=ffmpeg
if [[ -x /opt/homebrew/opt/ffmpeg-full/bin/ffmpeg ]]; then
  FFMPEG=/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg
elif ! ffmpeg -filters 2>/dev/null | grep -q 'subtitles'; then
  echo "error: ffmpeg lacks subtitles filter — brew install ffmpeg-full" >&2
  exit 1
fi

# Run from videos dir so the SRT path has no special chars for the filter.
cd "$VID"
"$FFMPEG" -y -i user-guide-cli-no-captions.mp4 \
  -vf "subtitles=filename=user-guide-cli.srt:force_style='FontName=Menlo,FontSize=20,PrimaryColour=&H00FFFFFF&,OutlineColour=&H80000000&,BackColour=&H80000000&,BorderStyle=3,Outline=1,Shadow=0,MarginV=40,Alignment=2'" \
  -c:v libx264 -pix_fmt yuv420p -crf 20 \
  user-guide-cli.mp4

echo "==> wrote $OUT"
ls -lh "$OUT" "$SRC"
