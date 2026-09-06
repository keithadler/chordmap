// Neural stem separation in the browser with onnxruntime-web and the
// KUIELab MDX-Net models (MIT). The runtime and the models are fetched only
// when the user asks, from this site's own origin, and cached for next time.
// The spectrogram transforms run in the Rust worker; the network runs here.

// Each model's frame count comes from its own input shape; these are the
// fallbacks when the runtime does not expose it.
export const MODELS = {
  vocals: { file: "kuielab_b_vocals.onnx", mb: 28, dimT: 256 },
  drums: { file: "kuielab_b_drums.onnx", mb: 20, dimT: 128 },
  bass: { file: "kuielab_b_bass.onnx", mb: 28, dimT: 256 },
  other: { file: "kuielab_b_other.onnx", mb: 28, dimT: 256 },
};
function specFor(name, s) {
  let dimF = 2048, dimT = MODELS[name].dimT;
  try {
    const meta = s.inputMetadata && s.inputMetadata[0];
    const shape = meta && (meta.shape || meta.dims);
    if (shape && shape.length === 4) { if (typeof shape[2] === "number") dimF = shape[2]; if (typeof shape[3] === "number") dimT = shape[3]; }
  } catch (e) { /* keep fallbacks */ }
  return { nFft: 6144, hop: 1024, dimF, dimT };
}
export const MODEL_RATE = 44100;
const ORT_DIR = new URL("./vendor/ort/", location.href).href;
const MODEL_BASE = location.hostname === "localhost" || location.hostname === "127.0.0.1" ? "./local/models/" : "https://keithadler.github.io/chordmap-models/";

let ortReady = null;
function loadOrt() {
  if (ortReady) return ortReady;
  ortReady = new Promise((resolve, reject) => {
    if (window.ort) return resolve(window.ort);
    const s = document.createElement("script");
    s.src = ORT_DIR + "ort.all.min.js";
    s.onload = () => {
      const ort = window.ort;
      ort.env.wasm.wasmPaths = ORT_DIR;
      ort.env.wasm.numThreads = 1; // GitHub Pages cannot send the isolation headers threads need
      resolve(ort);
    };
    s.onerror = () => reject(new Error("onnxruntime failed to load"));
    document.head.appendChild(s);
  });
  return ortReady;
}

async function fetchCached(url, onProgress) {
  let cache = null;
  try { cache = await caches.open("chordmap-models"); const hit = await cache.match(url); if (hit) return await hit.arrayBuffer(); } catch (e) { /* no cache api */ }
  const res = await fetch(url);
  if (!res.ok) throw new Error("model download failed (" + res.status + ")");
  const total = +res.headers.get("content-length") || 0;
  const reader = res.body.getReader(); const parts = []; let got = 0;
  for (;;) { const { done, value } = await reader.read(); if (done) break; parts.push(value); got += value.length; if (onProgress) onProgress(total ? got / total : 0); }
  const buf = new Uint8Array(got); let o = 0; for (const p of parts) { buf.set(p, o); o += p.length; }
  try { if (cache) await cache.put(url, new Response(buf, { headers: { "content-type": "application/octet-stream" } })); } catch (e) { /* quota */ }
  return buf.buffer;
}

const sessions = new Map();
async function session(name, onProgress) {
  if (sessions.has(name)) return sessions.get(name);
  const ort = await loadOrt();
  const bytes = await fetchCached(MODEL_BASE + MODELS[name].file, onProgress);
  let s;
  try { s = await ort.InferenceSession.create(bytes, { executionProviders: ["webgpu", "wasm"] }); }
  catch (e) { s = await ort.InferenceSession.create(bytes, { executionProviders: ["wasm"] }); }
  sessions.set(name, s);
  return s;
}

/** Run one model over a stereo mix at 44.1 kHz. `call(op, payload, transfer)` talks to the Rust worker. */
async function runModel(name, left, right, call, onProgress) {
  const ort = await loadOrt();
  const s = await session(name, (p) => onProgress && onProgress({ stage: "download", stem: name, p }));
  const SPEC = specFor(name, s);
  const n = left.length, chunk = SPEC.hop * (SPEC.dimT - 1), trim = SPEC.nFft / 2, gen = chunk - 2 * trim;
  const pad = gen - (n % gen);
  const padded = (x) => { const y = new Float32Array(trim + n + pad + trim); y.set(x, trim); return y; };
  const pl = padded(left), pr = padded(right);
  const outL = new Float32Array(n), outR = new Float32Array(n);
  const total = Math.ceil((n + pad) / gen);
  for (let i = 0, k = 0; i < n + pad; i += gen, k++) {
    const cl = pl.slice(i, i + chunk), cr = pr.slice(i, i + chunk);
    const spec = await call("mdxStft", { spec: SPEC, left: cl, right: cr }, [cl.buffer, cr.buffer]);
    const input = new ort.Tensor("float32", spec.data, [1, 4, SPEC.dimF, SPEC.dimT]);
    const out = await s.run({ [s.inputNames[0]]: input });
    const data = out[s.outputNames[0]].data;
    const wave = await call("mdxIstft", { spec: SPEC, data: new Float32Array(data) }, []);
    // Keep the middle of the chunk; edges are transform artefacts.
    const start = i, count = Math.min(gen, n - start);
    for (let j = 0; j < count; j++) { outL[start + j] = wave.left[trim + j]; outR[start + j] = wave.right[trim + j]; }
    if (onProgress) onProgress({ stage: "run", stem: name, p: (k + 1) / total });
  }
  return [outL, outR];
}

/**
 * Separate `stems` (subset of vocals, drums, bass, other) from a stereo mix.
 * Returns { stemName: [L, R] } plus `instrumental` = mix minus vocals.
 */
export async function separate(left, right, stems, call, onProgress) {
  const out = {};
  for (const name of stems) out[name] = await runModel(name, left, right, call, onProgress);
  if (out.vocals) {
    const n = left.length, il = new Float32Array(n), ir = new Float32Array(n);
    for (let i = 0; i < n; i++) { il[i] = left[i] - out.vocals[0][i]; ir[i] = right[i] - out.vocals[1][i]; }
    out.instrumental = [il, ir];
  }
  return out;
}

export function supportsWebGPU() { return !!navigator.gpu; }
