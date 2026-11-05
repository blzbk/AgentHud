#!/bin/sh
# AgentHud statusline for Claude Code (Linux, WSL, macOS, Git Bash).
#
# Claude Code pipes its session JSON to this script on each update. When it
# carries `rate_limits` (Pro, Max and Ultra plans, after the first answer),
# the script copies them to
#   ${CLAUDE_CONFIG_DIR:-$HOME/.claude}/agenthud/rate-limits.json
# which AgentHud reads. Nothing is sent anywhere.
#
# Install, in ~/.claude/settings.json:
#   "statusLine": { "type": "command", "command": "sh /path/to/agenthud-statusline.sh" }
# Keep an existing statusline by passing its command as the only argument:
#   "command": "sh /path/to/agenthud-statusline.sh '~/.claude/statusline.sh'"
# It gets the same input, and its output is shown.

input=$(cat)

dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/agenthud"
file="$dir/rate-limits.json"

write_rate_limits() {
    mkdir -p "$dir" || return
    tmp="$file.$$.tmp"
    if command -v jq >/dev/null 2>&1; then
        printf '%s' "$input" | jq -ce 'select(.rate_limits != null) | {rate_limits}' >"$tmp" 2>/dev/null
    else
        # Without jq, keep the input as is: AgentHud reads only rate_limits.
        case "$input" in
            *'"rate_limits"'*) printf '%s\n' "$input" >"$tmp" ;;
            *) false ;;
        esac
    fi && mv -f "$tmp" "$file"
    rm -f "$tmp"
}

write_rate_limits

if [ $# -gt 0 ]; then
    printf '%s' "$input" | sh -c "$1"
fi
