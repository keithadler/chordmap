# Privacy

chordmap runs entirely on your device.

- The web app decodes and analyses audio in your browser. The file is never
  uploaded; there is no server to upload it to.
- No analytics, no cookies, no accounts. The browser keeps your theme
  choice and a library of recent charts (tempo, key, chords, sections,
  your corrections and the file name) in its own storage on this device.
  Audio is never stored. Each chart has a forget button.
- The CLI reads the file you name and writes to standard output.
- The page loads no third-party scripts or fonts on its own. When you click
  "Separate vocals with AI" it fetches the onnxruntime engine (bundled with
  the site) and the separation models from keithadler.github.io, the same
  publisher, and caches them in your browser. Your audio still never leaves
  the device; only the model files come in.
- "Share chart" builds a link with the chart (tempo, key, chords, sections,
  file name) compressed into the URL itself. The audio is not in it and
  nothing is stored anywhere; whoever gets the link gets only the chart.
