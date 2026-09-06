// Runs the WebAssembly engine off the main thread. Every message carries an
// `op`; the result is posted back with the same `id`. Nothing here touches
// the network.
import init, { analyze, chordSheet, centerSplit, pitchShift, mdxStft, mdxIstft } from "./pkg/chordmap.js?v=dev";

// The wasm URL carries the build stamp too, so a new deploy is never served
// a stale binary from the browser or the CDN cache.
let ready = init({ module_or_path: new URL("./pkg/chordmap_bg.wasm?v=dev", import.meta.url) });

self.onmessage = async (e) => {
  const { id, op } = e.data;
  try {
    await ready;
    let result, transfer = [];
    if (!op || op === "analyze") {
      const { samples, sampleRate, options } = e.data;
      const json = analyze(samples, sampleRate, JSON.stringify(options || {}));
      result = { json, sheet: chordSheet(json) };
    } else if (op === "centerSplit") {
      const { left, right, sampleRate, strength } = e.data;
      const out = centerSplit(left, right, sampleRate, strength ?? 1);
      const n = left.length;
      const part = (k) => out.slice(k * n, (k + 1) * n);
      result = { instL: part(0), instR: part(1), vocL: part(2), vocR: part(3) };
      transfer = [result.instL.buffer, result.instR.buffer, result.vocL.buffer, result.vocR.buffer];
    } else if (op === "pitchShift") {
      const { channels, sampleRate, semitones } = e.data;
      const out = channels.map((c) => pitchShift(c, sampleRate, semitones));
      result = { channels: out };
      transfer = out.map((c) => c.buffer);
    } else if (op === "mdxStft") {
      const { spec, left, right } = e.data;
      const out = mdxStft(spec.nFft, spec.hop, spec.dimF, spec.dimT, left, right);
      result = { data: out }; transfer = [out.buffer];
    } else if (op === "mdxIstft") {
      const { spec, data } = e.data;
      const out = mdxIstft(spec.nFft, spec.hop, spec.dimF, spec.dimT, data);
      const n = out.length / 2;
      result = { left: out.slice(0, n), right: out.slice(n) };
      transfer = [result.left.buffer, result.right.buffer];
    } else {
      throw new Error("unknown op " + op);
    }
    self.postMessage({ id, ok: true, ...result }, transfer);
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err && err.message ? err.message : err) });
  }
};
