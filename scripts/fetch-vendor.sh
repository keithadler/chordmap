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

# transformers.js (Apache-2.0) for lyrics transcription with Whisper, plus the
# exact onnxruntime-web build it depends on (a dev version, so use npm pack).
T=4.2.0
if [ ! -f examples/web/vendor/tjs/transformers.min.js ]; then
  mkdir -p examples/web/vendor/tjs
  tmp=$(mktemp -d)
  (cd "$tmp" && npm pack "@huggingface/transformers@$T" >/dev/null 2>&1 && tar xzf huggingface-transformers-*.tgz)
  OV=$(node -p "require('$tmp/package/package.json').dependencies['onnxruntime-web']")
  (cd "$tmp" && npm pack "onnxruntime-web@$OV" >/dev/null 2>&1 && mkdir -p ort && tar xzf onnxruntime-web-*.tgz -C ort)
  cp "$tmp/package/dist/transformers.min.js" examples/web/vendor/tjs/
  # The runtime picks a build at load time (plain, jsep for WebGPU, asyncify
  # or jspi for async wasm), so ship all of them.
  cp "$tmp"/ort/package/dist/ort-wasm-simd-threaded*.mjs "$tmp"/ort/package/dist/ort-wasm-simd-threaded*.wasm examples/web/vendor/tjs/
  cp "$tmp/package/LICENSE" examples/web/vendor/tjs/LICENSE 2>/dev/null || true
  rm -rf "$tmp"
  echo "vendored transformers.js $T with onnxruntime-web $OV"
fi
