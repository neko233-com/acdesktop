param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [string] $Binary = (Join-Path $PSScriptRoot '../../../target/debug/acdesktop.exe')
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess) {
    throw 'Run this fixture with 64-bit PowerShell 7.'
}
if (Test-Path -LiteralPath $OutputRoot) {
    throw 'Choose a new OutputRoot so existing files are not overwritten.'
}
$log = Join-Path $env:TEMP 'acdesktop-unity-gesture-input.csv'
if (-not (Test-Path -LiteralPath $log)) {
    throw 'The Unity fixture input log does not exist; start the fixture first.'
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$checks = @()
foreach ($name in @('three-touch', 'three-finger')) {
    $before = @(Import-Csv -LiteralPath $log).Count
    $result = & $Binary game --window-id $WindowId --plan (Join-Path $PSScriptRoot "$name.json") --out (Join-Path $OutputRoot $name) --headed | ConvertFrom-Json
    if (-not $result.ok) {
        throw "The $name session failed: $($result.error | ConvertTo-Json -Compress)"
    }
    $rows = @(Import-Csv -LiteralPath $log | Select-Object -Skip $before)
    $maximum = ($rows | Measure-Object -Property count -Maximum).Maximum
    $began = @($rows | Where-Object phase -eq 'Began').Count
    $ended = @($rows | Where-Object phase -eq 'Ended').Count
    if ($maximum -lt 3 -or $began -ne 3 -or $ended -ne 3) {
        throw "Unity did not record three complete contacts for $name. Check the Windows touch gesture setting and the input log."
    }
    $starts = @($rows | Where-Object phase -eq 'Began')
    $ends = @($rows | Where-Object phase -eq 'Ended')
    $displacements = @()
    foreach ($contact in $starts) {
        $end = @($ends | Where-Object { $_.id -eq $contact.id })
        if ($end.Count -ne 1) { throw "$name did not preserve contact identity." }
        $dx = [double]$end[0].x - [double]$contact.x
        $dy = [double]$end[0].y - [double]$contact.y
        if ($name -eq 'three-finger' -and ($dx -lt 150 -or [math]::Abs($dy) -gt 5 -or @($rows | Where-Object { $_.id -eq $contact.id -and $_.phase -eq 'Moved' }).Count -lt 2)) {
            throw 'Unity did not observe all three contacts moving right.'
        }
        if ($name -eq 'three-touch' -and ([math]::Abs($dx) -gt 5 -or [math]::Abs($dy) -gt 5)) {
            throw 'Unity observed unexpected movement during the stationary hold.'
        }
        $displacements += $dx
    }
    $checks += [pscustomobject]@{ test = $name; max_contacts = $maximum; began = $began; ended = $ended; horizontal_displacements = $displacements; events = $rows.Count }
}
[pscustomobject]@{ ok = $true; checks = $checks; output = $OutputRoot } | ConvertTo-Json -Depth 4
