# Installing acdesktop

Install the native CLI from a [release archive](https://github.com/neko233-com/acdesktop/releases), or build it from source. The install scripts support macOS ARM64 and Windows ARM64:

```bash
curl -fsSL https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.sh | bash
```

```powershell
irm https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.ps1 | iex
```

## Release archives

Download the archive for your OS and architecture from [GitHub Releases](https://github.com/neko233-com/acdesktop/releases). Extract the executable and its platform helper into one directory on your PATH. To upgrade, replace both files with the ones from the same release.

The release's `checksums.txt` catches corrupted downloads but does not prove where the archive came from. Verify its origin with the build attestation:

```bash
gh attestation verify <downloaded-archive> --repo neko233-com/acdesktop
```

## From source

You need the Rust toolchain pinned in `rust-toolchain.toml` (1.89.0). `rustup` installs it on first use. You also need a C toolchain: the Xcode command-line tools on macOS 13 or later, the MSVC build tools on Windows, or `build-essential` on Linux.

```bash
git clone https://github.com/neko233-com/acdesktop && cd acdesktop
cargo build --release -p acdesktop
```

Each platform crate builds only on its own OS. Build the helper for your platform. A source build puts it in `target/release/`, next to the CLI. Windows has no helper.

```bash
cargo build --release -p acdesktop-macos --bin acdesktop-macos-helper   # macOS
cargo build --release -p acdesktop-linux --bins                             # Linux
```

The CLI looks for the helper in its own directory, so move the two together.

## Permissions

**macOS** needs Accessibility permission. Screenshots also need Screen Recording. Some commands need Automation permission for System Events, and report it when it is missing. `acdesktop permissions` reports the state without prompting. `permissions --request` shows the system prompts. `permissions --guide` opens System Settings with an on-screen guide for Accessibility and Screen Recording.

**Windows** needs no grant for apps at your terminal's integrity level. An app running as administrator can be read, but only an elevated agent can send it input. The binary is unsigned. SmartScreen may warn on a browser download, and antivirus may quarantine it. On a clean Windows 11 install, Smart App Control blocks it, and no download method or launch mode avoids that. Turn Smart App Control off to run it. To restore a quarantined file, first check its SHA-256 checksum against the release.

**Linux** needs Ubuntu 24.04 or later on x86_64 or aarch64, with a glibc 2.35 floor. Ubuntu 22.04's AT-SPI bridge aborts on `Action.GetActions`. Run the CLI inside the user's graphical session, with `at-spi2-core` installed. A TTY, an SSH shell, or a container without that session's bus, display, and runtime environment sees no apps. `acdesktop status` activates and checks the accessibility bus. An empty `supported_surfaces` list means the bus did not answer.

Keys need XTEST on X11 and GNOME Mutter RemoteDesktop on Wayland. The Wayland clipboard needs `wl-clipboard`. For the rest, run `acdesktop skills get platform` on the Linux machine.

## Platform notes

**Windows.** `list-surfaces` lists each process's `window`, `focused`, `sheet`, and `menu` surfaces. `snapshot --surface` also reads `taskbar`, `system-tray`, `system-tray-overflow`, `start-menu`, and `action-center` without `--app`. `quick-settings` returns `PLATFORM_NOT_SUPPORTED` and names `action-center` as the surface that carries those controls.

`launch` takes an absolute path, or a bare name under System32 or the Windows directory, with or without `.exe`. It does not resolve a display name such as "Google Chrome". Use `list-apps` to find a running app.

**Linux.** Semantic actions work on X11 and Wayland. On Wayland, app pointer input returns `ACTION_NOT_SUPPORTED`, and window control is refused for native windows.
