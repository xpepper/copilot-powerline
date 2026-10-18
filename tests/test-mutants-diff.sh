#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/mutants-diff"
work="$(cd "$(mktemp -d)" && pwd -P)"
trap 'rm -rf "$work"' EXIT

# A repo with `main` and a branch that changes one Rust file.
repo="$work/repo"
git init -q -b main "$repo"
git -C "$repo" config user.email test@example.com
git -C "$repo" config user.name test
echo 'fn a() {}' > "$repo/lib.rs"
git -C "$repo" add lib.rs
git -C "$repo" commit -q -m base
git -C "$repo" switch -q -c feature
echo 'fn b() {}' >> "$repo/lib.rs"
git -C "$repo" commit -q -am change

# A fake cargo that records its arguments and HOME, then exits $FAKE_EXIT.
fake_cargo="$work/cargo"
cat > "$fake_cargo" <<EOF
#!/usr/bin/env bash
echo "\$*" > "$work/args"
echo "\$HOME" > "$work/home"
echo "\${RUSTUP_HOME:-}" > "$work/rustup_home"
exit "\${FAKE_EXIT:-0}"
EOF
chmod +x "$fake_cargo"

run() {
  (cd "$repo" && env -u RUSTUP_HOME CARGO="$fake_cargo" bash "$script" "$@")
}

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

# A clean branch runs cargo mutants on the diff against the base.
run main || fail "expected success on a clean branch"
args="$(cat "$work/args")"
diff_file="$repo/target/mutants-diff/pr.diff"
[ "$args" = "mutants --in-diff $diff_file --test-tool nextest -j 4" ] ||
  fail "unexpected cargo arguments: $args"
grep -q '^+fn b() {}$' "$diff_file" || fail "diff lacks the branch's change"
grep -q '^+fn a() {}$' "$diff_file" && fail "diff includes the base's own lines"
[ "$(cat "$work/home")" = "$repo/target/mutants-home" ] ||
  fail "HOME is not the throwaway home: $(cat "$work/home")"
[ "$(cat "$work/rustup_home")" = "$HOME/.rustup" ] ||
  fail "RUSTUP_HOME does not point at the real toolchains"

# A surviving mutant (cargo mutants exits 2) fails the script.
rm -f "$work/args"
if FAKE_EXIT=2 run; then
  fail "expected failure when cargo mutants reports a missed mutant"
fi
[ -f "$work/args" ] || fail "cargo mutants did not run"

# Uncommitted changes would not be in the diff, so the script refuses.
echo 'fn c() {}' >> "$repo/lib.rs"
rm -f "$work/args"
if run 2> "$work/stderr"; then
  fail "expected refusal on a dirty tree"
fi
grep -q 'commit' "$work/stderr" || fail "refusal does not say to commit"
[ ! -f "$work/args" ] || fail "cargo mutants ran on a dirty tree"

echo "PASS: mutants-diff"
