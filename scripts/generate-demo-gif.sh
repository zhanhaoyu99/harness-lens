#!/bin/sh

set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
overview="$repo_root/docs/assets/overview.png"
runs="$repo_root/docs/assets/runs.png"
compare="$repo_root/docs/assets/snapshot-compare.png"
output="$repo_root/docs/assets/harness-lens-tour.gif"
temp_output="$(mktemp "$repo_root/docs/assets/.harness-lens-tour.XXXXXX")"

cleanup() {
  rm -f "$temp_output"
}
trap cleanup EXIT HUP INT TERM

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is required to generate the README tour." >&2
  exit 1
fi

for input in "$overview" "$runs" "$compare"; do
  if [ ! -f "$input" ]; then
    echo "Missing synthetic source frame: $input" >&2
    exit 1
  fi
done

# Three 11-second synthetic screens with short cross-fades produce a 31.6-second
# tour. Static source frames keep the result small and make every published byte
# reviewable before it reaches the README.
ffmpeg \
  -y \
  -hide_banner \
  -loglevel error \
  -loop 1 -t 11 -i "$overview" \
  -loop 1 -t 11 -i "$runs" \
  -loop 1 -t 11 -i "$compare" \
  -filter_complex \
  "[0:v]fps=5,scale=960:600:force_original_aspect_ratio=decrease,pad=960:600:(ow-iw)/2:(oh-ih)/2:color=0x0c1117,format=rgba[v0];\
[1:v]fps=5,scale=960:600:force_original_aspect_ratio=decrease,pad=960:600:(ow-iw)/2:(oh-ih)/2:color=0x0c1117,format=rgba[v1];\
[2:v]fps=5,scale=960:600:force_original_aspect_ratio=decrease,pad=960:600:(ow-iw)/2:(oh-ih)/2:color=0x0c1117,format=rgba[v2];\
[v0][v1]xfade=transition=fade:duration=0.75:offset=10.25[x1];\
[x1][v2]xfade=transition=fade:duration=0.75:offset=20.5,split[p0][p1];\
[p0]palettegen=max_colors=128:stats_mode=diff[pal];\
[p1][pal]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle" \
  -loop 0 \
  -fflags +bitexact \
  -flags:v +bitexact \
  -f gif \
  "$temp_output"

mv "$temp_output" "$output"
chmod 0644 "$output"

echo "Generated $output"
