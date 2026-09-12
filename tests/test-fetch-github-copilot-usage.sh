#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$root/tests/fixtures/copilot-features-usage.txt"

actual="$("$root/scripts/fetch-github-copilot-usage" --html-file "$fixture" --no-cache)"
expected='{"ai_credits_used":49854,"usd":"498.54","cycle":"September 1-30, 2026"}'

[[ "$actual" == "$expected" ]] || {
    echo "Expected: $expected" >&2
    echo "Actual:   $actual" >&2
    exit 1
}

missing_usage="$root/tests/fixtures/copilot-features-no-usage.txt"
error_file="$(mktemp)"
trap 'rm -f "$error_file"' EXIT

if "$root/scripts/fetch-github-copilot-usage" --html-file "$missing_usage" --no-cache \
    >/dev/null 2>"$error_file"; then
    echo "Expected parsing a page without usage to fail" >&2
    exit 1
fi

grep -Fq "Could not find Copilot usage" "$error_file"
