# AgentHud statusline for Claude Code (Windows PowerShell 5.1 or later).
#
# Claude Code pipes its session JSON to this script on each update. When it
# carries `rate_limits` (Pro, Max and Ultra plans, after the first answer),
# the script copies them to
#   $env:CLAUDE_CONFIG_DIR (default: ~\.claude)\agenthud\rate-limits.json
# which AgentHud reads. Nothing is sent anywhere.
#
# Install, in %USERPROFILE%\.claude\settings.json (forward slashes):
#   "statusLine": { "type": "command",
#     "command": "powershell -NoProfile -ExecutionPolicy Bypass -File C:/path/to/agenthud-statusline.ps1" }
# Keep an existing statusline by passing its command with -Chain:
#   ... -File C:/path/to/agenthud-statusline.ps1 -Chain \"powershell -NoProfile -File C:/Users/me/.claude/statusline.ps1\"
# It gets the same input, through cmd.exe, and its output is shown.

param([string]$Chain = "")

$raw = [Console]::In.ReadToEnd()

try {
    $session = $raw | ConvertFrom-Json
    if ($null -ne $session.rate_limits) {
        $configDir = if ($env:CLAUDE_CONFIG_DIR) { $env:CLAUDE_CONFIG_DIR } else { Join-Path $HOME ".claude" }
        $dir = Join-Path $configDir "agenthud"
        $file = Join-Path $dir "rate-limits.json"
        $tmp = "$file.$PID.tmp"
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $json = @{ rate_limits = $session.rate_limits } | ConvertTo-Json -Depth 5 -Compress
        # UTF-8 without BOM: Windows PowerShell 5 adds one with Set-Content.
        [IO.File]::WriteAllText($tmp, $json, (New-Object Text.UTF8Encoding $false))
        Move-Item -Force -Path $tmp -Destination $file
    }
} catch {
    # A statusline must never fail: AgentHud keeps the previous quotas.
}

if ($Chain) {
    $raw | cmd.exe /d /c $Chain
}
