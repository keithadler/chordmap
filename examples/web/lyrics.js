// Lyrics from the vocal stem, transcribed on this device with Whisper
// (OpenAI's model, Apache-2.0, in the Xenova ONNX export) through
// transformers.js. Library and model are fetched only when asked, from this
// site's own publisher, and cached. Nothing is uploaded.

const TJS_DIR = new URL("./vendor/tjs/", location.href).href;
const LOCAL = location.hostname === "localhost" || location.hostname === "127.0.0.1";
const MODEL_BASE = new URL(LOCAL ? "./local/models/" : "https://keithadler.github.io/chordmap-models/", location.href).href;
export const MODEL = { id: "whisper-base.en", mb: 76 };

let libPromise = null, pipePromise = null;
async function lib() {
  if (!libPromise) {
    libPromise = import(TJS_DIR + "transformers.min.js").then((m) => {
      const env = m.env;
      env.allowLocalModels = false;
      env.allowRemoteModels = true;
      env.remoteHost = MODEL_BASE;
      env.remotePathTemplate = "{model}/";
      env.useBrowserCache = true;
      env.backends.onnx.wasm.wasmPaths = TJS_DIR;
      env.backends.onnx.wasm.numThreads = 1;
      return m;
    });
  }
  return libPromise;
}

async function transcriber(onProgress) {
  if (pipePromise) return pipePromise;
  pipePromise = (async () => {
    const { pipeline } = await lib();
    const files = new Map();
    const progress_callback = (p) => {
      if (p.status === "progress" && p.file) files.set(p.file, p.progress || 0);
      if (onProgress && files.size) { let s = 0; for (const v of files.values()) s += v; onProgress({ stage: "download", p: s / files.size / 100 }); }
    };
    const opts = { dtype: "q8", progress_callback };
    try {
      if (navigator.gpu) return await pipeline("automatic-speech-recognition", MODEL.id, { ...opts, device: "webgpu" });
    } catch (e) { /* fall through to wasm */ }
    return await pipeline("automatic-speech-recognition", MODEL.id, { ...opts, device: "wasm" });
  })();
  pipePromise.catch(() => { pipePromise = null; });
  return pipePromise;
}

/** Mono 16 kHz from any stereo pair and rate. */
export async function to16k(left, right, sampleRate) {
  const n = left.length, len = Math.ceil(n * 16000 / sampleRate);
  const ctx = new OfflineAudioContext(1, len, 16000);
  const b = ctx.createBuffer(2, n, sampleRate); b.copyToChannel(left, 0); b.copyToChannel(right || left, 1);
  const src = ctx.createBufferSource(); src.buffer = b; src.connect(ctx.destination); src.start();
  return (await ctx.startRendering()).getChannelData(0);
}

/**
 * Words with times, from mono 16 kHz samples. The audio is fed to the
 * model in 30-second windows with a two-second overlap so progress is real
 * and a long song never sits in one call. Returns [{ text, start, end }].
 */
export async function transcribe(audio16k, onProgress) {
  const asr = await transcriber(onProgress);
  if (onProgress) onProgress({ stage: "run", p: 0 });
  const SR = 16000, WIN = 30, OVER = 2;
  const total = audio16k.length / SR;
  const step = WIN - OVER;
  const n = Math.max(1, Math.ceil((total - OVER) / step));
  const words = [];
  let lastEnd = -1;
  for (let k = 0; k < n; k++) {
    const t0 = k * step, t1 = Math.min(total, t0 + WIN);
    const seg = audio16k.subarray(Math.floor(t0 * SR), Math.floor(t1 * SR));
    if (seg.length < SR * 0.5) break;
    let out;
    try { out = await asr(seg, { return_timestamps: "word" }); } catch (e) { out = { chunks: [] }; }
    for (const c of out.chunks || []) {
      const text = (c.text || "").trim();
      if (!text || !c.timestamp) continue;
      let start = +c.timestamp[0], end = c.timestamp[1] == null ? start + 0.3 : +c.timestamp[1];
      if (!isFinite(start)) continue;
      start += t0; end += t0;
      // The first seconds of a window were already covered by the previous one.
      if (start <= lastEnd + 0.05) continue;
      words.push({ text, start: +start.toFixed(2), end: +Math.max(start + 0.05, end).toFixed(2) });
      lastEnd = start;
    }
    if (onProgress) onProgress({ stage: "run", p: (k + 1) / n });
  }
  return words;
}

/** Words whose start falls inside [t0, t1). */
export function wordsBetween(words, t0, t1) {
  const out = [];
  for (const w of words || []) { if (w.start >= t1) break; if (w.start >= t0) out.push(w); }
  return out;
}
