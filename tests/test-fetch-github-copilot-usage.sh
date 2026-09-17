#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$root/tests/fixtures/copilot-features-usage.txt"

actual="$("$root/scripts/fetch-github-copilot-usage" --html-file "$fixture" --no-cache --no-history)"
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

if "$root/scripts/fetch-github-copilot-usage" --html-file "$missing_usage" --no-cache --no-history \
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
            exit "${AGENT_BROWSER_OPEN_EXIT:-0}"
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
    "$root/scripts/fetch-github-copilot-usage" --no-cache --no-history \
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

: >"$agent_browser_log"
if PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_OPEN_EXIT=1 \
    "$root/scripts/fetch-github-copilot-usage" --no-cache --no-history \
    >/dev/null 2>"$error_file"; then
    echo "Expected a browser open failure to fail" >&2
    exit 1
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${#browser_calls[@]}" == 2 && "${browser_calls[1]}" == "close:${browser_calls[0]#open:}" ]] || {
    echo "Expected a failed open to close its browser session, got: ${browser_calls[*]}" >&2
    exit 1
}

history_dir="$(mktemp -d)"
history_file="$history_dir/github-usage-history.jsonl"
trap 'rm -f "$error_file"; rm -rf "$temporary_dir" "$history_dir"' EXIT

COPILOT_USAGE_HISTORY_FILE="$history_file" \
    "$root/scripts/fetch-github-copilot-usage" --html-file "$fixture" --no-cache >/dev/null
COPILOT_USAGE_HISTORY_FILE="$history_file" \
    "$root/scripts/fetch-github-copilot-usage" --html-file "$fixture" --no-cache >/dev/null

[[ "$(wc -l <"$history_file" | tr -d ' ')" == "2" ]] || {
    echo "Expected two appended history entries, found:" >&2
    cat "$history_file" >&2
    exit 1
}

first_entry="$(sed -n '1p' "$history_file")"
[[ "$first_entry" == *'"ai_credits_used":49854'* ]] || {
    echo "Expected history entry to record ai_credits_used: $first_entry" >&2
    exit 1
}
[[ "$first_entry" == *'"usd":"498.54"'* ]] || {
    echo "Expected history entry to record usd: $first_entry" >&2
    exit 1
}
[[ "$first_entry" == *'"cycle":"September 1-30, 2026"'* ]] || {
    echo "Expected history entry to record cycle: $first_entry" >&2
    exit 1
}
[[ "$first_entry" =~ \"ts\":\"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\" ]] || {
    echo "Expected history entry to record an ISO8601 UTC timestamp: $first_entry" >&2
    exit 1
}

no_history_file="$history_dir/no-history.jsonl"
COPILOT_USAGE_HISTORY_FILE="$no_history_file" \
    "$root/scripts/fetch-github-copilot-usage" --html-file "$fixture" --no-cache --no-history >/dev/null

[[ -e "$no_history_file" ]] && {
    echo "Expected --no-history to skip writing the history file" >&2
    exit 1
}

exit 0
