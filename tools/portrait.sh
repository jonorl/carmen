#!/bin/sh
# Turns a photo into a dossier portrait in the style of 1990s digitised
# pictures: 66x96 pixels in 28 colours, shown at double size in the game.
#
#   tools/portrait.sh PHOTO "Suspect Name" [CROP]
#
# CROP is an optional ImageMagick geometry (WIDTHxHEIGHT+X+Y) choosing the
# part of the photo to use: head and shoulders, about 2 wide by 3 high.
# Without it the middle of the photo is used. Colour is boosted to 120%;
# set SATURATION=75 (say) first for photos that are already warm or vivid.
# Needs ImageMagick 7 (the `magick` command). Rebuild the game afterwards
# to pick the picture up.
set -e
if [ $# -lt 2 ]; then
    sed -n '2,12p' "$0"
    exit 1
fi
photo=$(realpath "$1")
# The file name the game looks for: "Jon Orlo" becomes jon-orlo.png.
slug=$(printf '%s' "$2" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]\{1,\}/-/g; s/^-//; s/-$//')
cd "$(dirname "$0")/.."
mkdir -p assets/suspects
out="assets/suspects/$slug.png"
if [ -n "$3" ]; then
    set -- -crop "$3" +repage
else
    set --
fi
magick "$photo" "$@" -resize '66x96^' -gravity center -extent 66x96 \
    -modulate "100,${SATURATION:-120}" -sigmoidal-contrast 4x50% +dither -colors 28 -strip "$out"
echo "Wrote $out"
