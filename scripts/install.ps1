# Installs acdesktop on Windows x86, x64 and ARM64 with PowerShell 5.1 or 7.
# irm https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.ps1 | iex
# powershell.exe -File scripts/install.ps1 -Version v1.4.0 -InstallDir C:\Tools\acdesktop

[CmdletBinding()]
param(
  [string]$Version = '',
  [string]$InstallDir = '',
  [switch]$NoPath
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$Repo = 'neko233-com/acdesktop'
$GhReleases = "https://github.com/$Repo/releases"

$NativeArchitecture = if ($env:PROCESSOR_ARCHITEW6432) {
  $env:PROCESSOR_ARCHITEW6432
} else {
  $env:PROCESSOR_ARCHITECTURE
}
if (-not $NativeArchitecture) { throw 'Windows processor architecture is unavailable' }
switch ($NativeArchitecture.ToUpperInvariant()) {
  'X86' { $Target = 'i686-pc-windows-msvc' }
  'AMD64' { $Target = 'x86_64-pc-windows-msvc' }
  'ARM64' { $Target = 'aarch64-pc-windows-msvc' }
  default { throw "Unsupported Windows architecture '$NativeArchitecture'. See $GhReleases" }
}

if (-not $InstallDir -and $env:ACDESKTOP_INSTALL_DIR) { $InstallDir = $env:ACDESKTOP_INSTALL_DIR }
if (-not $InstallDir) { $InstallDir = Join-Path $HOME '.local\bin' }
$SkipPath = $NoPath -or ($env:ACDESKTOP_INSTALL_NO_PATH -eq '1')
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
if ($Version) {
  $Tag = if ($Version -match '^v') { $Version } else { "v$Version" }
} else {
  $releaseUrl = & curl.exe -fsSLI --connect-timeout 15 --max-time 60 `
    -o NUL -w '%{url_effective}' "$GhReleases/latest"
  $releasePattern = '^https://github\.com/' + [regex]::Escape($Repo) + '/releases/tag/(v[^/?#]+)$'
  if ($LASTEXITCODE -eq 0 -and $releaseUrl -match $releasePattern) {
    $Tag = $Matches[1]
  } else {
    $response = & curl.exe -fsSL --connect-timeout 15 --max-time 60 `
      -H 'Accept: application/vnd.github+json' -H 'User-Agent: acdesktop-installer' `
      "https://api.github.com/repos/$Repo/releases/latest"
    if ($LASTEXITCODE -ne 0) { throw "Could not query the latest release: $LASTEXITCODE" }
    $latest = $response | ConvertFrom-Json
    $Tag = [string]$latest.tag_name
  }
}
if ($Tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
  throw "Invalid release version '$Tag'"
}

$Ver = $Tag.Substring(1)
$Tarball = "acdesktop-v$Ver-$Target.tar.gz"
$BaseUrl = "$GhReleases/download/$Tag"
$TempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$Staging = Join-Path $TempRoot ("acdesktop-install-" + [Guid]::NewGuid().ToString('N'))
$TmpExe = $null
try {
  New-Item -ItemType Directory -Path $Staging | Out-Null
  $Archive = Join-Path $Staging $Tarball
  $Checksums = Join-Path $Staging 'checksums.txt'
  Write-Host "Downloading $Tarball"
  & curl.exe -fsSL --retry 3 --retry-delay 1 -o $Archive "$BaseUrl/$Tarball"
  if ($LASTEXITCODE -ne 0) { throw "Archive download failed: $LASTEXITCODE" }
  & curl.exe -fsSL --retry 3 --retry-delay 1 -o $Checksums "$BaseUrl/checksums.txt"
  if ($LASTEXITCODE -ne 0) { throw "Checksum download failed: $LASTEXITCODE" }

  $pattern = '^([a-fA-F0-9]{64})\s+\*?' + [regex]::Escape($Tarball) + '$'
  $matchingLines = @(Get-Content -LiteralPath $Checksums | Where-Object { $_ -match $pattern })
  if ($matchingLines.Count -ne 1 -or $matchingLines[0] -notmatch $pattern) {
    throw "checksums.txt must contain exactly one SHA256 for $Tarball"
  }
  $expected = $Matches[1].ToLowerInvariant()
  $actual = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $expected) { throw "SHA256 mismatch for $Tarball" }

  $entries = @(& tar.exe -tzf $Archive)
  if ($LASTEXITCODE -ne 0 -or $entries.Count -ne 1 -or $entries[0] -cne 'acdesktop.exe') {
    throw 'The release archive must contain only acdesktop.exe'
  }
  & tar.exe -xzf $Archive -C $Staging
  if ($LASTEXITCODE -ne 0) { throw "Archive extraction failed: $LASTEXITCODE" }
  $SrcExe = Join-Path $Staging 'acdesktop.exe'
  if (-not (Test-Path -LiteralPath $SrcExe -PathType Leaf)) { throw 'acdesktop.exe is missing from the archive' }

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  $DstExe = Join-Path $InstallDir 'acdesktop.exe'
  $TmpExe = Join-Path $InstallDir ('acdesktop.exe.install.' + [Guid]::NewGuid().ToString('N'))
  Copy-Item -LiteralPath $SrcExe -Destination $TmpExe
  Move-Item -LiteralPath $TmpExe -Destination $DstExe -Force
} finally {
  if ($TmpExe) { Remove-Item -LiteralPath $TmpExe -Force -ErrorAction SilentlyContinue }
  if (Test-Path -LiteralPath $Staging) {
    $ResolvedStaging = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Staging).ProviderPath)
    $StagingItem = Get-Item -LiteralPath $Staging -Force
    if ($ResolvedStaging.StartsWith($TempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        $ResolvedStaging -ieq [IO.Path]::GetFullPath($Staging) -and
        -not ($StagingItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
      Remove-Item -LiteralPath $ResolvedStaging -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
}

if (-not $SkipPath) {
  $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $normalized = $InstallDir.TrimEnd('\')
  $userPaths = @($UserPath -split ';' | Where-Object { $_ })
  if (-not @($userPaths | Where-Object { $_.TrimEnd('\') -ieq $normalized }).Count) {
    $newPath = (@($userPaths) + @($InstallDir)) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
  }
  $processPaths = @($env:Path -split ';' | Where-Object { $_ })
  if (-not @($processPaths | Where-Object { $_.TrimEnd('\') -ieq $normalized }).Count) {
    $env:Path = "$InstallDir;$env:Path"
  }
}
Write-Host "Installed acdesktop $Tag to $DstExe"
if ($SkipPath) { Write-Host 'PATH was not changed.' }
Write-Host 'Run acdesktop version to verify the installation.'
