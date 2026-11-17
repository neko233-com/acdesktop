param(
    [Parameter(Mandatory)] [string] $Binary,
    [Parameter(Mandatory)] [string] $OutputFile,
    [switch] $Release
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess -or -not $IsWindows) {
    throw 'Run this script with 64-bit PowerShell 7 on Windows.'
}
$wix = Get-Command wix -CommandType Application -ErrorAction Stop | Select-Object -First 1
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputFile)
if ((Test-Path -LiteralPath $outputPath) -or (Test-Path -LiteralPath "$outputPath.sha256")) {
    throw 'Choose a new MSI output path so existing files are not overwritten.'
}
if (-not (Test-Path -LiteralPath (Split-Path -Parent $outputPath) -PathType Container)) {
    throw 'The MSI output directory must already exist.'
}
$versionResponse = & $binaryPath version | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or -not $versionResponse.ok -or $versionResponse.data.os -ne 'windows' -or $versionResponse.data.target -ne 'x86_64') {
    throw 'The input must be a working Windows x64 acdesktop binary.'
}
$version = [string]$versionResponse.data.version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'MSI requires a three-part numeric product version.' }
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$source = if ($Release) { 'windows-release.wxs' } else { 'windows-game-preview.wxs' }
& $wix.Source build (Join-Path $PSScriptRoot $source) -arch x64 -d "Version=$version" -d "Binary=$binaryPath" -pdbtype none -out $outputPath | Out-Host
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $outputPath -PathType Leaf)) { throw 'MSI creation failed.' }
$sha = (Get-FileHash -LiteralPath $outputPath -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$outputPath.sha256", "$sha  $([System.IO.Path]::GetFileName($outputPath))`n")
[pscustomobject]@{ msi = $outputPath; sha256 = $sha; version = $version; seconds = [math]::Round($clock.Elapsed.TotalSeconds, 2) } | ConvertTo-Json -Compress
