// Exports that need no server: a Standard MIDI File of the chords, a
// ChordPro sheet, and a share link with the chart gzipped into the URL hash.

const SHARP = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const FLAT = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
function parse(l) {
  for (const suf of ["maj7", "m7", "7", "m", ""]) {
    if (!l.endsWith(suf)) continue;
    const name = suf ? l.slice(0, -suf.length) : l;
    let pc = SHARP.indexOf(name); if (pc < 0) pc = FLAT.indexOf(name);
    if (pc >= 0) return { pc, suf };
  }
  return null;
}
const TONES = { "": [0, 4, 7], "m": [0, 3, 7], "7": [0, 4, 7, 10], "maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10] };

function vlq(n) { const b = [n & 0x7f]; while ((n >>= 7) > 0) b.unshift((n & 0x7f) | 0x80); return b; }
function u32(n) { return [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255]; }
function u16(n) { return [(n >> 8) & 255, n & 255]; }
function str(s) { return [...s].map((c) => c.charCodeAt(0)); }

/** Chords as block voicings with a bass note, one track, tempo from the analysis. */
export function midiBytes(a) {
  const ppq = 480, spb = 60 / a.tempo.bpm;
  const ticks = (sec) => Math.round(sec / spb * ppq);
  const ev = []; // [tick, bytes]
  ev.push([0, [0xff, 0x51, 0x03, ...u32(Math.round(60000000 / a.tempo.bpm)).slice(1)]]);
  ev.push([0, [0xff, 0x58, 0x04, a.beatsPerBar || 4, 2, 24, 8]]);
  ev.push([0, [0xc0, 24]]); // nylon guitar
  ev.push([0, [0xc1, 33]]); // fingered bass
  for (const c of a.chords) {
    const p = parse(c.label); if (!p) continue;
    const on = ticks(c.start), off = Math.max(on + 1, ticks(c.end) - 10);
    const notes = TONES[p.suf].map((iv) => 60 + ((p.pc + iv) % 12) + (iv >= 12 ? 12 : 0)).map((n, i, arr) => (i > 0 && n < arr[0] ? n + 12 : n));
    for (const n of notes) { ev.push([on, [0x90, n, 80]]); ev.push([off, [0x80, n, 0]]); }
    const bass = 36 + p.pc;
    ev.push([on, [0x91, bass, 90]]); ev.push([off, [0x81, bass, 0]]);
  }
  ev.sort((x, y) => x[0] - y[0] || (x[1][0] & 0xf0) - (y[1][0] & 0xf0));
  const track = []; let last = 0;
  for (const [t, bytes] of ev) { track.push(...vlq(t - last), ...bytes); last = t; }
  track.push(0, 0xff, 0x2f, 0);
  return new Uint8Array([...str("MThd"), ...u32(6), ...u16(0), ...u16(1), ...u16(ppq), ...str("MTrk"), ...u32(track.length), ...track]);
}

/** Words that start inside a bar, joined. */
function barLyric(a, bar) {
  return (a.lyrics || []).filter((w) => w.start >= bar.start && w.start < bar.end).map((w) => w.text).join(" ");
}

/** Chords over lyrics, four bars per line, plain text. Falls back to chords only. */
export function lyricSheet(a, title, display) {
  let out = `${title}\n${a.tempo.bpm} BPM, ${a.meter}, ${a.key.name}` + (a.guitar.capo ? `, capo ${a.guitar.capo}` : "") + "\n";
  a.sections.forEach((s, si) => {
    const endBar = si + 1 < a.sections.length ? a.sections[si + 1].bar : a.bars.length;
    out += `\n[${s.label} ${s.guess}]\n`;
    const from = si === 0 ? 0 : s.bar;
    for (let i = from; i < endBar; i += 4) {
      const row = a.bars.slice(i, Math.min(endBar, i + 4));
      const cells = row.map((b) => { let prev = null, c = ""; for (const x of b.beats) { if (x !== prev) { c += (c ? " " : "") + display(x); prev = x; } } return c; });
      const lyr = row.map((b) => barLyric(a, b));
      const w = cells.map((c, k) => Math.max(c.length, lyr[k].length, 6) + 2);
      out += "| " + cells.map((c, k) => c.padEnd(w[k])).join("| ") + "|\n";
      if (lyr.some(Boolean)) out += "  " + lyr.map((l, k) => l.padEnd(w[k])).join("  ") + "\n";
    }
  });
  return out;
}

/** ChordPro text with sections as comments and one line per four bars; chords sit inside the lyrics when there are any. */
export function chordPro(a, title, display) {
  let out = `{title: ${title}}\n{key: ${a.key.name}}\n{tempo: ${a.tempo.bpm}}\n{time: ${a.meter}}\n`;
  if (a.guitar.capo) out += `{capo: ${a.guitar.capo}}\n`;
  a.sections.forEach((s, si) => {
    const endBar = si + 1 < a.sections.length ? a.sections[si + 1].bar : a.bars.length;
    out += `\n{comment: ${s.label} ${s.guess}}\n`;
    let line = "";
    const from = si === 0 ? 0 : s.bar;
    for (let i = from; i < endBar; i++) {
      const bar = a.bars[i];
      const words = (a.lyrics || []).filter((w) => w.start >= bar.start && w.start < bar.end);
      let cell = "";
      if (words.length) {
        // Each chord goes in front of the first word sung after its beat.
        const spb = (bar.end - bar.start) / bar.beats.length;
        let prev = null, wi = 0;
        for (let k = 0; k < bar.beats.length; k++) {
          const b = bar.beats[k], t = bar.start + k * spb;
          while (wi < words.length && words[wi].start < t - spb / 2) cell += words[wi++].text + " ";
          if (b !== prev) { cell += "[" + display(b) + "]"; prev = b; }
        }
        while (wi < words.length) cell += words[wi++].text + " ";
        line += cell.trim() + " ";
      } else {
        let prev = null;
        for (const b of bar.beats) { if (b !== prev) { cell += "[" + display(b) + "] "; prev = b; } else cell += ". "; }
        line += "| " + cell;
      }
      if ((i - from + 1) % 4 === 0) { out += line.trim() + (words.length ? "" : " |") + "\n"; line = ""; }
    }
    if (line) out += line.trim() + "\n";
  });
  return out;
}

async function gzip(text) {
  const cs = new CompressionStream("gzip");
  const w = cs.writable.getWriter(); w.write(new TextEncoder().encode(text)); w.close();
  return new Uint8Array(await new Response(cs.readable).arrayBuffer());
}
async function gunzip(bytes) {
  const ds = new DecompressionStream("gzip");
  const w = ds.writable.getWriter(); w.write(bytes); w.close();
  return new TextDecoder().decode(await new Response(ds.readable).arrayBuffer());
}
const b64u = (u8) => btoa(String.fromCharCode(...u8)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const unb64u = (s) => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));

/** A URL carrying the chart (not the audio). */
export async function shareLink(a, title) {
  const slim = { t: title, a };
  const bytes = await gzip(JSON.stringify(slim));
  return location.origin + location.pathname + "#c=" + b64u(bytes);
}
export async function readShareLink() {
  const m = location.hash.match(/^#c=([A-Za-z0-9_-]+)$/);
  if (!m) return null;
  try { return JSON.parse(await gunzip(unb64u(m[1]))); } catch (e) { return null; }
}
