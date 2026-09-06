#!/bin/bash
# Builds the chordmap bridge as a static library for every Apple target the toolchain has,
# joined with lipo into ffi/lib/libchordmap_ffi.a. Needs Rust (rustup); the first run fetches
# the chordmap crate from GitHub.
set -euo pipefail
cd "$(dirname "$0")"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null || { echo "cargo not found: install Rust with rustup"; exit 1; }
installed="$(rustup target list --installed 2>/dev/null || true)"
slices=()
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  if echo "$installed" | grep -q "$t" || [ "$(uname -m)" = "x86_64" -a "$t" = "x86_64-apple-darwin" ]; then
    echo "chordmap-ffi: building $t"
    cargo build --release --target "$t" --quiet
    slices+=("target/$t/release/libchordmap_ffi.a")
  fi
done
[ "${#slices[@]}" -gt 0 ] || { echo "no Apple target installed for Rust"; exit 1; }
mkdir -p lib
if [ "${#slices[@]}" -gt 1 ]; then lipo -create "${slices[@]}" -output lib/libchordmap_ffi.a; else cp "${slices[0]}" lib/libchordmap_ffi.a; fi
echo "chordmap-ffi: lib/libchordmap_ffi.a ($(lipo -archs lib/libchordmap_ffi.a))"
