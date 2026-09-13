# Installation

OrbyNode runs on Linux, macOS, and Windows (x86_64 and aarch64). Three supported install methods, all served from GitHub Releases.

## Requirements

- **Linux**: any distro with glibc 2.17+ or musl
- **macOS**: 11.0+ (Big Sur or later)
- **Windows**: Windows 10+ (64-bit)
- **Architecture**: x86_64 or aarch64 (ARM64)
- curl or wget (installer only)
- Optional: systemd for Linux service units

## Method 1 — installer script (recommended)

### Linux/macOS one-liner

```bash
curl -fsSL https://orbynode.docs.potenfyr.in/install.sh | sh
```

Same script, straight from GitHub Releases:

```bash
curl -fsSL https://github.com/PotenFYR-Studios/OrbyNode/releases/latest/download/orbynode-install.sh | sh
```

### Windows one-liner (PowerShell)

```powershell
irm https://orbynode.docs.potenfyr.in/install.ps1 | iex
```

Same script from GitHub Releases:

```powershell
irm https://github.com/PotenFYR-Studios/OrbyNode/releases/latest/download/orbynode-install.ps1 | iex
```

### Review-first (safer)

Linux/macOS:

```bash
curl -fLo install-orbynode.sh https://github.com/PotenFYR-Studios/OrbyNode/releases/latest/download/orbynode-install.sh
less install-orbynode.sh
sh install-orbynode.sh
```

Windows:

```powershell
Invoke-WebRequest -Uri https://github.com/PotenFYR-Studios/OrbyNode/releases/latest/download/orbynode-install.ps1 -OutFile install-orbynode.ps1
notepad install-orbynode.ps1
.\install-orbynode.ps1
```

### What the installer does

1. Detects OS and architecture (x86_64, aarch64)
2. Downloads the appropriate release archive
3. Verifies SHA-256 checksum
4. Checks Cosign signature presence (full keyless verification if `cosign` is on PATH)
5. Installs binaries to `~/.local/bin` (Linux/macOS) or `%LOCALAPPDATA%\Programs\orbynode\bin` (Windows)
6. Adds install location to PATH (Windows only; Linux/macOS users should add `~/.local/bin` manually)
7. Bundles uninstaller in `~/.local/share/orbynode/uninstall.sh` (Linux/macOS) or install directory (Windows)

The installer is **per-user** and does not require root/admin privileges. It does not automatically launch the daemon.

### Installer environment variables

| Variable | Default | Effect |
|---|---|---|
| `ORBYNODE_VERSION` | latest release | Pin a version: `ORBYNODE_VERSION=v1.0.0` |
| `ORBYNODE_IDENTITY` | (pinned) | Cosign certificate identity for signature verification |
| `ORBYNODE_ISSUER` | (pinned) | Cosign OIDC issuer for signature verification |

Pinned install:

```bash
ORBYNODE_VERSION=v1.0.0 sh install-orbynode.sh
```

### Uninstall

Linux/macOS:

```bash
~/.local/share/orbynode/uninstall.sh
```

To also remove configuration and data (irreversible):

```bash
~/.local/share/orbynode/uninstall.sh --purge
```

Windows:

```powershell
& "$env:LOCALAPPDATA\Programs\orbynode\bin\uninstall.ps1"
```

To purge data:

```powershell
& "$env:LOCALAPPDATA\Programs\orbynode\bin\uninstall.ps1" -Purge
```

## Method 2 — download and extract

1. Download the archive for your platform from [Releases](https://github.com/PotenFYR-Studios/OrbyNode/releases/latest)
2. Verify the checksum:
   ```bash
   sha256sum -c orbynode-<version>-<platform>-<arch>.tar.gz.sha256
   ```
3. Extract:
   ```bash
   tar -xzf orbynode-<version>-<platform>-<arch>.tar.gz
   ```
4. Move `orbynode-daemon` to a directory on your PATH
5. Run `orbynode-daemon`

## Method 3 — build from source

See [Development](development.md) for build requirements and instructions.

## First-run setup

1. Start the daemon:
   ```bash
   orbynode-daemon
   ```
2. Open <http://127.0.0.1:7676/setup>
3. Read the warning about terminal access (OrbyNode provides remote code execution capability)
4. Create the first Owner with a username, display name, and password
5. Sign in with the returned session cookie

The first Owner can be created only while the users table is empty.

## Configuration

OrbyNode reads configuration from environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `ORBYNODE_BIND` | `127.0.0.1:7676` | Bind address |
| `ORBYNODE_PORT` | `7676` | Port shorthand (overrides port in BIND) |
| `ORBYNODE_DATA_DIR` | `~/.orbynode` | SQLite database and durable state |
| `ORBYNODE_STATIC_DIR` | embedded | Serve `web/dist` from disk (development only) |
| `ORBYNODE_LOG_FORMAT` | `human` | Set to `json` for structured logs |
| `RUST_LOG` | `info` | Tracing filter (debug, info, warn, error) |

Example development daemon:

```bash
ORBYNODE_STATIC_DIR="$PWD/web/dist" RUST_LOG=debug orbynode-daemon
```

## Next steps

- Read [Architecture](../ARCHITECTURE.md)
- Review [Security](security-model.md)
- Track status in [Roadmap](../ROADMAP.md)
