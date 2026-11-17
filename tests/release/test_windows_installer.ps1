$ErrorActionPreference = 'Stop'
$installer = (Resolve-Path (Join-Path $PSScriptRoot '..\..\scripts\install.ps1')).ProviderPath
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$root = Join-Path $tempRoot ('acdesktop-installer-test-' + [Guid]::NewGuid().ToString('N'))
$oldArch = $env:PROCESSOR_ARCHITECTURE
$oldNative = $env:PROCESSOR_ARCHITEW6432
$oldInstall = $env:ACDESKTOP_INSTALL_DIR
$oldNoPath = $env:ACDESKTOP_INSTALL_NO_PATH

function global:curl.exe {
  $global:LASTEXITCODE = 0
  if ($args -contains '-w') {
    if ($global:InstallFixtureFailRedirect) { $global:LASTEXITCODE = 22; return }
    return 'https://github.com/neko233-com/acdesktop/releases/tag/v1.2.3'
  }
  $index = [Array]::IndexOf($args, '-o')
  if ($index -lt 0 -and [string]$args[$args.Count - 1] -like '*/releases/latest') {
    return '{"tag_name":"v1.2.3"}'
  }
  if ($index -lt 0) { throw 'fixture expected curl -o' }
  $destination = [string]$args[$index + 1]
  $url = [string]$args[$args.Count - 1]
  if ($url.EndsWith('/checksums.txt')) {
    $hash = (Get-FileHash -LiteralPath $global:InstallFixtureArchive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($global:InstallFixtureBadHash) { $hash = '0' * 64 }
    Set-Content -LiteralPath $destination -Encoding Ascii -Value "$hash  $global:InstallFixtureName"
  } else {
    $global:InstallFixtureName = [IO.Path]::GetFileName($url)
    Copy-Item -LiteralPath $global:InstallFixtureArchive -Destination $destination
  }
}

function Assert-True([bool]$condition, [string]$message) {
  if (-not $condition) { throw $message }
}

try {
  New-Item -ItemType Directory -Path $root | Out-Null
  $stage = Join-Path $root 'stage'
  New-Item -ItemType Directory -Path $stage | Out-Null
  Set-Content -LiteralPath (Join-Path $stage 'acdesktop.exe') -Encoding Ascii -Value 'fixture executable'
  $global:InstallFixtureArchive = Join-Path $root 'fixture.tar.gz'
  & tar.exe -czf $global:InstallFixtureArchive -C $stage acdesktop.exe
  if ($LASTEXITCODE -ne 0) { throw 'Could not create the installer fixture' }
  $env:ACDESKTOP_INSTALL_NO_PATH = '1'
  $env:ACDESKTOP_INSTALL_DIR = ''
  $oldUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $oldProcessPath = $env:Path
  $env:PROCESSOR_ARCHITECTURE = 'AMD64'
  $env:PROCESSOR_ARCHITEW6432 = ''
  $x64 = Join-Path $root 'x64'
  & $installer -InstallDir $x64
  Assert-True (Test-Path -LiteralPath (Join-Path $x64 'acdesktop.exe')) 'x64 install failed'
  Assert-True ($global:InstallFixtureName -eq 'acdesktop-v1.2.3-x86_64-pc-windows-msvc.tar.gz') 'wrong x64 asset'
  Assert-True ([Environment]::GetEnvironmentVariable('Path', 'User') -eq $oldUserPath) 'NoPath changed the user PATH'
  Assert-True ($env:Path -eq $oldProcessPath) 'NoPath changed the process PATH'

  $env:PROCESSOR_ARCHITECTURE = 'x86'
  & $installer -Version 1.2.3 -InstallDir (Join-Path $root 'x86')
  Assert-True ($global:InstallFixtureName -eq 'acdesktop-v1.2.3-i686-pc-windows-msvc.tar.gz') 'wrong x86 asset'
  $env:PROCESSOR_ARCHITECTURE = 'AMD64'

  $global:InstallFixtureFailRedirect = $true
  & $installer -InstallDir (Join-Path $root 'api-fallback')
  Assert-True ($global:InstallFixtureName -eq 'acdesktop-v1.2.3-x86_64-pc-windows-msvc.tar.gz') 'API fallback selected the wrong asset'
  $global:InstallFixtureFailRedirect = $false

  $env:PROCESSOR_ARCHITECTURE = 'x86'
  $env:PROCESSOR_ARCHITEW6432 = 'ARM64'
  $arm = Join-Path $root 'arm64'
  & $installer -Version 1.2.3 -InstallDir $arm
  Assert-True ($global:InstallFixtureName -eq 'acdesktop-v1.2.3-aarch64-pc-windows-msvc.tar.gz') 'wrong native ARM64 asset'
  Assert-True (Test-Path -LiteralPath (Join-Path $arm 'acdesktop.exe')) 'ARM64 install failed'

  $failed = $false
  try { & $installer -Version '../bad' -InstallDir (Join-Path $root 'invalid') } catch { $failed = $true }
  Assert-True $failed 'invalid versions must fail'

  $global:InstallFixtureBadHash = $true
  $existing = Join-Path $root 'existing'
  New-Item -ItemType Directory -Path $existing | Out-Null
  Set-Content -LiteralPath (Join-Path $existing 'acdesktop.exe') -Encoding Ascii -Value 'existing executable'
  $failed = $false
  try { & $installer -Version 1.2.3 -InstallDir $existing } catch { $failed = $true }
  Assert-True $failed 'bad checksum must fail'
  Assert-True ((Get-Content -Raw -LiteralPath (Join-Path $existing 'acdesktop.exe')).Trim() -eq 'existing executable') 'bad checksum replaced the executable'

  $global:InstallFixtureBadHash = $false
  Set-Content -LiteralPath (Join-Path $stage 'extra.txt') -Encoding Ascii -Value 'unexpected entry'
  & tar.exe -czf $global:InstallFixtureArchive -C $stage acdesktop.exe extra.txt
  if ($LASTEXITCODE -ne 0) { throw 'Could not create the unsafe archive fixture' }
  $failed = $false
  try { & $installer -Version 1.2.3 -InstallDir $existing } catch { $failed = $true }
  Assert-True $failed 'extra archive entries must fail'
  Assert-True ((Get-Content -Raw -LiteralPath (Join-Path $existing 'acdesktop.exe')).Trim() -eq 'existing executable') 'unsafe archive replaced the executable'
  Write-Output 'Windows installer tests passed'
} finally {
  $env:PROCESSOR_ARCHITECTURE = $oldArch
  $env:PROCESSOR_ARCHITEW6432 = $oldNative
  $env:ACDESKTOP_INSTALL_DIR = $oldInstall
  $env:ACDESKTOP_INSTALL_NO_PATH = $oldNoPath
  Remove-Item Function:\curl.exe -ErrorAction SilentlyContinue
  if (Test-Path -LiteralPath $root) {
    $resolved = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $root).ProviderPath)
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        $resolved -ieq [IO.Path]::GetFullPath($root)) {
      Remove-Item -LiteralPath $resolved -Recurse -Force
    }
  }
}
