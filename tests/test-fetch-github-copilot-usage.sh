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
temporary_dir="$(mktemp -d)"
trap 'rm -f "$error_file"; rm -rf "$temporary_dir"' EXIT

if "$root/scripts/fetch-github-copilot-usage" --html-file "$missing_usage" --no-cache \
    >/dev/null 2>"$error_file"; then
    echo "Expected parsing a page without usage to fail" >&2
    exit 1
fi

grep -Fq "Could not find Copilot usage" "$error_file"

agent_browser_log="$temporary_dir/agent-browser.log"
agent_browser="$temporary_dir/agent-browser"
cat >"$agent_browser" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

session=""
while (($#)); do
    case "$1" in
        --session)
            session="$2"
            shift 2
            ;;
        --profile)
            shift 2
            ;;
        open)
            printf 'open:%s\n' "$session" >>"$AGENT_BROWSER_LOG"
            exit 0
            ;;
        read)
            printf 'read:%s\n' "$session" >>"$AGENT_BROWSER_LOG"
            exit "${AGENT_BROWSER_READ_EXIT:-0}"
            ;;
        close)
            printf 'close:%s\n' "$session" >>"$AGENT_BROWSER_LOG"
            exit 0
            ;;
        *)
            shift
            ;;
    esac
done
EOF
chmod +x "$agent_browser"

if PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_READ_EXIT=1 \
    "$root/scripts/fetch-github-copilot-usage" --no-cache \
    >/dev/null 2>"$error_file"; then
    echo "Expected a browser read failure to fail" >&2
    exit 1
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${#browser_calls[@]}" == 3 ]] || {
    echo "Expected open, read, and cleanup close calls" >&2
    exit 1
}
[[ "${browser_calls[0]}" =~ ^open:copilot-powerline-github-usage-[0-9]+$ ]] || {
    echo "Expected a process-specific browser session name" >&2
    exit 1
}
[[ "${browser_calls[1]}" == "read:${browser_calls[0]#open:}" ]]
[[ "${browser_calls[2]}" == "close:${browser_calls[0]#open:}" ]]
