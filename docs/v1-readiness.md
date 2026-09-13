# v1.0 Readiness

The engineering baseline now covers all roadmap milestones. A release must not
be labeled v1.0 until the following clean-machine guarantees are verified:

1. A one-command installer detects OS and architecture.
2. The installer verifies checksum and signature.
3. Installation is per-user and reversible.
4. First-run setup creates the initial Owner securely.
5. The daemon performs the standard persistent-agent workflow.
6. Upgrade preserves configuration and durable state.
7. Clean uninstall removes installed files without deleting user data unless
   explicitly requested.
8. Each of the six first-class targets passes the same clean-environment test.

Current artifacts provide locked builds, checksums, SBOMs, and Cosign
signatures. Installer bootstrapping and clean-machine test execution remain
release gate work.

`scripts/install.sh` now provides the bootstrap shape: OS and CPU detection,
release discovery, archive download, checksum verification, signature-presence
check, user-local installation and no automatic service launch.

Before v1.0, run that bootstrap in clean Linux and macOS environments, add the
Windows archive/PATH flow, enforce Cosign identity policy at install time, and
test upgrade, rollback and clean uninstall on every first-class target.

## Installer and uninstaller coverage

- `scripts/install.sh` (Linux/macOS): OS/CPU detection, checksum, Cosign
  keyless verification with pinned certificate identity
  (`ORBYNODE_IDENTITY`, `ORBYNODE_ISSUER`), per-user install, no service
  launch. Full verification requires `cosign` on PATH; without it only
  signature presence is checked. Bundled uninstaller lands in
  `~/.local/share/orbynode/uninstall.sh`.
- `scripts/install.ps1` (Windows): SHA-256 + signature/certificate presence,
  per-user install to `%LOCALAPPDATA%\Programs\orbynode\bin`, user PATH
  registration.
- `scripts/uninstall.sh` / `scripts/uninstall.ps1`: graceful daemon stop,
  binary removal, and an explicit keep-or-purge decision for `~/.orbynode`
  (kept by default so configuration and durable state survive reinstall).
- CI (`ci.yml`) gained a shellcheck job; release builds pin
  `bun install --frozen-lockfile` and install the aarch64-linux cross linker.

## Release flow

- Every push to `main` builds the release matrix (publish skipped);
  tags build the extended matrix and publish.
- Same-tag re-run replaces assets in place (`overwrite_files`) and
  regenerates the changelog body — no duplicate releases.
- Two install paths ship with every release: the curl bootstrap
  (`scripts/install.sh` / `install.ps1` in-repo) and standalone
  `orbynode-install.sh` / `.ps1` artifacts published to the release with
  checksums. Both verify SHA-256 and Cosign signatures.
- Target matrix: Linux x86_64 (gnu + musl), Linux aarch64, macOS x86_64,
  macOS aarch64, Windows x86_64, Windows aarch64.

## Remaining release gates

1. Execute the clean-machine install test on all six first-class targets
   (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/ARM64).
2. Upgrade/rollback exercise: install N-1, upgrade to N, verify config and
   database survive; rollback N to N-1; clean uninstall both ways.
3. Validate the Tauri desktop package separately.
