#!/bin/sh
# Build the browser playground: compile the crate to wasm32 and place the
# module next to the static page that loads it.
#
# Usage:  ./build.sh            (from playground-web/)
# Then:   serve web/ over HTTP, e.g.  python3 -m http.server -d web 8000
#         and open http://localhost:8000/
#
# Requires the wasm32-unknown-unknown target:
#   rustup target add wasm32-unknown-unknown
set -eu

cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/izanagi_playground_web.wasm web/
echo "built web/izanagi_playground_web.wasm — serve the web/ directory over HTTP"
