// Waveform timeline on a canvas: peaks tinted by section, beat ticks,
// played part bright, playhead. Falls back to coloured blocks when there is
// no audio (a saved or shared chart).

const COLS = 1600;

export function peaksOf(samples) {
  const n = samples.length, per = Math.max(1, Math.floor(n / COLS));
  const out = new Float32Array(COLS);
  for (let c = 0; c < COLS; c++) {
    let m = 0;
    const a = c * per, b = Math.min(n, a + per);
    for (let i = a; i < b; i += 2) { const v = Math.abs(samples[i]); if (v > m) m = v; }
    out[c] = m;
  }
  // Light compression so quiet verses still show.
  for (let c = 0; c < COLS; c++) out[c] = Math.pow(out[c], 0.6);
  return out;
}

export function draw(canvas, { peaks, analysis, time, colorOf, ink, muted }) {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (!w || !h) return;
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); }
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  const dur = analysis.duration, px = (t) => (t / dur) * w;
  const played = px(time);
  // Section bands.
  for (const s of analysis.sections) {
    const x0 = px(s.start), x1 = px(s.end);
    ctx.fillStyle = colorOf(s.label); ctx.globalAlpha = 0.16;
    ctx.fillRect(x0, 0, x1 - x0, h);
    ctx.globalAlpha = 0.9; ctx.fillStyle = colorOf(s.label);
    ctx.fillRect(x0, h - 3, x1 - x0, 3);
    ctx.globalAlpha = 1; ctx.fillStyle = ink; ctx.font = "700 11px system-ui, sans-serif";
    if (x1 - x0 > 18) ctx.fillText(s.label, x0 + 6, 13);
  }
  // Beat ticks, downbeats taller.
  ctx.globalAlpha = 0.35; ctx.fillStyle = muted;
  const down = new Set(analysis.downbeats.map((t) => t.toFixed(3)));
  for (const b of analysis.beats) { const x = px(b); const tall = down.has(b.toFixed(3)); ctx.fillRect(x, h - (tall ? 12 : 7), 1, tall ? 9 : 4); }
  ctx.globalAlpha = 1;
  // Waveform.
  if (peaks) {
    const mid = h / 2 - 3, amp = (h / 2) - 10;
    for (let x = 0; x < w; x++) {
      const t = (x / w) * dur;
      const c = Math.min(COLS - 1, Math.floor((x / w) * COLS));
      const v = peaks[c] * amp;
      const sec = analysis.sections.find((s) => t >= s.start && t < s.end) || analysis.sections[analysis.sections.length - 1];
      ctx.fillStyle = sec ? colorOf(sec.label) : ink;
      ctx.globalAlpha = x <= played ? 0.95 : 0.45;
      ctx.fillRect(x, mid - v, 1, Math.max(1, v * 2));
    }
    ctx.globalAlpha = 1;
  } else {
    ctx.fillStyle = ink; ctx.globalAlpha = 0.08; ctx.fillRect(0, h / 2 - 4, w, 4); ctx.globalAlpha = 1;
  }
  // Playhead.
  ctx.fillStyle = ink; ctx.fillRect(played - 1, 0, 2, h);
  ctx.beginPath(); ctx.arc(played, 4, 4, 0, Math.PI * 2); ctx.fill();
}
