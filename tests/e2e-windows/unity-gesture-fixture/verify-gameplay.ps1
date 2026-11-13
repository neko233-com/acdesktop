param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [string] $Binary = (Join-Path $PSScriptRoot '../../../target/release/acdesktop.exe')
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess) {
    throw 'Run this fixture with 64-bit PowerShell 7.'
}
if (Test-Path -LiteralPath $OutputRoot) {
    throw 'Choose a new OutputRoot so existing files are not overwritten.'
}
$log = Join-Path $env:TEMP 'acdesktop-unity-gameplay.csv'
if (-not (Test-Path -LiteralPath $log)) {
    throw 'The Unity gameplay log does not exist; start the fixture first.'
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$checks = @()
foreach ($name in @('gameplay-negative', 'gameplay')) {
    $before = @(Import-Csv -LiteralPath $log).Count
    $result = & $Binary game --window-id $WindowId --plan (Join-Path $PSScriptRoot "$name.json") --out (Join-Path $OutputRoot $name) --headed | ConvertFrom-Json
    if (-not $result.ok) {
        throw "The $name session failed: $($result.error | ConvertTo-Json -Compress)"
    }
    $rows = @(Import-Csv -LiteralPath $log | Select-Object -Skip $before)
    if ($name -eq 'gameplay-negative') {
        if (@($rows | Where-Object event -eq 'early_fire').Count -ne 1 -or @($rows | Where-Object event -eq 'success').Count -ne 0) {
            throw 'Unity did not reject firing before joystick movement.'
        }
        $checks += [pscustomobject]@{ test = $name; early_fire = 1; successes = 0 }
        continue
    }
    $expected = @('joystick_down', 'joystick_moved', 'success', 'joystick_up') * 3
    if (($rows.event -join ',') -ne ($expected -join ',')) {
        throw "Unexpected Unity gameplay events: $($rows.event -join ',')"
    }
    $success = @($rows | Where-Object event -eq 'success')
    $scores = @($success | ForEach-Object { [int]$_.score })
    if ($scores[1] -ne $scores[0] + 1 -or $scores[2] -ne $scores[1] + 1) {
        throw 'Unity did not award a point for each joystick-and-fire action.'
    }
    $intervals = @($success | Select-Object -Skip 1 | ForEach-Object { [double]::Parse($_.interval, [cultureinfo]::InvariantCulture) })
    if (@($intervals | Where-Object { $_ -lt 0.6 -or $_ -gt 1.5 }).Count -ne 0) {
        throw "Unity recorded unexpected action intervals: $($intervals -join ', ')"
    }
    $checks += [pscustomobject]@{ test = $name; scores = $scores; intervals_seconds = $intervals; input_metrics = $result.data.summary.input_metrics }
}
[pscustomobject]@{ ok = $true; checks = $checks; log = $log; output = $OutputRoot } | ConvertTo-Json -Depth 8
