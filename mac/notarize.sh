#!/bin/bash
# Notarizes build/Chordmap for Mac.app so it opens on any Mac without warnings.
#
# One-time setup (needs an Apple Developer account, $99/year). Nothing below is required to build or run locally.
#   1. Create a Developer ID Application certificate at developer.apple.com and
#      install it in Keychain Access.
#   2. Create an app-specific password at appleid.apple.com, then store it:
#        xcrun notarytool store-credentials "chordmac" --apple-id you@example.com --team-id TEAMID
#   3. Build signed:  SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)" ./build-app.sh
#   4. Run this script.
set -euo pipefail
cd "$(dirname "$0")"
APP="build/Chordmap for Mac.app"
PROFILE="${NOTARY_PROFILE:-chordmac}"
[ -d "$APP" ] || { echo "build first"; exit 1; }
ZIP="build/ChordMap.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
xcrun notarytool submit "$ZIP" --keychain-profile "$PROFILE" --wait
xcrun stapler staple "$APP"
spctl -a -vv "$APP"
echo "Notarized and stapled: $APP"
