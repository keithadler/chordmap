// chordmap web app. Decodes audio in the browser, analyzes it in a worker
// running the Rust engine, and draws the chart. No network calls.

import { diagram, ukeDiagram } from "./chords-guitar.js?v=dev";
import * as library from "./library.js?v=dev";
import { peaksOf, draw as drawWave } from "./waveform.js?v=dev";
import { openVisualizer } from "./visualizer.js?v=dev";
import { midiBytes, chordPro, shareLink, readShareLink } from "./export.js?v=dev";

const $ = (id) => document.getElementById(id);
const SHARP = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLAT = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
const COLORS = ["--sA", "--sB", "--sC", "--sD", "--sE", "--sF", "--sG", "--sH"];

const state = {
  file: null, samples: null, sampleRate: 0, url: null,
  analysis: null, sheet: "", bpmHint: null,
  capo: 0, transpose: 0, show: "shapes", spelling: "auto", taps: [],
  loop: null, shared: false, instrument: "guitar", editing: false, fileKey: null, queue: [],
};

// ---------- a chart shared by link: no audio, everything else works
readShareLink().then((shared) => {
  if (!shared || !shared.a) return;
  state.shared = true; state.sharedTitle = shared.t || "Shared chart";
  state.analysis = shared.a; state.capo = shared.a.guitar.capo;
  $("fname").textContent = state.sharedTitle;
  $("fmeta").textContent = fmtTime(shared.a.duration) + " · shared chart, no audio";
  $("home").style.display = "none";
  $("results").classList.add("active");
  document.body.classList.add("shared");
  render();
  mountLive();
});

// ---------- offline (the app shell is cached after the first visit)
if ("serviceWorker" in navigator && location.protocol === "https:") {
  navigator.serviceWorker.register("./sw.js").catch(() => {});
}

// ---------- theme
$("theme").addEventListener("click", () => {
  const dark = document.documentElement.dataset.theme !== "dark";
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  try { localStorage.setItem("chordmap.theme", dark ? "dark" : "light"); } catch (e) { /* private mode */ }
});

// ---------- worker
const worker = new Worker("./worker.js?v=dev", { type: "module" });
let nextId = 1;
const pending = new Map();
worker.onmessage = (e) => {
  const p = pending.get(e.data.id);
  if (!p) return;
  pending.delete(e.data.id);
  e.data.ok ? p.resolve(e.data) : p.reject(new Error(e.data.error));
};
worker.onerror = (e) => { setStatus("The analysis engine failed to load: " + (e.message || "unknown error"), true); };
function runAnalysis(options) {
  const id = nextId++;
  // The worker gets its own copy so the page keeps the samples for re-runs.
  const copy = new Float32Array(state.samples);
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, samples: copy, sampleRate: state.sampleRate, options }, [copy.buffer]);
  });
}

// ---------- input
const drop = $("drop"), picker = $("picker");
drop.addEventListener("click", () => picker.click());
drop.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); picker.click(); } });
picker.addEventListener("change", () => { openFiles([...picker.files]); picker.value = ""; });
// The overlay is shown while dragover events keep arriving and cleared a
// moment after they stop, which also covers a drag that leaves the window.
let dragTimer = null;
["dragenter", "dragover"].forEach((ev) => document.addEventListener(ev, (e) => {
  e.preventDefault();
  if (!(e.dataTransfer && [...e.dataTransfer.types].includes("Files"))) return;
  document.body.classList.add("dragging");
  clearTimeout(dragTimer); dragTimer = setTimeout(() => document.body.classList.remove("dragging"), 250);
}));
["dragleave", "drop"].forEach((ev) => document.addEventListener(ev, (e) => { e.preventDefault(); if (ev === "drop" || e.relatedTarget === null) { clearTimeout(dragTimer); document.body.classList.remove("dragging"); } }));
document.addEventListener("drop", (e) => { const fs = e.dataTransfer ? [...e.dataTransfer.files] : []; if (fs.length) openFiles(fs); });
// Several files at once: each is analyzed and saved to the library, the last one is shown.
async function openFiles(files) {
  const audio = files.filter((f) => /audio|\.(mp3|m4a|wav|flac|ogg|aac|aiff?)$/i.test(f.type + " " + f.name));
  if (!audio.length) { setStatus("That does not look like an audio file.", true); return; }
  for (let i = 0; i < audio.length; i++) {
    state.queue = audio.length > 1 ? [i + 1, audio.length] : [];
    await openFile(audio[i]);
  }
  state.queue = [];
}
$("another").addEventListener("click", () => { showHome(); });
// For integration tests and power users: chordmapOpen(file).
window.chordmapOpen = openFile;
picker.multiple = true;

function setStatus(text, err, busy) {
  const s = $("status"); s.classList.toggle("err", !!err);
  s.innerHTML = (busy ? '<span class="listen" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i></span>' : "") + "<span></span>";
  s.lastChild.textContent = text;
}
function showHome() {
  renderLibrary();
  if (liveViz) { liveViz.close(); liveViz = null; }
  $("results").classList.remove("active");
  $("home").style.display = "";
  setStatus("");
  const p = $("player"); p.pause();
}

async function openFile(file) {
  state.file = file; state.bpmHint = null; state.taps = []; state.transpose = 0; state.loop = null; state.editing = false;
  state.fileKey = library.fileKey(file); state.shared = false; state.peaks = null; document.body.classList.remove("shared");
  $("home").style.display = "";
  $("results").classList.remove("active");
  const q = state.queue.length ? " (" + state.queue[0] + " of " + state.queue[1] + ")" : "";
  setStatus("Decoding " + file.name + q + "…", false, true);
  try {
    const buf = await file.arrayBuffer();
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    const audio = await ctx.decodeAudioData(buf.slice(0));
    ctx.close && ctx.close();
    const n = audio.length, ch = audio.numberOfChannels;
    const mono = new Float32Array(n);
    for (let c = 0; c < ch; c++) {
      const d = audio.getChannelData(c);
      for (let i = 0; i < n; i++) mono[i] += d[i] / ch;
    }
    state.samples = mono; state.sampleRate = audio.sampleRate;
    state.peaks = peaksOf(mono);
    if (state.url) URL.revokeObjectURL(state.url);
    state.url = URL.createObjectURL(file);
    $("player").src = state.url;
    await analyze();
  } catch (err) {
    setStatus("Could not read that file: " + (err && err.message ? err.message : err), true);
  }
}

async function analyze() {
  setStatus("Listening for the beat, the key and the chords…", false, true);
  const t0 = performance.now();
  const options = {};
  if (state.bpmHint) options.bpmHint = state.bpmHint;
  const res = await runAnalysis(options);
  state.analysis = JSON.parse(res.json);
  state.sheet = res.sheet;
  state.capo = state.analysis.guitar.capo;
  // The same file seen before gets its corrections back.
  const saved = state.bpmHint ? null : await library.load(state.fileKey);
  if (saved && saved.analysis && saved.analysis.duration === state.analysis.duration) { state.analysis = saved.analysis; state.capo = state.analysis.guitar.capo; }
  await library.save(state.fileKey, state.file.name, state.analysis);
  renderLibrary();
  const secs = ((performance.now() - t0) / 1000).toFixed(1);
  $("fname").textContent = state.file.name;
  $("fmeta").textContent = fmtTime(state.analysis.duration) + " · analyzed in " + secs + " s on this device" + (state.queue.length ? " · " + state.queue[0] + " of " + state.queue[1] : "");
  $("home").style.display = "none";
  $("results").classList.add("active");
  setStatus("");
  render();
  mountLive();
}

// ---------- helpers
function fmtTime(s) { s = Math.max(0, Math.round(s)); return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0"); }
const SUFFIXES = ["maj7", "m7", "7", "m", ""];
function parseLabel(l) {
  if (l === "N") return null;
  for (const suf of SUFFIXES) {
    if (!l.endsWith(suf)) continue;
    const name = suf ? l.slice(0, -suf.length) : l;
    let pc = SHARP.indexOf(name); if (pc < 0) pc = FLAT.indexOf(name);
    if (pc >= 0) return { pc, minor: suf === "m" || suf === "m7", suffix: suf };
  }
  return null;
}
function keyUsesFlats(tonicPc, minor) { const maj = minor ? (tonicPc + 3) % 12 : tonicPc; return [5, 10, 3, 8, 1, 6].includes(maj); }
// Diatonic roots as the key writes them, chromatic roots as flats, except the raised fourth.
function spell(pc, tonicPc, minor) {
  if (state.spelling === "flat") return FLAT[pc];
  if (state.spelling === "sharp") return SHARP[pc];
  if (keyUsesFlats(tonicPc, minor)) return FLAT[pc];
  const maj = minor ? (tonicPc + 3) % 12 : tonicPc;
  const diatonic = [0, 2, 4, 5, 7, 9, 11].some((d) => (maj + d) % 12 === pc);
  const raisedSeventh = minor && pc === (tonicPc + 11) % 12;
  return diatonic || raisedSeventh || pc === (maj + 6) % 12 ? SHARP[pc] : FLAT[pc];
}
function shift() { return state.show === "shapes" ? state.transpose - state.capo : state.transpose; }
function shownTonicBase() { const a = state.analysis; const k = parseLabel(a.key.tonic + (a.key.minor ? "m" : "")); return k ? k.pc : 0; }
function shownTonic() { return (shownTonicBase() + shift() + 120) % 12; }
const DEGREES = ["1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7"];
const DEGREES_MINOR = ["1", "b2", "2", "3", "#3", "4", "b5", "5", "6", "#6", "7", "#7"];
function display(label) {
  const c = parseLabel(label); if (!c) return "N.C.";
  if (state.show === "numbers") {
    // Nashville numbers relative to the key: 1 4 5 6m, b7 for borrowed chords.
    const a = state.analysis;
    const deg = (c.pc - shownTonicBase() + 120) % 12;
    return (a.key.minor ? DEGREES_MINOR : DEGREES)[deg] + c.suffix;
  }
  const pc = (c.pc + shift() + 120) % 12;
  return spell(pc, shownTonic(), state.analysis.key.minor) + c.suffix;
}
function keyName() {
  const a = state.analysis; const k = parseLabel(a.key.tonic + (a.key.minor ? "m" : ""));
  const pc = ((k ? k.pc : 0) + state.transpose + 120) % 12;
  const name = state.spelling === "flat" ? FLAT[pc] : state.spelling === "sharp" ? SHARP[pc] : (keyUsesFlats(pc, a.key.minor) ? FLAT : SHARP)[pc];
  return name + (a.key.minor ? " minor" : " major");
}

// ---------- render
function render() {
  const a = state.analysis;
  $("bpm").textContent = a.tempo.bpm.toFixed(1);
  const alts = a.tempo.alternatives.map((b) => b.toFixed(0)).join(", ");
  const tuning = Math.abs(a.tuningCents) >= 7.5 ? " Tuned " + (a.tuningCents > 0 ? "+" : "") + a.tuningCents.toFixed(0) + " cents from A440, corrected." : "";
  $("bpm-sub").textContent = a.meter + " time. " + (a.tempo.confidence > 0.5 ? "Confident." : "Could be half or double.") + (alts ? " Also plausible: " + alts + "." : "") + tuning;
  $("reset-bpm").hidden = !state.bpmHint;
  $("key").textContent = keyName();
  $("key-sub").textContent = (a.key.confidence < 0.05 ? "Close call, could also be " : "Runner-up: ") + a.key.alternative + ".";
  const g = a.guitar;
  $("capo").textContent = g.capo ? "Capo " + g.capo : "No capo";
  const opt = g.options.find((o) => o.capo === g.capo) || {};
  const none = g.options.find((o) => o.capo === 0) || {};
  $("capo-sub").textContent = g.capo
    ? "Turns " + fmtTime(none.hardSeconds || 0) + " of barre chords into " + fmtTime(opt.hardSeconds || 0) + "."
    : "The open shapes already fit this song.";
  const shapes = $("shapes"); shapes.innerHTML = "";
  const respell = (label, shiftBy, tonic) => { const c = parseLabel(label); return c ? spell((c.pc + shiftBy + 120) % 12, tonic, a.key.minor) + c.suffix : label; };
  const soundingTonic = (shownTonicBase() + state.transpose + 120) % 12;
  const shapeTonic = (soundingTonic - g.capo + 120) % 12;
  Object.entries(g.shapes).forEach(([sounding, shape]) => {
    const el = document.createElement("span"); el.className = "shape";
    const s1 = respell(sounding, state.transpose, soundingTonic), s2 = respell(shape, state.transpose, shapeTonic);
    el.innerHTML = g.capo ? `${esc(s1)} → <b>${esc(s2)}</b>` : `<b>${esc(s1)}</b>`;
    shapes.appendChild(el);
  });
  const dia = $("diagrams"); dia.innerHTML = "";
  const uke = state.instrument === "ukulele";
  (uke ? Object.keys(g.shapes) : Object.values(g.shapes)).forEach((shape) => {
    const c = parseLabel(shape); if (!c) return;
    const pc = (c.pc + state.transpose + 120) % 12;
    const name = spell(pc, uke ? soundingTonic : shapeTonic, a.key.minor) + c.suffix;
    const svg = uke ? (ukeDiagram(name) || ukeDiagram(SHARP[pc] + c.suffix) || ukeDiagram(FLAT[pc] + c.suffix)) : diagram(name, pc, c.suffix);
    if (!svg) return;
    const wrap = document.createElement("span"); wrap.innerHTML = svg;
    dia.appendChild(wrap.firstChild);
  });
  $("capo-sub").textContent = uke ? "Ukulele shapes for the sounding chords (no capo)." : $("capo-sub").textContent;
  [...$("instrument").children].forEach((b) => b.classList.toggle("on", b.dataset.v === state.instrument));
  const cs = $("capo-select"); cs.innerHTML = "";
  g.options.forEach((o) => {
    const el = document.createElement("option"); el.value = o.capo;
    el.textContent = (o.capo === 0 ? "None" : "Fret " + o.capo) + (o.capo === g.capo ? " (suggested)" : "") + " · " + fmtTime(o.hardSeconds) + " barre";
    cs.appendChild(el);
  });
  cs.value = state.capo;
  const ts = $("transpose"); ts.innerHTML = "";
  for (let i = -6; i <= 6; i++) { const el = document.createElement("option"); el.value = i; el.textContent = i === 0 ? "Original key" : (i > 0 ? "+" : "") + i + " semitones"; ts.appendChild(el); }
  ts.value = state.transpose;
  [...$("show").children].forEach((b) => b.classList.toggle("on", b.dataset.v === state.show));
  $("edit").classList.toggle("on", state.editing); $("edit").textContent = state.editing ? "Done editing" : "Edit chart"; $("nudge").hidden = !state.editing;
  renderTimeline();
  renderChart();
  const w = $("warnings"); w.innerHTML = "";
  a.warnings.forEach((t) => { const d = document.createElement("div"); d.className = "warn"; d.textContent = t; w.appendChild(d); });
}
function esc(s) { return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])); }
function color(label) { return "var(" + COLORS[(label.charCodeAt(0) - 65) % COLORS.length] + ")"; }

function cssVar(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }
function colorOf(label) { return cssVar(COLORS[(label.charCodeAt(0) - 65) % COLORS.length]); }
function renderTimeline() {
  const a = state.analysis; if (!a) return;
  drawWave($("wave"), { peaks: state.peaks, analysis: a, time: $("player").currentTime || 0, colorOf, ink: cssVar("--ink"), muted: cssVar("--muted") });
}
new ResizeObserver(() => renderTimeline()).observe($("timeline"));
$("theme").addEventListener("click", () => setTimeout(() => { renderTimeline(); }, 50));
$("timeline").addEventListener("click", (e) => {
  if (state.shared) return;
  const r = $("timeline").getBoundingClientRect();
  const p = $("player"); p.currentTime = (e.clientX - r.left) / r.width * state.analysis.duration; p.play();
});

function renderChart() {
  const a = state.analysis, root = $("chart"); root.innerHTML = "";
  a.sections.forEach((s, si) => {
    const endBar = si + 1 < a.sections.length ? a.sections[si + 1].bar : a.bars.length;
    const sec = document.createElement("div"); sec.className = "section";
    const h = document.createElement("h4");
    h.innerHTML = `<span class="dot" style="background:${color(s.label)}">${esc(s.label)}</span><span class="guess">${esc(s.guess)}</span><span class="time">${fmtTime(s.start)} – ${fmtTime(s.end)}</span>`;
    if (state.editing) {
      const g = h.querySelector(".guess"); g.classList.add("editable"); g.title = "Click to rename";
      g.addEventListener("click", () => {
        const inp = document.createElement("input"); inp.value = s.guess; inp.className = "rename";
        g.replaceWith(inp); inp.focus(); inp.select();
        let finished = false;
        const done = () => { if (finished) return; finished = true; s.guess = inp.value.trim() || s.guess; touched(); renderChart(); renderTimeline(); };
        inp.addEventListener("blur", done); inp.addEventListener("change", done);
        inp.addEventListener("keydown", (k) => { if (k.key === "Enter") done(); if (k.key === "Escape") { inp.value = s.guess; done(); } });
      });
    }
    if (!state.shared) {
      const loop = document.createElement("button"); loop.type = "button"; loop.className = "btn ghost small loopbtn";
      const active = state.loop && state.loop.start === s.start;
      loop.textContent = active ? "Looping" : "Loop";
      loop.classList.toggle("on", !!active);
      loop.addEventListener("click", () => setLoop(active ? null : { start: s.start, end: s.end, label: s.label }));
      h.appendChild(loop);
    }
    sec.appendChild(h);
    const grid = document.createElement("div"); grid.className = "bars";
    const from = si === 0 ? 0 : s.bar;
    for (let i = from; i < endBar; i++) {
      const bar = a.bars[i];
      const el = document.createElement("div"); el.className = "bar"; el.dataset.start = bar.start; el.dataset.end = bar.end; el.dataset.i = i; el.style.setProperty("--sec", color(s.label));
      let prev = null;
      bar.beats.forEach((b) => {
        const span = document.createElement("span");
        if (b === prev) { span.className = "beat"; span.textContent = "·"; } else { span.textContent = display(b); }
        el.appendChild(span); prev = b;
      });
      if (bar.beats.length < (a.bars[1] ? a.bars[1].beats.length : 4)) { const p = document.createElement("span"); p.className = "pickup"; p.textContent = "pickup"; el.appendChild(p); }
      el.addEventListener("click", (ev) => {
        if (state.editing) { openChordEditor(el, i, ev); return; }
        if (state.shared) return;
        const p = $("player"); p.currentTime = bar.start; p.play();
      });
      grid.appendChild(el);
    }
    sec.appendChild(grid);
    root.appendChild(sec);
  });
}

// ---------- playback highlight (timeupdate keeps working when the tab is hidden)
let lastBar = null;
function tick() {
  if (!state.analysis) return;
  const t = $("player").currentTime;
  renderTimeline();
  const bars = $("chart").querySelectorAll(".bar");
  let now = null;
  for (const b of bars) { if (t >= +b.dataset.start && t < +b.dataset.end) { now = b; break; } }
  if (now !== lastBar) { if (lastBar) { lastBar.classList.remove("now"); lastBar.style.removeProperty("--p"); } if (now) now.classList.add("now"); lastBar = now; }
  if (now) now.style.setProperty("--p", ((t - +now.dataset.start) / (+now.dataset.end - +now.dataset.start)).toFixed(3));
  renderNowPlaying(t);
}

// ---------- editing: fix a chord, rename a section, move the bar lines
const QUALITIES = [["", "maj"], ["m", "min"], ["7", "7"], ["maj7", "maj7"], ["m7", "m7"]];
function beatOffset(barIndex) { let n = 0; for (let i = 0; i < barIndex; i++) n += state.analysis.bars[i].beats.length; return n; }
function openChordEditor(barEl, barIndex, ev) {
  closeEditor();
  const a = state.analysis, bar = a.bars[barIndex];
  const cells = [...barEl.children].filter((c) => !c.classList.contains("pickup"));
  let beat = cells.findIndex((c) => c === ev.target || c.contains(ev.target)); if (beat < 0) beat = 0;
  const cur = parseLabel(bar.beats[beat]) || { pc: 0, suffix: "" };
  const pop = document.createElement("div"); pop.className = "editor"; pop.id = "editor";
  // Root buttons are named the way the chart is showing them (shapes with
  // the capo on, or sounding chords); data-pc is always the sounding root.
  const frame = state.show === "numbers" ? 0 : shift();
  const name = (pc) => state.show === "numbers" ? (state.analysis.key.minor ? DEGREES_MINOR : DEGREES)[(pc - shownTonicBase() + 120) % 12] : spell((pc + frame + 120) % 12, shownTonic(), state.analysis.key.minor);
  pop.innerHTML = `<div class="ed-row" data-k="root">${SHARP.map((_, pc) => `<button type="button" data-pc="${pc}" class="${pc === cur.pc ? "on" : ""}">${name(pc)}</button>`).join("")}</div>
    <div class="ed-row" data-k="q">${QUALITIES.map(([v, t]) => `<button type="button" data-q="${v}" class="${v === cur.suffix ? "on" : ""}">${t}</button>`).join("")}<button type="button" data-q="N">N.C.</button></div>
    <div class="ed-row"><label class="check"><input type="checkbox" id="ed-bar" checked> whole bar</label><span class="ed-hint">Beat ${beat + 1} of ${bar.beats.length}</span><button type="button" class="btn small" id="ed-close">Done</button></div>`;
  barEl.after(pop);
  let pc = cur.pc, q = cur.suffix;
  const apply = () => {
    const label = q === "N" ? "N" : SHARP[pc] + q;
    const whole = pop.querySelector("#ed-bar").checked;
    for (let k = 0; k < bar.beats.length; k++) if (whole || k === beat) bar.beats[k] = label;
    rebuildChordsFromBars(); touched();
    cells.forEach((c, k) => { c.textContent = k > 0 && bar.beats[k] === bar.beats[k - 1] ? "·" : display(bar.beats[k]); c.className = k > 0 && bar.beats[k] === bar.beats[k - 1] ? "beat" : ""; });
  };
  pop.addEventListener("click", (e) => {
    const b = e.target.closest("button"); if (!b) return;
    if (b.id === "ed-close") { closeEditor(); renderChart(); return; }
    if (b.dataset.pc !== undefined) { pc = +b.dataset.pc; if (q === "N") q = ""; }
    if (b.dataset.q !== undefined) q = b.dataset.q;
    pop.querySelectorAll("[data-pc]").forEach((x) => x.classList.toggle("on", +x.dataset.pc === pc && q !== "N"));
    pop.querySelectorAll("[data-q]").forEach((x) => x.classList.toggle("on", x.dataset.q === q));
    apply();
  });
}
function closeEditor() { const e = $("editor"); if (e) e.remove(); }
// Chord spans follow the bars: one label per beat, merged into runs.
function rebuildChordsFromBars() {
  const a = state.analysis, spans = [];
  let k = 0;
  for (const bar of a.bars) {
    for (let j = 0; j < bar.beats.length; j++, k++) {
      const start = a.beats[k] ?? bar.start, end = a.beats[k + 1] ?? a.duration, label = bar.beats[j];
      const last = spans[spans.length - 1];
      if (last && last.label === label) last.end = end; else spans.push({ start, end, label });
    }
  }
  a.chords = spans;
}
// Move every bar line one beat earlier or later (for a missed first beat).
function nudgeBars(dir) {
  const a = state.analysis, bpb = a.beatsPerBar || 4;
  const labels = a.bars.flatMap((b) => b.beats);
  const cur = a.bars[0].beats.length % bpb; // beats in the pickup bar
  const phase = (cur + dir + bpb) % bpb;
  const bars = []; let k = 0;
  while (k < labels.length) {
    const end = k === 0 && phase > 0 ? phase : Math.min(labels.length, k + bpb);
    bars.push({ start: a.beats[k], end: a.beats[end] ?? a.duration, beats: labels.slice(k, end) });
    k = end;
  }
  a.bars = bars;
  a.downbeats = bars.filter((b) => b.beats.length === bpb).map((b) => b.start);
  a.sections.forEach((s) => { let i = bars.findIndex((b) => b.start >= s.start - 0.01); if (i < 0) i = bars.length - 1; s.bar = i; s.start = bars[i].start; });
  touched(); renderChart(); renderTimeline();
}
let saveTimer = null;
function touched() {
  if (!state.fileKey) return;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => library.save(state.fileKey, state.file ? state.file.name : state.sharedTitle, state.analysis).then(renderLibrary), 400);
}
$("edit").addEventListener("click", () => { state.editing = !state.editing; $("edit").classList.toggle("on", state.editing); $("edit").textContent = state.editing ? "Done editing" : "Edit chart"; $("nudge").hidden = !state.editing; closeEditor(); renderChart(); });
$("nudge-left").addEventListener("click", () => nudgeBars(-1));
$("nudge-right").addEventListener("click", () => nudgeBars(1));
$("instrument").addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b) return; state.instrument = b.dataset.v; render(); });

// ---------- library of recent charts (this browser only, charts not audio)
async function renderLibrary() {
  const rows = await library.list();
  const box = $("library"); box.hidden = !rows || !rows.length; if (box.hidden) return;
  const ul = $("library-list"); ul.innerHTML = "";
  rows.forEach((r) => {
    const li = document.createElement("li");
    const a = r.analysis;
    li.innerHTML = `<button type="button" class="lib-open"><b>${esc(r.title)}</b><span>${esc(a.key.name)} · ${a.tempo.bpm} BPM · ${fmtTime(a.duration)}${a.guitar.capo ? " · capo " + a.guitar.capo : ""}</span></button><button type="button" class="iconbtn lib-del" title="Forget">×</button>`;
    li.querySelector(".lib-open").addEventListener("click", () => openSavedChart(r));
    li.querySelector(".lib-del").addEventListener("click", async () => { await library.remove(r.key); renderLibrary(); });
    ul.appendChild(li);
  });
}
function openSavedChart(r) {
  state.shared = true; state.sharedTitle = r.title; state.fileKey = r.key; state.file = null;
  state.analysis = r.analysis; state.capo = r.analysis.guitar.capo; state.loop = null; state.editing = false;
  $("fname").textContent = r.title;
  $("fmeta").textContent = fmtTime(r.analysis.duration) + " · saved chart, drop the audio file to play along";
  $("home").style.display = "none"; $("results").classList.add("active"); document.body.classList.add("shared");
  render();
  mountLive();
}
renderLibrary();

// ---------- practice: loop a section, play it slower without changing pitch
function setLoop(loop) {
  state.loop = loop;
  const p = $("player");
  $("loop-status").textContent = loop ? "Looping " + loop.label + " " + fmtTime(loop.start) + " – " + fmtTime(loop.end) + ". Esc stops." : "";
  if (loop) { p.currentTime = loop.start; p.play(); }
  renderChart();
}
$("player").addEventListener("timeupdate", () => {
  const p = $("player"), l = state.loop;
  if (l && (p.currentTime >= l.end - 0.05 || p.currentTime < l.start - 1)) p.currentTime = l.start;
});
$("speed").addEventListener("change", (e) => { const p = $("player"); p.preservesPitch = true; p.playbackRate = +e.target.value; });

// Persistent footer while playing: the three chords just played, the one
// sounding now with a countdown, and the next three.
let lastNowKey = "", lastBeat = -1;
const TONES = { "": [0, 4, 7], "m": [0, 3, 7], "7": [0, 4, 7, 10], "maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10] };
function ringSvg() {
  // Twelve pitch classes around a ring; the chord's notes light up.
  let s = '<svg class="ring" viewBox="0 0 100 100" aria-hidden="true">';
  for (let k = 0; k < 12; k++) {
    const a0 = (k - 3) / 12 * Math.PI * 2 - Math.PI / 12, a1 = a0 + Math.PI * 2 / 12;
    const r0 = 28, r1 = 46, cx = 50, cy = 50;
    const p = (r, a) => `${(cx + r * Math.cos(a)).toFixed(2)},${(cy + r * Math.sin(a)).toFixed(2)}`;
    s += `<path class="seg" data-pc="${k}" d="M${p(r0, a0)} L${p(r1, a0)} A${r1},${r1} 0 0 1 ${p(r1, a1)} L${p(r0, a1)} A${r0},${r0} 0 0 0 ${p(r0, a0)} Z"/>`;
    const am = (a0 + a1) / 2; s += `<text data-pc="${k}" x="${(cx + 37 * Math.cos(am)).toFixed(2)}" y="${(cy + 37 * Math.sin(am)).toFixed(2)}">${SHARP[k].replace("#", "♯")}</text>`;
  }
  return s + "</svg>";
}
function renderNowPlaying(t) {
  const a = state.analysis, foot = $("nowplaying");
  const playing = !$("player").paused;
  foot.classList.toggle("on", playing);
  if (!playing) { lastNowKey = ""; return; }
  const spans = a.chords;
  let i = spans.findIndex((c) => t >= c.start && t < c.end);
  if (i < 0) i = t < spans[0].start ? -1 : spans.length - 1;
  const cur = spans[i];
  const spb = 60 / a.tempo.bpm;
  const beatsLeft = cur ? Math.max(0, Math.ceil((cur.end - t) / spb)) : 0;
  // A pulse on every beat.
  let bi = -1; for (let k = 0; k < a.beats.length; k++) { if (a.beats[k] <= t) bi = k; else break; }
  const key = i + ":" + beatsLeft + ":" + shift() + ":" + state.show;
  if (key !== lastNowKey) {
    lastNowKey = key;
    const cell = (c, cls) => `<div class="np ${cls}">${c ? esc(display(c.label)) : "<span class='dim'>·</span>"}</div>`;
    let html = ringSvg();
    for (let k = 3; k >= 1; k--) html += cell(spans[i - k], "prev");
    html += `<div class="np now">${cur ? esc(display(cur.label)) : "…"}<span class="count">${cur ? beatsLeft : ""}</span></div>`;
    for (let k = 1; k <= 3; k++) html += cell(spans[i + k], "next n" + k);
    foot.innerHTML = html;
    const c = cur ? parseLabel(cur.label) : null;
    if (c) {
      const tones = (TONES[c.suffix] || TONES[""]).map((iv) => (c.pc + iv + shift() + 120) % 12);
      foot.querySelectorAll(".ring [data-pc]").forEach((el) => { const pc = +el.dataset.pc; el.classList.toggle("on", tones.includes(pc)); el.classList.toggle("root", pc === tones[0]); });
    }
  }
  if (bi !== lastBeat) {
    lastBeat = bi;
    const now = foot.querySelector(".np.now");
    if (now) { now.classList.remove("pulse"); void now.offsetWidth; now.classList.add("pulse"); }
  }
}
document.addEventListener("keydown", (e) => {
  if (viz || e.target.matches("input, select, textarea, button")) return;
  const p = $("player");
  if (!state.analysis || state.shared) return;
  if (e.code === "Space") { e.preventDefault(); p.paused ? p.play() : p.pause(); }
  if (e.code === "Escape" && state.loop) setLoop(null);
  if (e.code === "ArrowLeft" || e.code === "ArrowRight") {
    e.preventDefault();
    const bars = state.analysis.bars, t = p.currentTime;
    let i = bars.findIndex((b) => t >= b.start && t < b.end); if (i < 0) i = 0;
    const j = e.code === "ArrowRight" ? Math.min(bars.length - 1, i + 1) : (t - bars[i].start > 1 ? i : Math.max(0, i - 1));
    p.currentTime = bars[j].start;
  }
});
$("player").addEventListener("pause", () => renderNowPlaying($("player").currentTime));
$("player").addEventListener("play", () => renderNowPlaying($("player").currentTime));
$("player").addEventListener("timeupdate", tick);
(function raf() { if (!$("player").paused) tick(); requestAnimationFrame(raf); })();

// ---------- controls
$("capo-select").addEventListener("change", (e) => { state.capo = +e.target.value; renderChart(); });
$("transpose").addEventListener("change", (e) => { state.transpose = +e.target.value; $("key").textContent = keyName(); renderChart(); });
$("spelling").addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b) return; state.spelling = b.dataset.v; [...$("spelling").children].forEach((x) => x.classList.toggle("on", x === b)); render(); });
$("show").addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b) return; state.show = b.dataset.v; [...$("show").children].forEach((x) => x.classList.toggle("on", x === b)); renderChart(); });
$("half").addEventListener("click", () => { state.bpmHint = state.analysis.tempo.bpm / 2; analyze(); });
$("double").addEventListener("click", () => { state.bpmHint = state.analysis.tempo.bpm * 2; analyze(); });
$("reset-bpm").addEventListener("click", () => { state.bpmHint = null; analyze(); });
$("tap").addEventListener("click", () => {
  const now = performance.now();
  if (state.taps.length && now - state.taps[state.taps.length - 1] > 2500) state.taps = [];
  state.taps.push(now);
  const n = state.taps.length;
  if (n < 2) { $("bpm-sub").textContent = "Keep tapping on the beat…"; return; }
  const gaps = []; for (let i = 1; i < n; i++) gaps.push(state.taps[i] - state.taps[i - 1]);
  gaps.sort((a, b) => a - b);
  const bpm = 60000 / gaps[Math.floor(gaps.length / 2)];
  $("bpm-sub").textContent = "Tapped " + bpm.toFixed(0) + " BPM (" + n + " taps). " + (n >= 4 ? "Re-analyzing…" : "Tap a few more.");
  if (n >= 4) { state.bpmHint = bpm; state.taps = []; analyze(); }
});

// ---------- export
function sheetText() {
  // Re-spell the engine's sheet for the chosen capo and transpose.
  const a = state.analysis;
  let out = a.tempo.bpm + " BPM, " + keyName() + "\n";
  if (state.show === "shapes" && state.capo) out += "Capo " + state.capo + ", shapes as played\n";
  else if (state.transpose) out += "Transposed " + (state.transpose > 0 ? "+" : "") + state.transpose + "\n";
  a.sections.forEach((s, si) => {
    const endBar = si + 1 < a.sections.length ? a.sections[si + 1].bar : a.bars.length;
    out += "\n[" + s.label + " " + s.guess + "]\n";
    let line = "";
    const from = si === 0 ? 0 : s.bar;
    for (let i = from; i < endBar; i++) {
      let cell = "", prev = null;
      a.bars[i].beats.forEach((b) => { if (b !== prev) { cell += (cell ? " " : "") + display(b); prev = b; } });
      line += "| " + cell.padEnd(7);
      if ((i - from + 1) % 4 === 0) { out += line + "|\n"; line = ""; }
    }
    if (line) out += line + "|\n";
  });
  return out;
}
function download(name, data, type) {
  const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([data], { type })); a.download = name; a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
}
function base() { return ((state.file ? state.file.name : state.sharedTitle || "song").replace(/\.[^.]+$/, "") || "song"); }
$("copy").addEventListener("click", async () => { try { await navigator.clipboard.writeText(sheetText()); $("copy").textContent = "Copied"; setTimeout(() => ($("copy").textContent = "Copy chord sheet"), 1500); } catch (e) { download(base() + " chords.txt", sheetText(), "text/plain"); } });
$("dl-sheet").addEventListener("click", () => download(base() + " chords.txt", sheetText(), "text/plain"));
$("print").addEventListener("click", () => window.print());
let viz = null, liveViz = null;
// Aurora is for the room, so it names what is heard, not the capo shape.
const displaySounding = (l) => { const c = parseLabel(l); if (!c) return "N.C."; const pc = (c.pc + state.transpose + 120) % 12; return spell(pc, (shownTonicBase() + state.transpose + 120) % 12, state.analysis.key.minor) + c.suffix; };
const vizOpts = () => ({ analysis: state.analysis, player: $("player"), display, displaySounding, transpose: () => state.transpose, colorOf, parseLabel });
$("visualize").addEventListener("click", () => {
  if (viz) return;
  viz = openVisualizer({ ...vizOpts(), style: liveStyle, onClose: () => { viz = null; } });
  if (!state.shared && $("player").paused) $("player").play().catch(() => {});
});
// The live panel draws the same thing inline while the song plays.
let liveStyle = "flow";
try { liveStyle = localStorage.getItem("chordmap.live") || "flow"; } catch (e) { /* private mode */ }
function mountLive() {
  if (liveViz) liveViz.close();
  liveViz = null;
  if (!state.analysis || state.shared) { $("live").hidden = true; return; }
  $("live").hidden = false;
  [...$("live-style").children].forEach((b) => b.classList.toggle("on", b.dataset.v === liveStyle));
  liveViz = openVisualizer({ ...vizOpts(), inline: true, mount: $("live-canvas"), style: liveStyle, always: true });
}
$("live-style").addEventListener("click", (e) => {
  const b = e.target.closest("button"); if (!b) return;
  liveStyle = b.dataset.v; try { localStorage.setItem("chordmap.live", liveStyle); } catch (err) { /* private mode */ }
  if (liveViz) liveViz.setStyle(liveStyle); [...$("live-style").children].forEach((x) => x.classList.toggle("on", x === b));
});
$("live-expand").addEventListener("click", () => $("visualize").click());
$("player").addEventListener("play", () => document.body.classList.add("playing"));
$("player").addEventListener("pause", () => document.body.classList.remove("playing"));
$("dl-midi").addEventListener("click", () => download(base() + " chords.mid", midiBytes(state.analysis), "audio/midi"));
$("dl-chordpro").addEventListener("click", () => download(base() + ".cho", chordPro(state.analysis, base(), display), "text/plain"));
$("share").addEventListener("click", async () => {
  try {
    const url = await shareLink(state.analysis, base());
    await navigator.clipboard.writeText(url);
    $("share").textContent = "Link copied (" + Math.round(url.length / 1024) + " KB)";
  } catch (e) { $("share").textContent = "Could not make a link"; }
  setTimeout(() => ($("share").textContent = "Share chart"), 2500);
});
$("dl-json").addEventListener("click", () => download(base() + " chordmap.json", JSON.stringify(state.analysis, null, 2), "application/json"));
