# Releasing copilot-powerline

`copilot-powerline` uses a Cargo-first release path. The recommended user installation command is:

```bash
cargo install copilot-powerline
```

GitHub Releases document published versions; they do not currently provide prebuilt binaries. Do not advertise a target as supported unless CI verifies it.

## Prepare a release

1. Start from an up-to-date `main` branch with the required CI checks passing.
2. Update the package version in `Cargo.toml` and any user-facing version references.
3. Review the README installation instructions and compatibility notes.
4. Run the release checks:

   ```bash
   cargo fmt --all --check
   cargo clippy --all-targets -- -D warnings
   cargo test --verbose
   cargo build --release --verbose
   cargo publish --dry-run
   ```

5. Open and merge the version-bump pull request.

## Publish and tag

From the merged release commit on `main`, publish to crates.io. Publishing must succeed before creating the tag so every release tag identifies an available crate version.

```bash
cargo publish
```

After crates.io shows the new version, replace the example version and create an annotated tag and GitHub Release:

```bash
VERSION=0.2.1
git tag -a "v$VERSION" -m "v$VERSION"
git push origin "v$VERSION"
gh release create "v$VERSION" --title "v$VERSION" --generate-notes
```

The release contains generated notes and source archives. It has no binary artifacts or checksums because this project currently distributes through crates.io.

## After publishing

Confirm that the crates.io version and GitHub tag agree, then update any launch or distribution material to use the published version and the canonical Cargo installation command.
