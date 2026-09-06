// Screenshots of the app on a local file for announcements. Needs
// examples/web/local/<song>.wav (gitignored), the static server on :8766,
// Google Chrome, and `npm i puppeteer-core` somewhere on NODE_PATH.
// Usage: node scripts/shots.mjs <out dir>

import puppeteer from "puppeteer-core";
const OUT = process.argv[2];
const browser = await puppeteer.launch({
  executablePath: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  headless: "new",
  args: ["--enable-unsafe-webgpu", "--use-angle=metal", "--autoplay-policy=no-user-gesture-required", "--window-size=1600,900", "--hide-scrollbars"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1600, height: 900, deviceScaleFactor: 2 });
await page.evaluateOnNewDocument(() => { try { localStorage.setItem("chordmap.theme", "dark"); localStorage.setItem("chordmap.live", "flow"); } catch (e) {} });
page.on("console", (m) => { if (m.type() === "error") console.log("console:", m.text().slice(0, 160)); });

// 1. Landing page.
await page.goto("http://localhost:8766/?shot=1", { waitUntil: "networkidle0" });
await new Promise((r) => setTimeout(r, 1200));
await page.screenshot({ path: OUT + "/1-landing.png" });
console.log("1 landing");

// 2. Chart on the song.
await page.goto("http://localhost:8766/?demo=" + encodeURIComponent("local/Who I Really Am.wav"), { waitUntil: "networkidle0" });
await page.waitForFunction(() => document.getElementById("results").classList.contains("active"), { timeout: 120000 });
await new Promise((r) => setTimeout(r, 1500));
await page.evaluate(() => { const p = document.getElementById("player"); p.currentTime = 62; });
await new Promise((r) => setTimeout(r, 800));
await page.screenshot({ path: OUT + "/2-chart.png" });
console.log("2 chart", await page.evaluate(() => document.getElementById("fmeta").textContent));

// 2b. Chord grid with the now-playing footer.
await page.evaluate(async () => { const p = document.getElementById("player"); p.currentTime = 62; await p.play().catch(() => {}); });
await new Promise((r) => setTimeout(r, 1500));
await page.evaluate(() => { const secs = document.querySelectorAll(".section"); (secs[1] || secs[0]).scrollIntoView(); window.scrollBy(0, -24); });
await new Promise((r) => setTimeout(r, 900));
await page.screenshot({ path: OUT + "/2b-grid.png" });
await page.evaluate(() => document.getElementById("player").pause());
console.log("2b grid");

// 3. Stems: vocals, then the other three, show the mixer.
for (let k = 0; k < 2; k++) {
  await page.evaluate(() => document.getElementById("separate").click());
  await new Promise((r) => setTimeout(r, 1500));
  await page.waitForFunction(() => /Separated|failed|Playing/.test(document.getElementById("mix-status").textContent) && document.getElementById("sep-progress").hidden, { timeout: 600000 });
  console.log("3 status", k, await page.evaluate(() => document.getElementById("mix-status").textContent));
  await new Promise((r) => setTimeout(r, 2500));
}
await page.evaluate(() => { const s = document.getElementById("source"); s.value = "stems-mix"; s.dispatchEvent(new Event("change")); });
await new Promise((r) => setTimeout(r, 2500));
await page.evaluate(() => { document.querySelector(".panel").scrollIntoView(); window.scrollBy(0, -12); });
await new Promise((r) => setTimeout(r, 600));
await page.screenshot({ path: OUT + "/3-stems.png" });

// 4. Aurora, full screen, playing.
await page.evaluate(async () => { const p = document.getElementById("player"); p.currentTime = 70; await p.play().catch(() => {}); });
await page.click("#visualize");
await page.keyboard.press("3");
await new Promise((r) => setTimeout(r, 3500));
await page.screenshot({ path: OUT + "/4-aurora.png" });
console.log("4 aurora", await page.evaluate(() => window.__vizDebug ? JSON.stringify(window.__vizDebug()) : "n/a"));
await page.keyboard.press("1");
await new Promise((r) => setTimeout(r, 1200));
await page.screenshot({ path: OUT + "/5-stage.png" });
await browser.close();
