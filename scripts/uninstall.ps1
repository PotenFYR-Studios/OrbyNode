# OrbyNode uninstaller for Windows.
# Stops the daemon, removes the installed binary, and asks whether to keep
# configuration and data (%USERPROFILE%\.orbynode) for reuse after reinstall.
# Data removal is opt-in via -Purge.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1 [-Purge] [-Force]

[CmdletBinding()]
param(
    [switch]$Purge,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$InstallDir = if ($env:ORBYNODE_INSTALL_DIR) { $env:ORBYNODE_INSTALL_DIR }
              else { Join-Path $env:LOCALAPPDATA 'Programs\orbynode\bin' }
$DataDir = if ($env:ORBYNODE_DATA_DIR) { $env:ORBYNODE_DATA_DIR }
           else { Join-Path $env:USERPROFILE '.orbynode' }

function Log([string]$Message) { Write-Host "==> $Message" }
function Warn([string]$Message) { Write-Warning $Message }

# 1. Stop a running daemon for this user, gracefully.
$daemon = Get-Process -Name 'orbynode', 'orbynode-daemon' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -like "$InstallDir*" }
if ($daemon) {
    foreach ($proc in $daemon) {
        Log "stopping daemon (pid $($proc.Id))"
        $proc | Stop-Process -ErrorAction SilentlyContinue
    }
    $daemon | Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
    $stillRunning = Get-Process -Name 'orbynode', 'orbynode-daemon' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like "$InstallDir*" }
    if ($stillRunning) {
        Warn 'daemon did not exit after 10s; forcing'
        $stillRunning | Stop-Process -Force -ErrorAction SilentlyContinue
    }
}
else {
    Log 'no running daemon found'
}

# 2. Remove installed binary.
$binary = Join-Path $InstallDir 'orbynode.exe'
if (Test-Path $binary) {
    Remove-Item -Force $binary
    Log "removed $binary"
}
else {
    Warn "no binary found at $binary"
}

# 3. Data directory: keep by default, purge only when asked.
if (Test-Path $DataDir) {
    $keep = $true
    if ($Purge) {
        $keep = $false
    }
    elseif (-not $Force) {
        # Interactive decision: config reuse is the default answer.
        $answer = Read-Host "Keep configuration and data in $DataDir for reuse after reinstall? [Y/n]"
        if ($answer -match '^[nN]') { $keep = $false }
    }

    if (-not $keep) {
        Remove-Item -Recurse -Force $DataDir
        Log "removed $DataDir (config, database, worktrees)"
    }
    else {
        Log "kept $DataDir (config preserved; reinstall will reuse it)"
    }
}
else {
    Log "no data directory at $DataDir"
}

Log 'uninstall complete'
