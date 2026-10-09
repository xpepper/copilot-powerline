#!/usr/bin/env bash
# Automates the deterministic steps of RELEASING.md.
#
#   scripts/release.sh prepare X.Y.Z   bump the version, run the release checks,
#                                      push a branch and open the bump PR
#   scripts/release.sh tag             after the bump PR is merged: tag main,
#                                      push the tag, watch the release run,
#                                      verify crates.io, append generated notes
#
# Merging the PR stays manual on purpose: it is the human review point.
set -euo pipefail

REPO="xpepper/copilot-powerline"
CRATE="copilot-powerline"

die() { echo "error: $*" >&2; exit 1; }

require_clean_tree() {
  [ -z "$(git status --porcelain)" ] || die "working tree is not clean"
}

crate_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1
}

prepare() {
  local version="${1:-}"
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 prepare X.Y.Z"
  require_clean_tree
  [ "$(git branch --show-current)" = "main" ] || die "run from the main branch"
  git pull --ff-only
  [ "$(crate_version)" != "$version" ] || die "Cargo.toml is already at $version"
  ! git rev-parse -q --verify "refs/tags/v$version" >/dev/null || die "tag v$version already exists"

  git checkout -b "chore/release-$version"
  VERSION="$version" perl -0pi -e 's/^version = ".*"/version = "$ENV{VERSION}"/m' Cargo.toml
  cargo build --quiet # refreshes Cargo.lock

  cargo fmt --all --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  cargo build --release
  dist plan >/dev/null

  git add Cargo.toml Cargo.lock
  git commit -m "chore(release): bump version to $version"
  cargo publish --dry-run # needs the clean tree the commit just created

  git push -u origin "chore/release-$version"
  gh pr create --base main \
    --title "chore(release): bump version to $version" \
    --body "TL;DR: bump to $version. Release checks passed locally (fmt, clippy, test, release build, publish dry-run, dist plan).

Merge, then run \`scripts/release.sh tag\`."
}

tag() {
  require_clean_tree
  [ "$(git branch --show-current)" = "main" ] || die "run from the main branch"
  git pull --ff-only
  local version tag_name
  version="$(crate_version)"
  tag_name="v$version"
  ! git rev-parse -q --verify "refs/tags/$tag_name" >/dev/null || die "tag $tag_name already exists"
  git log -1 --format='%s' | grep -q "bump version to $version" ||
    die "HEAD is not the version bump commit for $version"

  git tag -a "$tag_name" -m "$tag_name"
  git push origin "$tag_name"

  local run_id=""
  for _ in $(seq 1 20); do
    run_id="$(gh run list --workflow=release.yml --branch "$tag_name" --limit 1 --json databaseId --jq '.[0].databaseId // empty')"
    [ -n "$run_id" ] && break
    sleep 3
  done
  [ -n "$run_id" ] || die "release run for $tag_name did not start"
  gh run watch "$run_id" --exit-status || die "release run failed: re-run the failed jobs (safe to repeat)"

  curl -fsS -H "User-Agent: $CRATE-release-script" \
    "https://crates.io/api/v1/crates/$CRATE/$version" >/dev/null ||
    die "$version is not on crates.io"
  echo "crates.io has $version"

  local notes body
  notes="$(gh api "repos/$REPO/releases/generate-notes" -f tag_name="$tag_name" --jq .body)"
  body="$(gh release view "$tag_name" --json body --jq .body)"
  gh release edit "$tag_name" --notes "$body

$notes"
  echo "Released $tag_name"
}

case "${1:-}" in
  prepare) shift; prepare "$@" ;;
  tag) tag ;;
  *) die "usage: $0 prepare X.Y.Z | tag" ;;
esac
