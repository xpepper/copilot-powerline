#[test]
fn rusqlite_keeps_sqlite_bundled() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml must be readable");
    let manifest: toml::Value = toml::from_str(&manifest).expect("Cargo.toml must be valid TOML");
    let features = manifest["dependencies"]["rusqlite"]["features"]
        .as_array()
        .expect("rusqlite must list its enabled features");

    assert!(
        features
            .iter()
            .any(|feature| feature.as_str() == Some("bundled")),
        "rusqlite must keep its bundled feature"
    );
}
