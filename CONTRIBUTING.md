# Contributing

- `cargo test --release --workspace` must pass; `cargo fmt` and
  `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
- New behaviour comes with a synthetic test in `crates/chordmap/tests` or a
  unit test next to the code. Do not add real songs to the repository.
- Keep the core crate free of I/O and `unsafe`.
- Keep the wasm TypeScript types in `crates/chordmap-wasm/src/lib.rs`, the
  README and `docs/limitations.md` in step with the Rust API.
- Plain words in UI strings and docs: say what it cannot do.
