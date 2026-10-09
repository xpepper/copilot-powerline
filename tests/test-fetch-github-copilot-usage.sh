#!/usr/bin/env bash
# Word-splitting the log is intended; mapfile is missing from macOS bash 3.2.
# shellcheck disable=SC2207
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/fetch-github-copilot-usage"
fixture="$root/tests/fixtures/copilot-features-usage.txt"

api_dir="$(mktemp -d)"
trap 'rm -rf "$api_dir"' EXIT

# Stub curl: records its arguments and stdin (the headers), then prints
# $CURL_RESPONSE_FILE or exits with $CURL_EXIT.
cat >"$api_dir/curl" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$CURL_ARGS_LOG"
cat >"$CURL_STDIN_LOG"
[[ "${CURL_EXIT:-0}" == 0 ]] || exit "$CURL_EXIT"
cat "$CURL_RESPONSE_FILE"
EOF
# Stub gh: `gh auth token` prints $GH_AUTH_TOKEN, or fails when it is unset.
cat >"$api_dir/gh" <<'EOF'
#!/usr/bin/env bash
[[ "${1:-} ${2:-}" == "auth token" && -n "${GH_AUTH_TOKEN:-}" ]] || exit 1
printf '%s\n' "$GH_AUTH_TOKEN"
EOF
chmod +x "$api_dir/curl" "$api_dir/gh"

# Runs the script against the stubbed API with no ambient tokens. Leading
# NAME=value arguments set the environment, e.g. GH_AUTH_TOKEN=abc.
run_api() {
    env -u COPILOT_GITHUB_TOKEN -u GH_TOKEN -u GITHUB_TOKEN \
        PATH="$api_dir:$PATH" \
        CURL_ARGS_LOG="$api_dir/curl-args.log" \
        CURL_STDIN_LOG="$api_dir/curl-stdin.log" \
        CURL_RESPONSE_FILE="$response" \
        "$@" "$script" --no-cache --no-history
}

expect_output() {
    local expected="$1" actual
    shift
    actual="$(run_api "$@")"
    [[ "$actual" == "$expected" ]] || {
        echo "Expected: $expected" >&2
        echo "Actual:   $actual" >&2
        exit 1
    }
}

expect_failure() {
    local message="$1" output
    shift
    if output="$(run_api "$@" 2>"$api_dir/stderr")"; then
        echo "Expected a failure mentioning '$message', got: $output" >&2
        exit 1
    fi
    [[ -z "$output" ]] || {
        echo "Expected no output on failure, got: $output" >&2
        exit 1
    }
    grep -Fq "$message" "$api_dir/stderr" || {
        echo "Expected stderr to mention '$message', got: $(<"$api_dir/stderr")" >&2
        exit 1
    }
}

# Writes the unlimited fixture with a jq edit applied, then points $response at it.
response_with() {
    response="$api_dir/response.json"
    jq "$1" "$root/tests/fixtures/copilot-user-unlimited.json" >"$response"
}

response="$root/tests/fixtures/copilot-user-unlimited.json"
expect_output '{"ai_credits_used":15649,"usd":"156.49","cycle":"October 1-31, 2026"}' \
    GH_AUTH_TOKEN=gh-cli-token
grep -Fxq "https://api.github.com/copilot_internal/user" "$api_dir/curl-args.log" || {
    echo "Expected curl to call the Copilot user API, got: $(<"$api_dir/curl-args.log")" >&2
    exit 1
}
grep -Fxq "Authorization: Bearer gh-cli-token" "$api_dir/curl-stdin.log" || {
    echo "Expected the token in a header on stdin, got: $(<"$api_dir/curl-stdin.log")" >&2
    exit 1
}
if grep -Fq "gh-cli-token" "$api_dir/curl-args.log"; then
    echo "Expected the token to stay out of curl's arguments" >&2
    exit 1
fi

# Copilot CLI's documented token order.
expect_token() {
    local expected="$1"
    shift
    run_api "$@" >/dev/null
    grep -Fxq "Authorization: Bearer $expected" "$api_dir/curl-stdin.log" || {
        echo "Expected token $expected, got: $(<"$api_dir/curl-stdin.log")" >&2
        exit 1
    }
}
expect_token copilot COPILOT_GITHUB_TOKEN=copilot GH_TOKEN=gh GITHUB_TOKEN=github GH_AUTH_TOKEN=cli
expect_token gh GH_TOKEN=gh GITHUB_TOKEN=github GH_AUTH_TOKEN=cli
expect_token github GITHUB_TOKEN=github GH_AUTH_TOKEN=cli

# A per-user budget counts entitlement - quota_remaining, not credits_used.
response="$root/tests/fixtures/copilot-user-budget.json"
expect_output '{"ai_credits_used":2000,"usd":"20.00","cycle":"February 1-29, 2028"}' \
    GH_AUTH_TOKEN=token

# The cycle is the calendar month before quota_reset_date.
for case in "2026-01-01|December 1-31, 2025" \
    "2026-07-01|June 1-30, 2026" \
    "2026-12-01|November 1-30, 2026" \
    "2100-03-01|February 1-28, 2100" \
    "2400-03-01|February 1-29, 2400"; do
    response_with ".quota_reset_date = \"${case%%|*}\""
    expect_output "{\"ai_credits_used\":15649,\"usd\":\"156.49\",\"cycle\":\"${case#*|}\"}" \
        GH_AUTH_TOKEN=token
done

response="$root/tests/fixtures/copilot-user-unlimited.json"
expect_failure "gh auth login"
expect_failure "GitHub API request failed" GH_AUTH_TOKEN=token CURL_EXIT=22

response_with 'del(.quota_snapshots.premium_interactions.credits_used)'
expect_failure "credits_used" GH_AUTH_TOKEN=token
response_with 'del(.quota_snapshots.premium_interactions)'
expect_failure "premium_interactions" GH_AUTH_TOKEN=token
response_with '.quota_snapshots.premium_interactions += {unlimited: false, entitlement: 3000}'
expect_failure "quota_remaining" GH_AUTH_TOKEN=token
response_with 'del(.quota_reset_date)'
expect_failure "quota_reset_date" GH_AUTH_TOKEN=token
response_with '.quota_reset_date = "2026-11-15"'
expect_failure "quota_reset_date" GH_AUTH_TOKEN=token
response="$api_dir/invalid.json"
printf 'not json' >"$response"
expect_failure "not valid JSON" GH_AUTH_TOKEN=token

actual="$("$script" --html-file "$fixture" --no-cache --no-history)"
expected='{"ai_credits_used":49854,"usd":"498.54","cycle":"September 1-30, 2026"}'

[[ "$actual" == "$expected" ]] || {
    echo "Expected: $expected" >&2
    echo "Actual:   $actual" >&2
    exit 1
}

missing_usage="$root/tests/fixtures/copilot-features-no-usage.txt"
error_file="$(mktemp)"
temporary_dir="$(mktemp -d)"
trap 'rm -f "$error_file"; rm -rf "$api_dir" "$temporary_dir"' EXIT

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
        session)
            [[ "${2:-}" == list ]] || exit 1
            printf 'Active sessions:\n'
            [[ -z "${AGENT_BROWSER_SESSIONS:-}" ]] || printf '  %s\n' $AGENT_BROWSER_SESSIONS
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
    "$root/scripts/fetch-github-copilot-usage" --browser --no-cache --no-history \
    >/dev/null 2>"$error_file"; then
    echo "Expected a browser read failure to fail" >&2
    exit 1
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${#browser_calls[@]}" == 3 ]] || {
    echo "Expected open, read, and cleanup close calls" >&2
    exit 1
}
[[ "${browser_calls[0]}" =~ ^open:copilot-powerline-github-usage-[0-9]+-[0-9]+$ ]] || {
    echo "Expected a session name with the run PID and start-time token" >&2
    exit 1
}
[[ "${browser_calls[1]}" == "read:${browser_calls[0]#open:}" ]]
[[ "${browser_calls[2]}" == "close:${browser_calls[0]#open:}" ]]

: >"$agent_browser_log"
if PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_OPEN_EXIT=1 \
    "$root/scripts/fetch-github-copilot-usage" --browser --no-cache --no-history \
    >/dev/null 2>"$error_file"; then
    echo "Expected a browser open failure to fail" >&2
    exit 1
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${#browser_calls[@]}" == 2 && "${browser_calls[1]}" == "close:${browser_calls[0]#open:}" ]] || {
    echo "Expected a failed open to close its browser session, got: ${browser_calls[*]}" >&2
    exit 1
}

bash -c 'exit 0' &
finished_pid=$!
wait "$finished_pid"
stale_session="copilot-powerline-github-usage-$finished_pid-1"
# Mirrors run_token in the script: checksum of the process start time.
running_token="$(printf '%s' "$(ps -o lstart= -p $$)" | cksum | cut -d' ' -f1)"
running_session="copilot-powerline-github-usage-$$-$running_token"
# Same PID as a live process, but a different start time: the PID was reused.
reused_pid_session="copilot-powerline-github-usage-$$-$((running_token + 1))"

: >"$agent_browser_log"
if ! PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_SESSIONS="$stale_session $reused_pid_session $running_session unrelated-session" \
    "$root/scripts/fetch-github-copilot-usage" --browser --no-cache --no-history \
    >/dev/null 2>"$error_file"; then
    : # The fake page has no usage; only the browser calls matter here.
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${browser_calls[0]:-}" == "close:$stale_session" &&
    "${browser_calls[1]:-}" == "close:$reused_pid_session" &&
    "${browser_calls[2]:-}" == open:* ]] || {
    echo "Expected stale and reused-PID sessions to be closed before opening, got: ${browser_calls[*]}" >&2
    exit 1
}
for call in "${browser_calls[@]}"; do
    [[ "$call" != "close:$running_session" && "$call" != "close:unrelated-session" ]] || {
        echo "Expected only stale script sessions to be closed, got: ${browser_calls[*]}" >&2
        exit 1
    }
done

: >"$agent_browser_log"
PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_SESSIONS="$stale_session" \
    "$root/scripts/fetch-github-copilot-usage" --login >/dev/null

browser_calls=($(<"$agent_browser_log"))
[[ "${browser_calls[0]:-}" == "close:$stale_session" && "${browser_calls[1]:-}" == open:* ]] || {
    echo "Expected --login to close the stale session before opening, got: ${browser_calls[*]}" >&2
    exit 1
}

: >"$agent_browser_log"
if PATH="$temporary_dir:$PATH" \
    AGENT_BROWSER_LOG="$agent_browser_log" \
    AGENT_BROWSER_OPEN_EXIT=1 \
    "$root/scripts/fetch-github-copilot-usage" --login >/dev/null 2>"$error_file"; then
    echo "Expected a failed --login open to fail" >&2
    exit 1
fi

browser_calls=($(<"$agent_browser_log"))
[[ "${#browser_calls[@]}" == 2 && "${browser_calls[1]}" == "close:${browser_calls[0]#open:}" ]] || {
    echo "Expected a failed --login open to close its browser session, got: ${browser_calls[*]}" >&2
    exit 1
}

history_dir="$(mktemp -d)"
history_file="$history_dir/github-usage-history.jsonl"
trap 'rm -f "$error_file"; rm -rf "$api_dir" "$temporary_dir" "$history_dir"' EXIT

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
