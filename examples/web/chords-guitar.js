// Guitar voicings for the shape diagrams. Frets low E to high e, -1 = muted.
// Open shapes first; anything else becomes an E-shape or A-shape barre.
export const OPEN = {
  "C": [-1, 3, 2, 0, 1, 0], "D": [-1, -1, 0, 2, 3, 2], "E": [0, 2, 2, 1, 0, 0], "F": [1, 3, 3, 2, 1, 1],
  "G": [3, 2, 0, 0, 0, 3], "A": [-1, 0, 2, 2, 2, 0], "B": [-1, 2, 4, 4, 4, 2],
  "Am": [-1, 0, 2, 2, 1, 0], "Dm": [-1, -1, 0, 2, 3, 1], "Em": [0, 2, 2, 0, 0, 0], "Bm": [-1, 2, 4, 4, 3, 2],
  "A7": [-1, 0, 2, 0, 2, 0], "B7": [-1, 2, 1, 2, 0, 2], "C7": [-1, 3, 2, 3, 1, 0], "D7": [-1, -1, 0, 2, 1, 2],
  "E7": [0, 2, 0, 1, 0, 0], "G7": [3, 2, 0, 0, 0, 1],
  "Am7": [-1, 0, 2, 0, 1, 0], "Dm7": [-1, -1, 0, 2, 1, 1], "Em7": [0, 2, 0, 0, 0, 0], "Bm7": [-1, 2, 0, 2, 0, 2],
  "Cmaj7": [-1, 3, 2, 0, 0, 0], "Fmaj7": [-1, -1, 3, 2, 1, 0], "Amaj7": [-1, 0, 2, 1, 2, 0], "Dmaj7": [-1, -1, 0, 2, 2, 2],
  "Emaj7": [0, 2, 1, 1, 0, 0], "Gmaj7": [3, 2, 0, 0, 0, 2],
};
const E_SHAPE = { "": [0, 2, 2, 1, 0, 0], "m": [0, 2, 2, 0, 0, 0], "7": [0, 2, 0, 1, 0, 0], "maj7": [0, 2, 1, 1, 0, 0], "m7": [0, 2, 0, 0, 0, 0] };
const A_SHAPE = { "": [-1, 0, 2, 2, 2, 0], "m": [-1, 0, 2, 2, 1, 0], "7": [-1, 0, 2, 0, 2, 0], "maj7": [-1, 0, 2, 1, 2, 0], "m7": [-1, 0, 2, 0, 1, 0] };

/** Frets for a chord given its pitch class and suffix; `barre` is the fret of the index finger or 0. */
export function voicing(name, pc, suffix) {
  if (OPEN[name]) return { frets: OPEN[name], barre: name === "F" ? 1 : name === "B" || name === "Bm" ? 2 : 0 };
  const e = (pc - 4 + 12) % 12 || 12, a = (pc - 9 + 12) % 12 || 12;
  const useA = a < e && a <= 9;
  const fret = useA ? a : e;
  const shape = (useA ? A_SHAPE : E_SHAPE)[suffix] || (useA ? A_SHAPE : E_SHAPE)[""];
  return { frets: shape.map((f) => (f < 0 ? -1 : f + fret)), barre: fret };
}

/** Small SVG chord box. */
export function diagram(name, pc, suffix) {
  const { frets, barre } = voicing(name, pc, suffix);
  const played = frets.filter((f) => f > 0);
  const lo = played.length ? Math.min(...played) : 1;
  const hi = played.length ? Math.max(...played) : 1;
  // Open strings or anything within the first four frets sit at the nut.
  const base = frets.includes(0) || hi <= 4 ? 1 : lo;
  const W = 68, H = 84, x0 = 12, y0 = 18, sx = 9, sy = 12;
  let s = `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" class="cbox" role="img" aria-label="${name}">`;
  s += `<text x="${W / 2}" y="10" text-anchor="middle" font-size="9" font-weight="700" fill="currentColor">${name}</text>`;
  for (let i = 0; i < 6; i++) s += `<line x1="${x0 + i * sx}" y1="${y0}" x2="${x0 + i * sx}" y2="${y0 + 4 * sy}" stroke="currentColor" stroke-width=".8"/>`;
  for (let j = 0; j <= 4; j++) s += `<line x1="${x0}" y1="${y0 + j * sy}" x2="${x0 + 5 * sx}" y2="${y0 + j * sy}" stroke="currentColor" stroke-width="${j === 0 && base === 1 ? 2.4 : .8}"/>`;
  if (base > 1) s += `<text x="${x0 - 4}" y="${y0 + sy - 3}" text-anchor="end" font-size="7" fill="currentColor">${base}</text>`;
  if (barre) { const b = barre - base; if (b >= 0 && b < 4) { const first = frets.findIndex((f) => f === barre); s += `<rect x="${x0 + first * sx - 3}" y="${y0 + b * sy + 3}" width="${(5 - first) * sx + 6}" height="${sy - 6}" rx="3" fill="currentColor" opacity=".45"/>`; } }
  frets.forEach((f, i) => {
    const x = x0 + i * sx;
    if (f < 0) s += `<text x="${x}" y="${y0 - 3}" text-anchor="middle" font-size="7" fill="currentColor">×</text>`;
    else if (f === 0) s += `<circle cx="${x}" cy="${y0 - 5}" r="2.2" fill="none" stroke="currentColor" stroke-width=".8"/>`;
    else { const j = f - base; if (j >= 0 && j < 4) s += `<circle cx="${x}" cy="${y0 + j * sy + sy / 2}" r="3.2" fill="currentColor"/>`; }
  });
  return s + "</svg>";
}
