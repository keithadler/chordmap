#!/bin/bash
# One-time setup for local builds: creates a self-signed code-signing certificate named
# "ChordMap Local Signing" in your login keychain and trusts it for code signing.
#
# Why: macOS remembers permissions (Downloads, Automation, Notifications) per app *signature*.
# An ad-hoc signed build gets a brand-new signature every time it is built, so macOS asks for
# every permission again after each rebuild. A stable certificate means one set of prompts,
# then never again, even across rebuilds. build-app.sh uses this certificate automatically
# when it exists. For distribution to other Macs use a Developer ID instead (see notarize.sh).
#
# You may see one or two keychain prompts. Enter your login password when asked.
set -euo pipefail

NAME="ChordMap Local Signing"
KC="$HOME/Library/Keychains/login.keychain-db"

if security find-identity -v -p codesigning | grep -q "$NAME"; then
  echo "\"$NAME\" already exists. Nothing to do."
  exit 0
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

cat > ext.cnf <<EOF
[req]
distinguished_name = dn
x509_extensions = v3
prompt = no
[dn]
CN = $NAME
[v3]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
subjectKeyIdentifier = hash
EOF

echo "Creating certificate…"
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -keyout key.pem -out cert.pem -config ext.cnf 2>/dev/null
# -legacy is needed on OpenSSL 3 for a p12 macOS can import; fall back for older OpenSSL.
openssl pkcs12 -export -inkey key.pem -in cert.pem -out local.p12 -passout pass:chordmac -name "$NAME" -legacy 2>/dev/null \
  || openssl pkcs12 -export -inkey key.pem -in cert.pem -out local.p12 -passout pass:chordmac -name "$NAME"

echo "Importing into your login keychain…"
security import local.p12 -k "$KC" -P chordmac -T /usr/bin/codesign -T /usr/bin/security -A

echo "Trusting it for code signing (macOS may ask for your password)…"
security add-trusted-cert -r trustRoot -p codeSign -k "$KC" cert.pem

echo
security find-identity -v -p codesigning | grep "$NAME" && echo "Done. Rebuild with ./build-app.sh --install and answer the permission prompts one last time."
