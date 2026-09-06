#!/bin/bash
# End-to-end run of the command line in an isolated home: chordmap's synth, a chart of it, the
# history. No audio device is opened, so this runs on a headless runner.
set -euo pipefail
cd "$(dirname "$0")/.."
BIN="${BIN:-.build/debug/ChordMap}"
[ -x "$BIN" ] || swift build >/dev/null
export CHORDMAC_HOME="$(mktemp -d)" CHORDMAC_DEMO_DEVICES=1
trap 'rm -rf "$CHORDMAC_HOME"' EXIT
pass=0; fail=0
check() { if eval "$2"; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL: $1"; echo "  command: $2"; fi; }
check "version names the engine" '"$BIN" version | grep -q "chordmap 1."'
check "help exits 0" '"$BIN" help >/dev/null'
check "unknown command exits 64" '"$BIN" bogus >/dev/null 2>&1; [ $? = 64 ]'
check "status sees the demo interface" '"$BIN" status | grep -q "Scarlett Solo USB"'
check "synth writes a progression" '"$BIN" synth "$CHORDMAC_HOME/chords.wav" "C G Am F" --loops 1 | grep -q "C G Am F"'
check "a file is charted in C major" '"$BIN" "$CHORDMAC_HOME/chords.wav" | grep -q "Key: C major"'
check "the chart carries a tempo" '"$BIN" "$CHORDMAC_HOME/chords.wav" | grep -q "Tempo:"'
check "--json carries the guitar section" '"$BIN" "$CHORDMAC_HOME/chords.wav" --json | grep -q "\"guitar\""'
check "--capo 3 shows shapes" '"$BIN" "$CHORDMAC_HOME/chords.wav" --capo 3 | grep -q "Shapes with capo 3"'
check "empty history exits 1" '"$BIN" history >/dev/null 2>&1; [ $? = 1 ]'
check "Info.plist carries the system audio string" 'grep -q NSAudioCaptureUsageDescription Info.plist'
check "Info.plist carries the microphone string" 'grep -q NSMicrophoneUsageDescription Info.plist'
if [ -d "/Applications/Chordmap for Mac.app" ]; then
  check "installed bundle carries the system audio string" 'grep -q NSAudioCaptureUsageDescription "/Applications/Chordmap for Mac.app/Contents/Info.plist"'
fi
check "selftest is green" '"$BIN" selftest | tail -1 | grep -q " 0 failed"'
echo "$pass passed, $fail failed"
[ "$fail" = 0 ]
