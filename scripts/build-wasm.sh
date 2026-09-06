#!/usr/bin/env sh
# Builds the npm package into ./pkg (ES module for browsers) and copies the
# two files the web app needs into examples/web/pkg. Requires wasm-pack.
set -eu
cd "$(dirname "$0")/.."
wasm-pack build crates/chordmap-wasm --release --target web --out-dir ../../pkg --out-name chordmap
cp LICENSE pkg/LICENSE
cp README.md pkg/README.md 2>/dev/null || true
node - <<'JS'
const fs = require("fs");
const p = JSON.parse(fs.readFileSync("pkg/package.json", "utf8"));
p.name = "chordmap";
p.description = "Tempo, key, chords, sections and a capo suggestion from audio, in WebAssembly.";
p.keywords = ["music", "audio", "chords", "tempo", "bpm", "key", "wasm"];
p.homepage = "https://github.com/keithadler/chordmap";
p.repository = { type: "git", url: "git+https://github.com/keithadler/chordmap.git" };
p.bugs = { url: "https://github.com/keithadler/chordmap/issues" };
p.license = "MIT";
p.type = "module";
p.files = Array.from(new Set([...(p.files || []), "chordmap_bg.wasm", "chordmap.js", "chordmap.d.ts", "chordmap_bg.wasm.d.ts", "LICENSE", "README.md"]));
fs.writeFileSync("pkg/package.json", JSON.stringify(p, null, 2) + "\n");
JS
mkdir -p examples/web/pkg
cp pkg/chordmap.js pkg/chordmap_bg.wasm examples/web/pkg/
echo "built pkg/ ($(wc -c < pkg/chordmap_bg.wasm) bytes of wasm)"
