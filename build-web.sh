#!/bin/sh
# Builds the browser version into dist/. Upload everything in dist/ to any
# static web host; it needs no server-side code.
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
rm -rf dist
mkdir dist
cp target/wasm32-unknown-unknown/release/carmen.wasm web/mq_js_bundle.js web/storage.js web/input.js dist/
# The loading bar needs the game's real size (see web/index.html).
sed "s/__WASM_BYTES__/$(wc -c < dist/carmen.wasm | tr -d ' ')/" web/index.html > dist/index.html
echo "Built dist/. Try it locally with: python3 -m http.server -d dist 8000"
