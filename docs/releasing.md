# Releasing

Update the Cargo workspace version, lockfile, and changelog together. After the change reaches `main`, the Release workflow builds and verifies the CLI and C library archives. It creates a draft, verifies the complete archive set, checksums, install scripts, and build attestations, then publishes the draft to GitHub Releases.

Before publication, the workflow checks the source commit, platform builds, FFI gates, archive contents, exact asset names, and checksums. A version with an existing published release is skipped. An existing draft is retried against its immutable tag. Investigate an incomplete existing release before retrying it; use a new version when the source has changed.
