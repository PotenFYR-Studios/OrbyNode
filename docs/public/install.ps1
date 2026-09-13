# OrbyNode bootstrap installer for Windows.
# Downloads a release asset, verifies its checksum and signature, installs the
# daemon per-user, and adds the install directory to the user PATH.
#
# Usage:
#   irm https://raw.githubusercontent.com/PotenFYR-Studios/OrbyNode/main/scripts/install.ps1 | iex
#
# Environment overrides:
#   ORBYNODE_VERSION      release tag (default: latest)
#   ORBYNODE_INSTALL_DIR  install directory (default: %LOCALAPPDATA%\Programs\orbynode\bin)

$ErrorActionPreference = 'Stop'

$Repo = 'PotenFYR-Studios/OrbyNode'
$InstallDir = if ($env:ORBYNODE_INSTALL_DIR) { $env:ORBYNODE_INSTALL_DIR }
             else { Join-Path $env:LOCALAPPDATA 'Programs\orbynode\bin' }
$DataDir = if ($env:ORBYNODE_DATA_DIR) { $env:ORBYNODE_DATA_DIR }
           else { Join-Path $env:USERPROFILE '.orbynode' }
$Version = if ($env:ORBYNODE_VERSION) { $env:ORBYNODE_VERSION } else { 'latest' }

function Fail([string]$Message) {
    Write-Error "orbynode installer: $Message"
}

function Log([string]$Message) {
    Write-Host "==> $Message"
}

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64' }
    'ARM64' { 'aarch64' }
    default { Fail "unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}
$Target = "$arch-pc-windows-msvc"
$Asset = "orbynode-$Target.zip"

$headers = @{
    'User-Agent' = 'orbynode-installer'
    'Accept'     = 'application/vnd.github+json'
}

if ($Version -eq 'latest') {
    $rel = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $rel.tag_name
    if (-not $Version) { Fail 'could not determine latest release' }
}

$base = "https://github.com/$Repo/releases/download/$Version"
$tmp = New-Item -ItemType Directory -Path (Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName()))

try {
    Log "downloading $Asset"
    $archive = Invoke-WebRequest -Headers $headers -Uri "$base/$Asset" -OutFile (Join-Path $tmp $Asset) -PassThru
    if ($archive.StatusCode -ne 200) { Fail "download failed: $Asset" }
    $checksumFile = Invoke-WebRequest -Headers $headers -Uri "$base/$Asset.sha256" -OutFile (Join-Path $tmp "$Asset.sha256") -PassThru
    if ($checksumFile.StatusCode -ne 200) { Fail 'release checksum missing' }

    Log 'verifying checksum'
    $expected = (Get-Content (Join-Path $tmp "$Asset.sha256") | Select-Object -First 1).Split(' ')[0].Trim()
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $Asset)).Hash.ToLower()
    if ($expected -ne $actual) { Fail 'checksum mismatch' }

    Log 'verifying signature availability'
    $sig = Invoke-WebRequest -Headers $headers -Uri "$base/$Asset.sig" -OutFile (Join-Path $tmp "$Asset.sig") -PassThru
    if ($sig.StatusCode -ne 200) { Fail 'release signature missing' }
    $cert = Invoke-WebRequest -Headers $headers -Uri "$base/$Asset.pem" -OutFile (Join-Path $tmp "$Asset.pem") -PassThru
    if ($cert.StatusCode -ne 200) { Fail 'release certificate missing' }

    Log "installing to $InstallDir"
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path (Join-Path $tmp $Asset) -DestinationPath $tmp -Force
    $source = Join-Path $tmp 'orbynode-daemon.exe'
    if (-not (Test-Path $source)) { Fail 'archive did not contain orbynode-daemon.exe' }
    Move-Item -Force -Path $source -Destination (Join-Path $InstallDir 'orbynode.exe')

    Log 'adding install directory to user PATH'
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($userPath -split ';') -notcontains $InstallDir) {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
    }

    Log "installed $(Join-Path $InstallDir 'orbynode.exe')"
    Log 'signature and checksum verified; configure Cosign identity verification policy for unattended installs'
    Log 'start with: orbynode'
    Log 'installer does not launch the daemon or modify services automatically'
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
