// Full-screen visualizers driven by the chart and the live audio.
//   Stage  : for the players. Huge current chord, the next one with a beat
//            countdown, beat dots, section progress. Readable across a room.
//   Flow   : for both. Chords slide toward a now line, sized by duration,
//            coloured by section, beats as grid lines.
//   Aurora : for the audience. Pitch-class bloom and spectrum ring fed by an
//            AnalyserNode, beat rings, section colour washes, chord small.

const SHARP = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const TONES = { "": [0, 4, 7], "m": [0, 3, 7], "7": [0, 4, 7, 10], "maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10] };
const FIFTHS_HUE = (pc) => ((pc * 7) % 12) / 12 * 360; // circle of fifths around the colour wheel

let audioCtx = null, source = null, analyser = null, freq = null;
function tapAudio(player) {
  // A media element can be routed into an AudioContext only once; keep it.
  try {
    if (!audioCtx) {
      audioCtx = new (window.AudioContext || window.webkitAudioContext)();
      source = audioCtx.createMediaElementSource(player);
      analyser = audioCtx.createAnalyser(); analyser.fftSize = 4096; analyser.smoothingTimeConstant = 0.7;
      source.connect(analyser); analyser.connect(audioCtx.destination);
      freq = new Float32Array(analyser.frequencyBinCount);
    }
    if (audioCtx.state === "suspended") audioCtx.resume();
    return true;
  } catch (e) { return false; }
}

export function openVisualizer(opts) {
  const { analysis: a, player, display, colorOf, parseLabel, onClose } = opts;
  const displaySounding = opts.displaySounding || display, transpose = opts.transpose || (() => 0);
  const root = document.createElement("div"); root.className = "viz"; root.id = "viz";
  root.innerHTML = `<canvas></canvas>
    <div class="viz-bar">
      <span class="seg"><button data-s="stage" class="on">Stage</button><button data-s="flow">Flow</button><button data-s="aurora">Aurora</button></span>
      <button data-a="play" title="Space">Play / Pause</button>
      <button data-a="full" title="F">Fullscreen</button>
      <button data-a="close" title="Esc">Close</button>
    </div>
    <div class="viz-hint">1 2 3 switch · space play · ← → bars · F fullscreen · Esc close</div>`;
  document.body.appendChild(root);
  document.body.classList.add("viz-open");
  const canvas = root.querySelector("canvas"), ctx = canvas.getContext("2d");
  let style = "stage", raf = 0, lastBeat = -1, pulse = 0, hideTimer = 0, energy = 0, onset = 0;
  const chroma = new Float32Array(12), rings = [], particles = [];
  const hasAudio = !!player.src && tapAudio(player);

  const css = (n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
  const ink = () => css("--ink"), muted = () => css("--muted"), accent = () => css("--accent");
  const sectionAt = (t) => a.sections.find((s) => t >= s.start && t < s.end) || a.sections[a.sections.length - 1];
  const chordIndex = (t) => { let i = a.chords.findIndex((c) => t >= c.start && t < c.end); if (i < 0) i = t < a.chords[0].start ? -1 : a.chords.length - 1; return i; };
  const beatIndex = (t) => { let bi = -1; for (let k = 0; k < a.beats.length; k++) { if (a.beats[k] <= t) bi = k; else break; } return bi; };
  const bpb = a.beatsPerBar || 4;

  function size() {
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(root.clientWidth * dpr); canvas.height = Math.round(root.clientHeight * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }
  size();
  const ro = new ResizeObserver(size); ro.observe(root);

  function readAudio() {
    if (!analyser) return;
    analyser.getFloatFrequencyData(freq);
    const sr = audioCtx.sampleRate, n = freq.length, binHz = sr / 2 / n;
    const acc = new Float32Array(12); let total = 0, hi = 0;
    for (let b = 1; b < n; b++) {
      const f = b * binHz; if (f < 50) continue; if (f > 8000) break;
      const p = Math.pow(10, freq[b] / 20); // magnitude
      total += p; if (f > 2500) hi += p;
      if (f <= 2200) { const pc = ((Math.round(69 + 12 * Math.log2(f / 440)) % 12) + 12) % 12; acc[pc] += p * (f < 250 ? 1.6 : 1); }
    }
    let max = 1e-6; for (let k = 0; k < 12; k++) max = Math.max(max, acc[k]);
    // Compress so chord tones bloom instead of one bass note taking it all.
    for (let k = 0; k < 12; k++) chroma[k] = chroma[k] * 0.7 + Math.max(0, 1 + 20 * Math.log10(acc[k] / max + 1e-9) / 30) * 0.3;
    const e = Math.min(1, total / 3);
    onset = Math.max(0, e - energy) * 4;
    energy = energy * 0.85 + e * 0.15;
  }

  // Shared header and footer strips.
  function frame(t, w, h) {
    const s = sectionAt(t), col = s ? colorOf(s.label) : accent();
    // Section strip along the bottom with the playhead.
    const y = h - 6;
    for (const sec of a.sections) {
      ctx.fillStyle = colorOf(sec.label); ctx.globalAlpha = sec === s ? 1 : 0.35;
      ctx.fillRect(sec.start / a.duration * w, y, (sec.end - sec.start) / a.duration * w, 6);
    }
    ctx.globalAlpha = 1; ctx.fillStyle = ink(); ctx.fillRect(t / a.duration * w - 1, y - 4, 2, 10);
    return { s, col };
  }
  function text(str, x, y, sizePx, weight, color, align) {
    ctx.font = `${weight} ${sizePx}px -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Roboto, Inter, system-ui, sans-serif`;
    ctx.fillStyle = color; ctx.textAlign = align || "center"; ctx.textBaseline = "middle"; ctx.fillText(str, x, y);
  }

  function drawStage(t, w, h) {
    const { s, col } = frame(t, w, h);
    const i = chordIndex(t), cur = a.chords[i], next = a.chords[i + 1];
    const spb = 60 / a.tempo.bpm;
    // Background wash from the section colour.
    const g = ctx.createRadialGradient(w / 2, h * 0.45, 0, w / 2, h * 0.45, Math.max(w, h) * 0.7);
    g.addColorStop(0, hexA(col, 0.22 + pulse * 0.12)); g.addColorStop(1, hexA(col, 0));
    ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
    // Section name and progress through it.
    // Below the toolbar so nothing collides on a narrow screen.
    if (s) {
      text(`${s.label} · ${s.guess}`, 32, 96, 22, 700, muted(), "left");
      const p = (t - s.start) / (s.end - s.start);
      ctx.fillStyle = hexA(col, 0.25); ctx.fillRect(32, 118, w - 64, 4);
      ctx.fillStyle = col; ctx.fillRect(32, 118, (w - 64) * Math.max(0, Math.min(1, p)), 4);
    }
    text(`${a.tempo.bpm} BPM · ${a.meter} · ${a.key.name}`, w - 32, 96, 20, 600, muted(), "right");
    // The chord.
    const big = Math.min(w * 0.3, h * 0.42) * (1 + pulse * 0.05);
    text(cur ? display(cur.label) : "…", w / 2, h * 0.44, big, 800, ink());
    // Next chord and countdown.
    if (next) {
      const beats = Math.max(1, Math.ceil((next.start - t) / spb));
      text(`next ${display(next.label)}`, w / 2, h * 0.44 + big * 0.62, Math.min(w * 0.08, h * 0.1), 700, hexA(col, 0.95));
      text(`in ${beats}`, w / 2, h * 0.44 + big * 0.62 + Math.min(w * 0.08, h * 0.1) * 0.9, Math.min(w * 0.04, h * 0.05), 600, muted());
    }
    // Beat dots.
    const bi = beatIndex(t), pickup = a.bars[0] ? a.bars[0].beats.length % bpb : 0;
    const inBar = bi < 0 ? 0 : ((bi - pickup) % bpb + bpb) % bpb;
    const r = Math.min(w, h) * 0.018, gap = r * 3.2, x0 = w / 2 - (bpb - 1) * gap / 2, y = h * 0.86;
    for (let k = 0; k < bpb; k++) {
      ctx.beginPath(); ctx.arc(x0 + k * gap, y, k === inBar ? r * (1.25 + pulse * 0.5) : r, 0, Math.PI * 2);
      ctx.fillStyle = k === inBar ? col : hexA(ink(), 0.25); ctx.fill();
    }
  }

  function drawFlow(t, w, h) {
    const { s, col } = frame(t, w, h);
    const nowX = w * 0.28, pxPerSec = Math.max(60, w / 12), lane = h * 0.5, laneH = h * 0.26;
    const g = ctx.createLinearGradient(0, 0, 0, h); g.addColorStop(0, hexA(col, 0.18)); g.addColorStop(1, hexA(col, 0.02));
    ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
    const x = (time) => nowX + (time - t) * pxPerSec;
    // Beat grid.
    for (const b of a.beats) { const bx = x(b); if (bx < -2 || bx > w + 2) continue; ctx.fillStyle = hexA(ink(), 0.12); ctx.fillRect(bx, lane - laneH * 0.8, 1, laneH * 1.6); }
    for (const d of a.downbeats) { const bx = x(d); if (bx < -2 || bx > w + 2) continue; ctx.fillStyle = hexA(ink(), 0.35); ctx.fillRect(bx, lane - laneH * 0.9, 2, laneH * 1.8); }
    // Chord blocks.
    for (const c of a.chords) {
      const x0 = x(c.start), x1 = x(c.end); if (x1 < 0 || x0 > w) continue;
      const sec = sectionAt(c.start), sc = sec ? colorOf(sec.label) : col;
      const active = t >= c.start && t < c.end, past = c.end <= t;
      const lift = active ? laneH * 0.06 * (1 + pulse) : 0;
      roundRect(x0 + 3, lane - laneH / 2 - lift, Math.max(6, x1 - x0 - 6), laneH, 14);
      ctx.fillStyle = hexA(sc, past ? 0.25 : active ? 0.95 : 0.6); ctx.fill();
      if (x1 - x0 > 40) text(display(c.label), (Math.max(x0, 0) + Math.min(x1, w)) / 2, lane - lift, Math.min(laneH * 0.5, (x1 - x0) * 0.4), 800, past ? hexA(ink(), 0.5) : "#fff");
    }
    // Now line and header.
    ctx.fillStyle = ink(); ctx.fillRect(nowX - 1.5, lane - laneH, 3, laneH * 2);
    const cur = a.chords[chordIndex(t)];
    text(cur ? display(cur.label) : "…", 32, Math.max(h * 0.16, 120), Math.min(w * 0.1, h * 0.16), 800, ink(), "left");
    if (s) text(`${s.label} · ${s.guess}   ${a.tempo.bpm} BPM`, w - 32, Math.max(h * 0.16, 120), 20, 600, muted(), "right");
  }

  function drawAurora(t, w, h) {
    const { s, col } = frame(t, w, h);
    const cx = w / 2, cy = h / 2, R = Math.min(w, h) * 0.3;
    // Slow colour wash and vignette.
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(w, h) * 0.75);
    g.addColorStop(0, hexA(col, 0.16 + energy * 0.2)); g.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
    const cur = a.chords[chordIndex(t)], pc = cur ? parseLabel(cur.label) : null;
    const tones = pc ? (TONES[pc.suffix] || TONES[""]).map((iv) => (pc.pc + iv + transpose() + 120) % 12) : [];
    // Spectrum ring.
    if (analyser) {
      const n = 96, inner = R * 1.12;
      for (let k = 0; k < n; k++) {
        const b = Math.floor(Math.pow(k / n, 2) * freq.length * 0.5) + 1;
        const v = Math.max(0, (freq[b] + 90) / 70);
        const ang = k / n * Math.PI * 2 - Math.PI / 2, len = 4 + v * R * 0.35;
        ctx.strokeStyle = hexA(col, 0.25 + v * 0.6); ctx.lineWidth = 3; ctx.lineCap = "round";
        ctx.beginPath(); ctx.moveTo(cx + Math.cos(ang) * inner, cy + Math.sin(ang) * inner); ctx.lineTo(cx + Math.cos(ang) * (inner + len), cy + Math.sin(ang) * (inner + len)); ctx.stroke();
      }
    }
    // Twelve petals around the circle of fifths, length from live chroma.
    for (let k = 0; k < 12; k++) {
      const pcK = (k * 7) % 12, ang = k / 12 * Math.PI * 2 - Math.PI / 2;
      const inChord = tones.includes(pcK);
      const len = R * (0.25 + chroma[pcK] * 0.9 + (inChord ? 0.15 : 0) + pulse * 0.08);
      const hue = FIFTHS_HUE(pcK);
      ctx.save(); ctx.translate(cx, cy); ctx.rotate(ang);
      // Gradient in the rotated frame: from the centre out along the petal.
      const grad = ctx.createLinearGradient(0, 0, len, 0);
      grad.addColorStop(0, `hsla(${hue}, 90%, 60%, 0)`); grad.addColorStop(0.55, `hsla(${hue}, 90%, ${inChord ? 68 : 52}%, ${inChord ? 0.95 : 0.4})`); grad.addColorStop(1, `hsla(${hue}, 95%, 75%, 0)`);
      ctx.beginPath(); ctx.ellipse(len / 2, 0, len / 2, R * (0.06 + chroma[pcK] * 0.08), 0, 0, Math.PI * 2);
      ctx.fillStyle = grad; ctx.shadowBlur = 40; ctx.shadowColor = `hsla(${hue}, 90%, 60%, .6)`; ctx.fill();
      ctx.restore();
    }
    for (let k = 0; k < 12; k++) {
      const pcK = (k * 7) % 12, ang = k / 12 * Math.PI * 2 - Math.PI / 2, inChord = tones.includes(pcK), hue = FIFTHS_HUE(pcK);
      text(SHARP[pcK].replace("#", "♯"), cx + Math.cos(ang) * (R * 1.62), cy + Math.sin(ang) * (R * 1.62), 15, 800, inChord ? `hsl(${hue}, 90%, 75%)` : hexA(muted(), 0.7));
    }
    // Beat rings expanding outward.
    for (const r of rings) { ctx.beginPath(); ctx.arc(cx, cy, R * (0.5 + r.age * 1.6), 0, Math.PI * 2); ctx.strokeStyle = hexA(col, 0.5 * (1 - r.age)); ctx.lineWidth = 2 + (1 - r.age) * 4; ctx.stroke(); }
    // Onset sparks.
    for (const p of particles) { ctx.beginPath(); ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2); ctx.fillStyle = hexA(p.c, p.life); ctx.fill(); }
    // The chord, small and calm in the centre.
    text(cur ? displaySounding(cur.label) : "", cx, cy, Math.min(w, h) * 0.11, 800, ink());
    if (s) text(`${s.label} · ${s.guess}`, cx, cy + Math.min(w, h) * 0.09, 16, 600, muted());
  }

  function roundRect(x, y, w, h, r) { ctx.beginPath(); ctx.moveTo(x + r, y); ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r); ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath(); }
  function hexA(c, alpha) {
    // "#rrggbb" or "rgb(...)" to rgba with alpha.
    if (c.startsWith("#")) { const n = parseInt(c.slice(1), 16); return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`; }
    const m = c.match(/[\d.]+/g); return m ? `rgba(${m[0]},${m[1]},${m[2]},${alpha})` : c;
  }

  // Debug hook for tests: live chroma and energy.
  window.__vizDebug = () => ({ chroma: Array.from(chroma).map((v) => +v.toFixed(2)), energy: +energy.toFixed(3), analyser: !!analyser, ctxState: audioCtx && audioCtx.state });
  let last = performance.now();
  function loop(now) {
    const dt = Math.min(0.05, (now - last) / 1000); last = now;
    const t = player.currentTime, w = root.clientWidth, h = root.clientHeight;
    if (!player.paused) readAudio();
    const bi = beatIndex(t);
    if (bi !== lastBeat) { lastBeat = bi; pulse = 1; rings.push({ age: 0 }); }
    pulse = Math.max(0, pulse - dt * 4);
    for (const r of rings) r.age += dt * 0.9; while (rings.length && rings[0].age >= 1) rings.shift();
    if (onset > 0.25 && particles.length < 120) { const s = sectionAt(t); for (let k = 0; k < 6; k++) { const ang = Math.random() * Math.PI * 2, sp = 60 + Math.random() * 160; particles.push({ x: w / 2, y: h / 2, vx: Math.cos(ang) * sp, vy: Math.sin(ang) * sp, r: 2 + Math.random() * 3, life: 1, c: s ? colorOf(s.label) : accent() }); } }
    for (const p of particles) { p.x += p.vx * dt; p.y += p.vy * dt; p.life -= dt * 0.8; }
    for (let k = particles.length - 1; k >= 0; k--) if (particles[k].life <= 0) particles.splice(k, 1);
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = css("--bg"); ctx.fillRect(0, 0, w, h);
    (style === "stage" ? drawStage : style === "flow" ? drawFlow : drawAurora)(t, w, h);
    raf = requestAnimationFrame(loop);
  }
  raf = requestAnimationFrame(loop);

  function setStyle(s) { style = s; root.querySelectorAll("[data-s]").forEach((b) => b.classList.toggle("on", b.dataset.s === s)); try { localStorage.setItem("chordmap.viz", s); } catch (e) { /* private mode */ } }
  try { const saved = localStorage.getItem("chordmap.viz"); if (saved) setStyle(saved); } catch (e) { /* private mode */ }
  function close() {
    cancelAnimationFrame(raf); ro.disconnect();
    document.removeEventListener("keydown", keys); root.removeEventListener("mousemove", wake);
    if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
    root.remove(); document.body.classList.remove("viz-open");
    if (onClose) onClose();
  }
  function keys(e) {
    if (e.code === "Escape") { e.preventDefault(); close(); }
    else if (e.code === "Space") { e.preventDefault(); player.paused ? player.play() : player.pause(); }
    else if (e.key === "1") setStyle("stage"); else if (e.key === "2") setStyle("flow"); else if (e.key === "3") setStyle("aurora");
    else if (e.key === "f" || e.key === "F") toggleFull();
    else if (e.code === "ArrowLeft" || e.code === "ArrowRight") {
      const bars = a.bars, t = player.currentTime; let i = bars.findIndex((b) => t >= b.start && t < b.end); if (i < 0) i = 0;
      const j = e.code === "ArrowRight" ? Math.min(bars.length - 1, i + 1) : (t - bars[i].start > 1 ? i : Math.max(0, i - 1));
      player.currentTime = bars[j].start;
    }
  }
  function toggleFull() { if (document.fullscreenElement) document.exitFullscreen().catch(() => {}); else root.requestFullscreen && root.requestFullscreen().catch(() => {}); }
  function wake() { root.classList.remove("idle"); clearTimeout(hideTimer); hideTimer = setTimeout(() => root.classList.add("idle"), 2500); }
  document.addEventListener("keydown", keys);
  root.addEventListener("mousemove", wake); wake();
  root.querySelector(".viz-bar").addEventListener("click", (e) => {
    const b = e.target.closest("button"); if (!b) return;
    if (b.dataset.s) setStyle(b.dataset.s);
    else if (b.dataset.a === "play") player.paused ? player.play() : player.pause();
    else if (b.dataset.a === "full") toggleFull();
    else if (b.dataset.a === "close") close();
  });
  if (!hasAudio) root.querySelector(".viz-hint").textContent = "No audio for this chart: Aurora needs the song playing. " + root.querySelector(".viz-hint").textContent;
  return { close, setStyle };
}
