#!/bin/bash
# Builds Chordmap for Mac with Swift Package Manager (no Xcode needed) and wraps it in a .app bundle.
#
#   ./build-app.sh [--run] [--install]
#
# Signing: by default the app is ad-hoc signed, which works on this Mac only. To sign for other Macs,
# set SIGN_IDENTITY to your Developer ID, e.g.
#   SIGN_IDENTITY="Developer ID Application: Keith Adler (TEAMID)" ./build-app.sh
# then run ./notarize.sh. See README.
set -euo pipefail
cd "$(dirname "$0")"

# Universal (Intel + Apple Silicon) by default. The Command Line Tools can't do a two-arch build in one
# go (that needs Xcode's xcbuild), so each slice is built separately and joined with lipo.
# Set ARCHS to one triple for a quick local build, e.g. ARCHS=arm64-apple-macosx ./build-app.sh
# The engine is chordmap, a Rust static library: build it first, then only the architectures it has.
./ffi/build.sh | tail -1
LIBARCHS="$(lipo -archs ffi/lib/libchordmap_ffi.a)"
if [ -z "${ARCHS:-}" ]; then
  ARCHS=""
  case "$LIBARCHS" in *arm64*) ARCHS="$ARCHS arm64-apple-macosx";; esac
  case "$LIBARCHS" in *x86_64*) ARCHS="$ARCHS x86_64-apple-macosx";; esac
  [[ "$LIBARCHS" == *x86_64* ]] || echo "Note: Intel slice skipped; run 'rustup target add x86_64-apple-darwin' for a universal build."
fi
SLICES=()
for triple in $ARCHS; do
  echo "Building ${triple}..."
  # Each slice gets its own scratch folder; sharing one confuses SwiftPM's build database.
  swift build -c release --triple "$triple" --scratch-path ".build/slices/$triple" 2>&1 | grep -vE '^\[|Compiling|Emitting|Linking|Build complete|Planning|Write' || true
  slice=".build/slices/$triple/$triple/release/ChordMap"
  [ -x "$slice" ] || { echo "build failed for $triple"; exit 1; }
  # Refuse a stale slice: it must be newer than every source file.
  newest_src=$(find Sources -name '*.swift' -newer "$slice" | head -1)
  [ -z "$newest_src" ] || { echo "build for $triple did not produce a fresh binary (see errors above)"; exit 1; }
  SLICES+=("$slice")
done
mkdir -p build
BIN="build/ChordMap"
if [ "${#SLICES[@]}" -gt 1 ]; then
  lipo -create "${SLICES[@]}" -output "$BIN"
else
  cp "${SLICES[0]}" "$BIN"
fi
echo "Binary: $BIN ($(lipo -archs "$BIN"))"

APP="build/Chordmap for Mac.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/ChordMap"
cp Info.plist "$APP/Contents/Info.plist"
if [ -f AppIcon.icns ]; then
  cp AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
  /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string AppIcon" "$APP/Contents/Info.plist" >/dev/null 2>&1 || true
fi
# Help pages travel inside the bundle so Help works offline.
cp docs/Help.html "$APP/Contents/Resources/Help.html"
cp docs/Help.es.html "$APP/Contents/Resources/Help.es.html"
# Translations: Bundle.main finds these at runtime.
for lproj in Localization/*.lproj; do
  [ -d "$lproj" ] && cp -R "$lproj" "$APP/Contents/Resources/"
done

# Signing identity, in order of preference: SIGN_IDENTITY from the environment (Developer ID),
# the local certificate from make-local-identity.sh (stable permissions across rebuilds), ad-hoc.
if [ -z "${SIGN_IDENTITY:-}" ]; then
  if security find-identity -v -p codesigning 2>/dev/null | grep -q "ChordMap Local Signing"; then
    SIGN_IDENTITY="ChordMap Local Signing"
  else
    SIGN_IDENTITY="-"
    echo "Note: ad-hoc signed, so macOS will re-ask permissions after every rebuild. Run ./make-local-identity.sh once to stop that."
  fi
fi
if [ "$SIGN_IDENTITY" = "-" ]; then
  codesign --force --sign - --entitlements ChordMap.entitlements "$APP" >/dev/null 2>&1 || codesign --force --sign - "$APP"
  echo "Signed: ad-hoc (this Mac only)"
elif [ "$SIGN_IDENTITY" = "ChordMap Local Signing" ]; then
  # Local certificate: no timestamp server (it's self-signed) and no hardened runtime needed.
  codesign --force --entitlements ChordMap.entitlements --sign "$SIGN_IDENTITY" "$APP"
  echo "Signed: local certificate (permissions stay granted across rebuilds)"
else
  codesign --force --options runtime --timestamp --entitlements ChordMap.entitlements --sign "$SIGN_IDENTITY" "$APP"
  echo "Signed: $SIGN_IDENTITY (hardened runtime)"
fi
echo "Built: $PWD/$APP"

for arg in "$@"; do
  case "$arg" in
    --install)
      rm -rf "/Applications/Chordmap for Mac.app" "/Applications/ChordMap.app"; cp -R "$APP" /Applications/; echo "Installed to /Applications"
      # Command line: a `chordmac` symlink somewhere on the PATH (`clip` is left free for other tools).
      linked=""
      for bindir in /usr/local/bin /opt/homebrew/bin "$HOME/.local/bin"; do
        [ -d "$bindir" ] && [ -w "$bindir" ] || continue
        ln -sf "/Applications/Chordmap for Mac.app/Contents/MacOS/ChordMap" "$bindir/chordmac" && linked="$bindir/chordmac" && break
      done
      if [ -n "$linked" ]; then echo "Command line: $linked"; else
        echo "No writable bin directory found. To add the command: sudo ln -sf \"/Applications/Chordmap for Mac.app/Contents/MacOS/ChordMap\" /usr/local/bin/chordmac"
      fi
      if [ -f docs/chordmac.1 ]; then
        for mandir in /usr/local/share/man/man1 /opt/homebrew/share/man/man1; do
          [ -d "$(dirname "$mandir")" ] && [ -w "$(dirname "$mandir")" ] || continue
          mkdir -p "$mandir" 2>/dev/null && cp docs/chordmac.1 "$mandir/chordmac.1" 2>/dev/null && echo "Man page: man chordmac" && break
        done
      fi
      ;;
    --run) if [ -d "/Applications/Chordmap for Mac.app" ] && [[ " $* " == *" --install "* ]]; then open "/Applications/Chordmap for Mac.app"; else open "$APP"; fi;;
  esac
done
exit 0
