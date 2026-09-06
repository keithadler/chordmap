#!/usr/bin/env sh
# Copies onnxruntime-web (MIT) into examples/web/vendor/ort for the stem
# separation feature. Not committed; run before serving or deploying.
set -eu
cd "$(dirname "$0")/.."
V=1.29.0
mkdir -p examples/web/vendor/ort
for f in ort.all.min.js ort-wasm-simd-threaded.mjs ort-wasm-simd-threaded.wasm ort-wasm-simd-threaded.jsep.mjs ort-wasm-simd-threaded.jsep.wasm; do
  [ -f "examples/web/vendor/ort/$f" ] || curl -sSL -o "examples/web/vendor/ort/$f" "https://cdn.jsdelivr.net/npm/onnxruntime-web@$V/dist/$f"
done
curl -sSL -o examples/web/vendor/ort/LICENSE "https://cdn.jsdelivr.net/npm/onnxruntime-web@$V/LICENSE" || true
echo "vendored onnxruntime-web $V"
