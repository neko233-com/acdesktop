param(
    [string] $PackageScript = (Join-Path $PSScriptRoot '../package-windows-game-preview.ps1')
)

$ErrorActionPreference = 'Stop'
$scratch = Join-Path $env:TEMP ('acdesktop-package-selftest-' + [guid]::NewGuid().ToString('N'))
$right = Join-Path $scratch 'powershell-location'
$wrong = Join-Path $scratch 'process-location'
New-Item -ItemType Directory -Path $right, $wrong | Out-Null
$script = (Resolve-Path -LiteralPath $PackageScript).Path
$originalProcessDirectory = [Environment]::CurrentDirectory
Push-Location $right
try {
    [Environment]::CurrentDirectory = $wrong
    $output = & $script -OutputDirectory 'preview'
    $testOutput = $output -join "`n"
    foreach ($testName in @(
        'input::gesture::tests::every_gesture_has_ordered_complete_contact_lifecycles',
        'input::touch::tests::a_touch_frame_reaches_the_recording_sink',
        'system::capture::persistent::tests::an_empty_frame_pool_is_distinct_from_a_failed_pool'
    )) {
        if ($testOutput -notmatch [regex]::Escape("test $testName ... ok")) {
            throw "Fast packaging did not run the required test: $testName"
        }
    }
    $archive = Join-Path $right 'preview/acdesktop-windows-x64-preview.tar.gz'
    if (-not (Test-Path -LiteralPath $archive) -or (Test-Path -LiteralPath (Join-Path $wrong 'preview'))) {
        throw 'Relative output did not resolve against the PowerShell location.'
    }
    $checksum = Get-Content -LiteralPath "$archive.sha256" -Raw
    if (-not $checksum.StartsWith((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant())) {
        throw 'Archive checksum does not match.'
    }
    $marker = Join-Path $right 'preview/sentinel'
    Set-Content -LiteralPath $marker -Value 'preserved'
    try {
        & $script -OutputDirectory 'preview'
        throw 'Existing output directory was accepted.'
    } catch {
        if ($_.Exception.Message -notlike 'Choose a new output directory*') { throw }
    }
    if ((Get-Content -LiteralPath $marker) -ne 'preserved') { throw 'Existing output was changed.' }
    [pscustomobject]@{ ok = $true; relative_output = $archive; existing_output_preserved = $true } | ConvertTo-Json -Compress
} finally {
    [Environment]::CurrentDirectory = $originalProcessDirectory
    Pop-Location
}
