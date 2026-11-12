const { existsSync } = require('fs');
const { join } = require('path');

const MACOS_HELPER_NAME = 'acdesktop-macos-helper';
const LINUX_HELPER_NAME = 'acdesktop-linux-clipboard-owner';

const PLATFORMS = {
  'darwin-arm64': {
    target: 'aarch64-apple-darwin',
    binaryName: 'acdesktop-darwin-arm64',
    entries: ['acdesktop', MACOS_HELPER_NAME],
    released: true,
  },
  'darwin-x64': {
    target: 'x86_64-apple-darwin',
    binaryName: 'acdesktop-darwin-x64',
    entries: ['acdesktop', MACOS_HELPER_NAME],
    released: true,
  },
  'linux-arm64': {
    target: 'aarch64-unknown-linux-gnu',
    binaryName: 'acdesktop-linux-arm64',
    entries: ['acdesktop', LINUX_HELPER_NAME],
    released: true,
  },
  'linux-x64': {
    target: 'x86_64-unknown-linux-gnu',
    binaryName: 'acdesktop-linux-x64',
    entries: ['acdesktop', LINUX_HELPER_NAME],
    released: true,
  },
  'win32-arm64': {
    target: 'aarch64-pc-windows-msvc',
    binaryName: 'acdesktop-win32-arm64.exe',
    entries: ['acdesktop.exe'],
    released: true,
  },
  'win32-x64': {
    target: 'x86_64-pc-windows-msvc',
    binaryName: 'acdesktop-win32-x64.exe',
    entries: ['acdesktop.exe'],
    released: true,
  },
};

function resolve(osPlatform, osArch) {
  return PLATFORMS[`${osPlatform}-${osArch}`];
}

// The first entry is always the CLI executable. Every other entry is a helper
// that is installed beside it under its own name.
function helperNames(entry) {
  return entry.entries.slice(1);
}

function releasedKeys() {
  return Object.entries(PLATFORMS)
    .filter(([, entry]) => entry.released)
    .map(([key]) => key);
}

function tarballName(version, target) {
  return `acdesktop-v${version}-${target}.tar.gz`;
}

// Windows has bsdtar at System32\tar.exe; GNU tar via PATH misinterprets drive-letter paths.
// Resolve in-box by absolute path, fall back to PATH if missing.
function tarCommand(osPlatform, environment) {
  if (osPlatform !== 'win32') {
    return 'tar';
  }
  const systemRoot = environment.SystemRoot || environment.windir;
  if (!systemRoot) {
    return 'tar';
  }
  const inBox = join(systemRoot, 'System32', 'tar.exe');
  return existsSync(inBox) ? inBox : 'tar';
}

module.exports = {
  LINUX_HELPER_NAME,
  MACOS_HELPER_NAME,
  PLATFORMS,
  helperNames,
  releasedKeys,
  resolve,
  tarCommand,
  tarballName,
};
