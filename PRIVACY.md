# Privacy

chordmap runs entirely on your device.

- The web app decodes and analyses audio in your browser. The file is never
  uploaded; there is no server to upload it to.
- No analytics, no cookies, no accounts. The browser keeps your theme
  choice and a library of recent charts (tempo, key, chords, sections,
  your corrections and the file name) in its own storage on this device.
  Audio is never stored. Each chart has a forget button.
- The CLI reads the file you name and writes to standard output.
- The AI stem separation is local. When you click "Separate vocals with
  on-device AI" the page fetches the onnxruntime engine (bundled with the
  site) and the model files from keithadler.github.io, the same publisher,
  and caches them in your browser. The model then runs on your device, on
  the GPU when the browser allows it. Your audio never leaves the device;
  only the model files come in, and nothing is sent back.
- Lyrics work the same way: "Transcribe lyrics on this device" fetches the
  Whisper model files and the transformers.js library from the same
  publisher, then runs on your device. Words are kept with the chart in
  your browser and in any share link you make; the audio stays put.
- "Share chart" builds a link with the chart (tempo, key, chords, sections,
  file name) compressed into the URL itself. The audio is not in it and
  nothing is stored anywhere; whoever gets the link gets only the chart.
