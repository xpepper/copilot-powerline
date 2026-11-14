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

#[test]
fn rust_version_matches_pinned_toolchain() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml must be readable");
    let manifest: toml::Value = toml::from_str(&manifest).expect("Cargo.toml must be valid TOML");
    let rust_version = manifest["package"]["rust-version"]
        .as_str()
        .expect("package.rust-version must be set in Cargo.toml");

    let toolchain =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/rust-toolchain.toml"))
            .expect("rust-toolchain.toml must be readable");
    let toolchain: toml::Value =
        toml::from_str(&toolchain).expect("rust-toolchain.toml must be valid TOML");
    let channel = toolchain["toolchain"]["channel"]
        .as_str()
        .expect("toolchain.channel must be set in rust-toolchain.toml");

    let matches = channel == rust_version
        || channel
            .strip_prefix(rust_version)
            .is_some_and(|rest| rest.starts_with('.'));
    assert!(
        matches,
        "Cargo.toml rust-version ({rust_version}) must match rust-toolchain.toml channel ({channel})"
    );
}
