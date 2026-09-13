# ADR 014 - Release Engineering

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §143; Milestone 20

## Decision

Use locked release builds for six first-class targets, produce checksums and
an SBOM for every artifact, and sign daemon archives with Cosign keyless
signing. Release notes are generated from commit history and must explicitly
document public interface changes.

`scripts/release.sh` performs the local subset: Bun web build, locked daemon
build, archive, checksum, and dependency inventory.

## Consequences

Installer signatures and clean-environment installation tests remain follow-up
work before claiming the v1.0 installation guarantee. The release workflow
provides reproducible artifact boundaries and signing now.
