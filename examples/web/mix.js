// Practice mix: pick a source (original, centre-cancel instrumental or
// vocals, or separated stems with faders), shift the pitch, and hand the
// player a fresh WAV. Everything is float math on decoded samples.

export function encodeWav(channels, sampleRate) {
  const n = channels[0].length, nc = channels.length;
  const buf = new ArrayBuffer(44 + n * nc * 2), v = new DataView(buf);
  const w = (o, s) => { for (let i = 0; i < s.length; i++) v.setUint8(o + i, s.charCodeAt(i)); };
  w(0, "RIFF"); v.setUint32(4, 36 + n * nc * 2, true); w(8, "WAVE"); w(12, "fmt ");
  v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, nc, true); v.setUint32(24, sampleRate, true);
  v.setUint32(28, sampleRate * nc * 2, true); v.setUint16(32, nc * 2, true); v.setUint16(34, 16, true); w(36, "data"); v.setUint32(40, n * nc * 2, true);
  let o = 44;
  for (let i = 0; i < n; i++) for (let c = 0; c < nc; c++) { const s = Math.max(-1, Math.min(1, channels[c][i])); v.setInt16(o, s < 0 ? s * 32768 : s * 32767, true); o += 2; }
  return new Blob([buf], { type: "audio/wav" });
}

/** Sum stems with gains into a stereo pair. `stems` is { name: [L, R] }, `gains` { name: 0..1 }. */
export function mixStems(stems, gains, n) {
  const l = new Float32Array(n), r = new Float32Array(n);
  for (const [name, ch] of Object.entries(stems)) {
    const g = gains[name] ?? 1; if (!g) continue;
    for (let i = 0; i < n; i++) { l[i] += ch[0][i] * g; r[i] += ch[1][i] * g; }
  }
  return [l, r];
}

/** Peak-normalise in place if anything clips. */
export function tame(channels) {
  let peak = 0;
  for (const c of channels) for (let i = 0; i < c.length; i++) { const a = Math.abs(c[i]); if (a > peak) peak = a; }
  if (peak > 0.99) { const g = 0.99 / peak; for (const c of channels) for (let i = 0; i < c.length; i++) c[i] *= g; }
  return channels;
}
