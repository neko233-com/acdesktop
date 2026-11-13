param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [string] $Binary = (Join-Path $PSScriptRoot '../../../target/x86_64-pc-windows-msvc/release/acdesktop.exe')
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess) {
    throw 'Run this fixture with 64-bit PowerShell 7.'
}
if (Test-Path -LiteralPath $OutputRoot) { throw 'Choose a new OutputRoot.' }
$log = Join-Path $env:TEMP 'acdesktop-unity-gesture-input.csv'
if (-not (Test-Path -LiteralPath $log)) { throw 'Start the Unity fixture first.' }
$before = @(Import-Csv -LiteralPath $log).Count
$result = & $Binary game --window-id $WindowId --plan (Join-Path $PSScriptRoot 'pinch-rotate.json') --out $OutputRoot --headed | ConvertFrom-Json
if (-not $result.ok) { throw "Game session failed: $($result.error | ConvertTo-Json -Depth 6 -Compress)" }
$rows = @(Import-Csv -LiteralPath $log | Select-Object -Skip $before)
$begins = @($rows | Where-Object phase -eq 'Began')
if ($begins.Count -ne 8 -or @($rows | Where-Object phase -eq 'Ended').Count -ne 8) {
    throw 'Unity did not record four complete two-finger gestures.'
}
$checks = @()
for ($i = 0; $i -lt 4; $i++) {
    $start = [double]$begins[2 * $i].time
    $next = if ($i -lt 3) { [double]$begins[2 * $i + 2].time } else { [double]::PositiveInfinity }
    $segment = @($rows | Where-Object { [double]$_.time -ge $start -and [double]$_.time -lt $next })
    $down = @($segment | Where-Object phase -eq 'Began' | Sort-Object { [double]$_.x })
    $up = @($segment | Where-Object phase -eq 'Ended')
    if ($down.Count -ne 2 -or $up.Count -ne 2 -or ($segment | Measure-Object -Property count -Maximum).Maximum -lt 2) {
        throw "Gesture $i did not contain two complete simultaneous contacts."
    }
    $left = @($up | Where-Object id -eq $down[0].id)
    $right = @($up | Where-Object id -eq $down[1].id)
    if ($left.Count -ne 1 -or $right.Count -ne 1) { throw "Gesture $i lost a contact identity." }
    foreach ($contact in $down) {
        if (@($segment | Where-Object { $_.id -eq $contact.id -and $_.phase -eq 'Moved' }).Count -lt 2) {
            throw "Gesture $i did not move both contacts."
        }
    }
    if ($i -lt 2) {
        $delta = ([double]$right[0].x - [double]$left[0].x) - ([double]$down[1].x - [double]$down[0].x)
        if (($i -eq 0 -and $delta -lt 60) -or ($i -eq 1 -and $delta -gt -60)) {
            throw "Gesture $i has the wrong pinch direction or distance."
        }
        $checks += [pscustomobject]@{ gesture = @('pinch-out', 'pinch-in')[$i]; span_delta = $delta }
    } else {
        $delta = [double]$right[0].y - [double]$down[1].y
        if (($i -eq 2 -and $delta -gt -30) -or ($i -eq 3 -and $delta -lt 30)) {
            throw "Gesture $i has the wrong rotation direction or distance."
        }
        $checks += [pscustomobject]@{ gesture = @('rotate-cw', 'rotate-ccw')[$i - 2]; unity_y_delta = $delta }
    }
}
[pscustomobject]@{ ok = $true; checks = $checks; output = $OutputRoot } | ConvertTo-Json -Depth 5 -Compress
