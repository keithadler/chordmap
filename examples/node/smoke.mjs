// Node smoke test for the wasm package: a synthesized click track at 120 BPM.
import { readFileSync } from "node:fs";
import init, { analyze, version } from "../../pkg/chordmap.js";

await init({ module_or_path: readFileSync(new URL("../../pkg/chordmap_bg.wasm", import.meta.url)) });
const sr = 22050, seconds = 20, bpm = 120;
const x = new Float32Array(sr * seconds);
let seed = 7;
const rnd = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x40000000 - 1; };
for (let k = 0; k * 60 / bpm < seconds; k++) {
  const start = Math.floor(k * 60 / bpm * sr);
  for (let i = 0; i < 0.02 * sr && start + i < x.length; i++) x[start + i] += rnd() * Math.exp(-i / sr * 300);
}
const a = JSON.parse(analyze(x, sr, "{}"));
const ok = [bpm, bpm / 2, bpm * 2].some((b) => Math.abs(a.tempo.bpm / b - 1) < 0.02);
if (!ok) { console.error("unexpected tempo", a.tempo); process.exit(1); }
console.log(`chordmap ${version()}: ${a.tempo.bpm} BPM, ${a.beats.length} beats, ${a.sections.length} section(s)`);
