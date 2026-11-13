param(
    [Parameter(Mandatory)] [string] $OutputDirectory
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess -or -not $IsWindows) {
    throw 'Run this script with 64-bit PowerShell 7 on Windows.'
}
$hostTriple = (rustc -vV | Select-String '^host: ').ToString().Substring(6)
if ($hostTriple -ne 'x86_64-pc-windows-msvc') {
    throw "Expected the native x64 MSVC Rust toolchain; found $hostTriple."
}
$output = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputDirectory)
if (Test-Path -LiteralPath $output) {
    throw 'Choose a new output directory so existing files are not overwritten.'
}
$repo = Split-Path -Parent $PSScriptRoot
New-Item -ItemType Directory -Path $output | Out-Null
$state = Join-Path $env:TEMP ('acdesktop-game-tests-' + [guid]::NewGuid().ToString('N'))
$oldState = $env:ACDESKTOP_HOME
$watch = [System.Diagnostics.Stopwatch]::StartNew()
Push-Location $repo
try {
    $env:ACDESKTOP_HOME = $state
    foreach ($package in @('acdesktop-core', 'acdesktop-windows')) {
        & cargo test --locked -p $package --lib game
        if ($LASTEXITCODE -ne 0) { throw "Targeted tests failed for $package." }
    }
    & cargo test --locked -p acdesktop-core --lib input::gesture::tests
    if ($LASTEXITCODE -ne 0) { throw 'Gesture tests failed.' }
    & cargo test --locked -p acdesktop-windows --lib input::touch::tests
    if ($LASTEXITCODE -ne 0) { throw 'Touch injection tests failed.' }
    & cargo test --locked -p acdesktop-windows --lib system::capture::persistent::tests
    if ($LASTEXITCODE -ne 0) { throw 'Persistent capture tests failed.' }
    & cargo test --locked -p acdesktop --bin acdesktop
    if ($LASTEXITCODE -ne 0) { throw 'CLI contract tests failed.' }
    $testsSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 2)
    & cargo build --locked --release --target $hostTriple -p acdesktop
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $metadata = & cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Could not locate the Cargo target directory.' }
    $binary = Join-Path $metadata.target_directory "$hostTriple/release/acdesktop.exe"
    $archive = Join-Path $output 'acdesktop-windows-x64-preview.tar.gz'
    & tar -czf $archive -C (Split-Path $binary) 'acdesktop.exe'
    if ($LASTEXITCODE -ne 0) { throw 'Archive creation failed.' }
    $entries = @(& tar -tzf $archive)
    if ($LASTEXITCODE -ne 0 -or $entries.Count -ne 1 -or $entries[0] -ne 'acdesktop.exe') {
        throw 'Archive contents do not match the expected binary.'
    }
    $verify = Join-Path $output ('verify-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $verify | Out-Null
    & tar -xzf $archive -C $verify
    if ($LASTEXITCODE -ne 0) { throw 'Archive extraction failed.' }
    $extracted = Join-Path $verify 'acdesktop.exe'
    if ((Get-FileHash $binary).Hash -ne (Get-FileHash $extracted).Hash) { throw 'Packaged binary differs from the build.' }
    & $extracted version
    if ($LASTEXITCODE -ne 0) { throw 'Packaged binary failed its version smoke test.' }
    Remove-Item -LiteralPath $extracted
    Remove-Item -LiteralPath $verify
    $sha = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText("$archive.sha256", "$sha  $([System.IO.Path]::GetFileName($archive))`n")
    [pscustomobject]@{ archive = $archive; sha256 = $sha; tests_seconds = $testsSeconds; total_seconds = [math]::Round($watch.Elapsed.TotalSeconds, 2) } | ConvertTo-Json -Compress
} finally {
    $env:ACDESKTOP_HOME = $oldState
    Pop-Location
}
