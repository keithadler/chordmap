// Passes the chordmap crate version through to the library, so `ampmac status` can say it.
fn main() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap_or_default();
    let mut version = "unknown".to_string();
    let mut in_pkg = false;
    for line in lock.lines() {
        if line.trim() == "name = \"chordmap\"" { in_pkg = true; continue; }
        if in_pkg && line.trim_start().starts_with("version = ") {
            version = line.split('"').nth(1).unwrap_or("unknown").to_string();
            break;
        }
    }
    println!("cargo:rustc-env=CHORDMAP_VERSION={version}");
    println!("cargo:rerun-if-changed=Cargo.lock");
}
