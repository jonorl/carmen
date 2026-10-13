#!/bin/sh
# Cuts the bundled fonts down to the characters the game can show, since
# they're built into the program and every byte is downloaded on the web:
# Latin with accents (English, Spanish and players' names), curly quotes,
# dashes, the ellipsis, € and ₹. Noto Sans is only there for ₹, which
# Pixelify Sans lacks (see MISSING in src/ui.rs). Running it again on cut
# fonts changes nothing. Hinting goes too: macroquad's renderer ignores it.
# Needs fontTools (pip install fonttools).
set -e
cd "$(dirname "$0")/../assets/fonts"
latin="U+0020-007E,U+00A0-017F,U+2010-2027,U+20AC,U+20B9"
for font in PixelifySans.ttf VT323.ttf; do
    pyftsubset "$font" --unicodes="$latin" --layout-features='*' --no-hinting --output-file="$font.tmp"
    mv "$font.tmp" "$font"
done
pyftsubset NotoSans-Regular.ttf --unicodes="U+0020,U+20B9" --no-hinting --output-file=NotoSans-Regular.ttf.tmp
mv NotoSans-Regular.ttf.tmp NotoSans-Regular.ttf
ls -l *.ttf
