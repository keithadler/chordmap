# Security

chordmap takes untrusted audio as input. The core crate forbids `unsafe`
and is fuzz-friendly (no I/O, plain slices in). If you find a crash, a hang
or anything that looks like a memory issue, email keith.adler@icloud.com
and allow a few days before publishing details.
