# Releasing

Update the Cargo workspace version, lockfile, and changelog together. After the change reaches `main`, the Release workflow builds and verifies the CLI and C library archives. It creates a draft, verifies the complete archive set, checksums, install scripts, and build attestations, then publishes the draft to GitHub Releases.

Before publication, the workflow checks the source commit, platform builds, FFI gates, archive contents, exact asset names, and checksums. A version with an existing published release is skipped. An existing draft is retried against its immutable tag. Investigate an incomplete existing release before retrying it; use a new version when the source has changed.

The Windows x64 release includes a per-user MSI alongside the portable archive.
It installs to `%LOCALAPPDATA%\Programs\acdesktop` by default, adds that
directory to the user `PATH`, and removes its own entry on uninstall. The
release job verifies installation, the installed binary, upgrades, and uninstallation;
Windows ARM64 continues to ship the portable archive. The repository does not
provide a code-signing certificate, so the MSI is unsigned.
