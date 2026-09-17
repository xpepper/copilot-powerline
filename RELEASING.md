# Releasing copilot-powerline

`copilot-powerline` is released to two channels from the same version:

- crates.io, for `cargo install copilot-powerline`
- GitHub Releases with prebuilt binaries, a shell installer, and checksums, built by [dist](https://github.com/axodotdev/cargo-dist) (`.github/workflows/release.yml`, configured in `dist-workspace.toml`)

Prebuilt targets are macOS and Linux on arm64 and x86_64.

On each release the workflow also pushes an updated formula to [xpepper/homebrew-tap](https://github.com/xpepper/homebrew-tap). This needs a `HOMEBREW_TAP_TOKEN` repository secret: a token with write access to the tap repository's contents. Do not advertise a target as supported unless CI verifies it.

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
   dist plan
   ```

   If you changed `dist-workspace.toml` or upgraded dist, run `dist generate` and commit the regenerated workflow; the release workflow fails when it is out of date.

5. Open and merge the version-bump pull request.

## Publish and tag

From the merged release commit on `main`, publish to crates.io. Publishing must succeed before creating the tag so every release tag identifies an available crate version.

```bash
cargo publish
```

After crates.io shows the new version, replace the example version and push an annotated tag:

```bash
VERSION=0.3.2
git tag -a "v$VERSION" -m "v$VERSION"
git push origin "v$VERSION"
```

Pushing the tag starts the Release workflow, which builds every target, creates the GitHub Release with the binaries, `copilot-powerline-installer.sh`, and checksums, and publishes the Homebrew formula. Do not create the release by hand with `gh release create`: the workflow creates it.

When the workflow has finished, append GitHub's generated notes to the release body:

```bash
NOTES=$(gh api "repos/xpepper/copilot-powerline/releases/generate-notes" -f tag_name="v$VERSION" --jq .body)
BODY=$(gh release view "v$VERSION" --json body --jq .body)
gh release edit "v$VERSION" --notes "$BODY

$NOTES"
```

## After publishing

Confirm that the crates.io version and GitHub tag agree, and that the installer works:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/xpepper/copilot-powerline/releases/latest/download/copilot-powerline-installer.sh | sh
copilot-powerline --version
brew upgrade xpepper/tap/copilot-powerline || brew install xpepper/tap/copilot-powerline
```

Then update any launch or distribution material to use the published version and the installation commands in the README.
