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
