# Releases

OrbyNode publishes signed releases for six first-class targets. This page covers the release process, verification, and upgrade/rollback workflow.

## Supported targets

| Target | Platform | Notes |
|---|---|---|
| `x86_64-unknown-linux-gnu` | Linux x86_64 | glibc 2.17+ |
| `x86_64-unknown-linux-musl` | Linux x86_64 | static, musl |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | glibc 2.17+ |
| `x86_64-apple-darwin` | macOS Intel | macOS 11+ |
| `aarch64-apple-darwin` | macOS Apple Silicon | macOS 11+ |
| `x86_64-pc-windows-msvc` | Windows x86_64 | Windows 10+ |

Windows ARM64 builds are provided as best-effort.

## What every release ships

- Platform archives (`.tar.gz` for Linux/macOS, `.zip` for Windows)
- `SHA256SUMS` file covering all artifacts
- Cosign signatures for each archive
- SBOM (Software Bill of Materials) in SPDX format
- Standalone installer scripts (`orbynode-install.sh`, `orbynode-install.ps1`) with their own checksums
- Auto-generated changelog comparing against the previous release

## Verification

### Checksum verification

```bash
# Download SHA256SUMS and the archive for your platform
sha256sum -c SHA256SUMS 2>&1 | grep orbynode-<version>-<platform>
```

### Signature verification

With [Cosign](https://docs.sigstore.dev/cosign/system_config/installation/) installed:

```bash
cosign verify-blob \
  --certificate-identity https://github.com/PotenFYR-Studios/OrbyNode/.github/workflows/release.yml@refs/tags/v<version> \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --bundle orbynode-<version>-<platform>-<arch>.tar.gz.bundle \
  orbynode-<version>-<platform>-<arch>.tar.gz
```

The release workflow signs with GitHub OIDC (keyless); no long-lived signing keys exist. If `cosign` is not on PATH, the installer falls back to checking signature presence only.

## Install, upgrade, rollback

### Fresh install

```bash
curl -fsSL https://orbynode.docs.potenfyr.in/install.sh | sh
```

See [installation.md](installation.md) for the full installer reference.

### Upgrade

1. Run the installer again — it replaces binaries in place
2. Restart the daemon
3. OrbyNode runs schema migrations on startup

Configuration and durable state (SQLite database, agent journals) are preserved across upgrades. The installer never touches `~/.orbynode`.

### Rollback

Install the previous version explicitly:

```bash
ORBYNODE_VERSION=v1.0.0 curl -fsSL https://orbynode.docs.potenfyr.in/install.sh | sh
```

The database schema is backward-compatible within a major version, so rolling back N → N-1 is safe for data. Feature flags or new configuration variables added in N are simply ignored by N-1.

### Clean uninstall

Linux/macOS:

```bash
~/.local/share/orbynode/uninstall.sh          # binaries only, keeps data
~/.local/share/orbynode/uninstall.sh --purge  # binaries + ~/.orbynode (irreversible)
```

Windows:

```powershell
& "$env:LOCALAPPDATA\Programs\orbynode\bin\uninstall.ps1"          # binaries only
& "$env:LOCALAPPDATA\Programs\orbynode\bin\uninstall.ps1" -Purge   # binaries + data
```

The uninstaller stops the daemon gracefully before removing files. Data is kept by default so configuration and durable state survive reinstall.

## Release flow

### Continuous

Every push to `main` builds the full release matrix and publishes artifacts to the workflow run (no GitHub Release is created). This catches packaging breakage immediately.

### Tagged

Pushing a `v*` tag builds the extended matrix and publishes a GitHub Release with all artifacts, signatures, and a generated changelog.

```bash
git tag v1.0.0
git push origin v1.0.0
```

Same-tag re-runs replace assets in place and regenerate the changelog body — no duplicate releases accumulate.

### Local release build

```bash
scripts/release.sh <rust-target-triple>
```

Produces the archive, checksum, and SBOM locally (signing requires CI). See [development.md](development.md) for details.

## v1.0 release gates

Before the v1.0 label, these clean-machine guarantees must be verified (see [v1-readiness.md](v1-readiness.md)):

1. Clean-machine install test on all six first-class targets
2. Upgrade/rollback exercise: install N-1 → upgrade to N → verify state survives → rollback → clean uninstall
3. Tauri desktop package validation

## Provenance

Release builds run on GitHub Actions with `id-token: write`, producing Sigstore provenance attestations. Anyone can verify that a release binary was built from the exact commit tagged:

```bash
cosign verify-attestation \
  --certificate-identity https://github.com/PotenFYR-Studios/OrbyNode/.github/workflows/release.yml@refs/tags/v<version> \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  PotenFYR-Studios/OrbyNode
```
