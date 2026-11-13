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
$log = Join-Path $env:TEMP 'acdesktop-unity-gesture-input.csv'
if (-not (Test-Path -LiteralPath $log)) {
    throw 'The Unity touch log does not exist; start the fixture first.'
}
$before = @(Import-Csv -LiteralPath $log).Count
$result = & $Binary game --window-id $WindowId --plan (Join-Path $PSScriptRoot 'plan.json') --out $OutputRoot --headed | ConvertFrom-Json
if (-not $result.ok) {
    throw "The gesture session failed: $($result.error | ConvertTo-Json -Compress)"
}
$rows = @(Import-Csv -LiteralPath $log | Select-Object -Skip $before)
$begins = @($rows | Where-Object phase -eq 'Began')
if ($begins.Count -ne 6 -or @($rows | Where-Object phase -eq 'Ended').Count -ne 6) {
    throw 'Unity did not record all six complete contacts.'
}
function Get-ContactPath($begin) {
    $id = $begin.id
    $start = [double]$begin.time
    $end = @($rows | Where-Object { $_.id -eq $id -and $_.phase -eq 'Ended' -and [double]$_.time -ge $start } | Select-Object -First 1)
    if ($end.Count -ne 1) { throw "Contact $id has no end." }
    return @($rows | Where-Object { $_.id -eq $id -and [double]$_.time -ge $start -and [double]$_.time -le [double]$end[0].time -and $_.phase -ne 'Stationary' })
}
$s = @(Get-ContactPath $begins[0])
$circle = @(Get-ContactPath $begins[1])
if ($s.Count -lt 8 -or $circle.Count -lt 8) { throw 'Unity recorded too few path samples.' }
$sMin = ($s | Measure-Object -Property x -Minimum).Minimum
$sMax = ($s | Measure-Object -Property x -Maximum).Maximum
$leftIndex = [array]::IndexOf($s, @($s | Where-Object { [double]$_.x -eq $sMin })[0])
$rightIndex = [array]::IndexOf($s, @($s | Where-Object { [double]$_.x -eq $sMax })[0])
if ($sMin -gt 300 -or $sMax -lt 400 -or $leftIndex -ge $rightIndex -or ([double]$s[0].y - [double]$s[-1].y) -lt 100) {
    throw 'Unity did not receive the expected S trajectory.'
}
$area = 0.0
for ($i = 1; $i -lt $circle.Count; $i++) {
    $area += [double]$circle[$i-1].x * [double]$circle[$i].y - [double]$circle[$i].x * [double]$circle[$i-1].y
}
$circleWidth = ($circle | Measure-Object -Property x -Maximum).Maximum - ($circle | Measure-Object -Property x -Minimum).Minimum
$circleHeight = ($circle | Measure-Object -Property y -Maximum).Maximum - ($circle | Measure-Object -Property y -Minimum).Minimum
if ($circleWidth -lt 100 -or $circleHeight -lt 100 -or $area -ge 0 -or [math]::Abs([double]$circle[0].x - [double]$circle[-1].x) -gt 15 -or [math]::Abs([double]$circle[0].y - [double]$circle[-1].y) -gt 15) {
    throw 'Unity did not receive a closed clockwise circle.'
}
$joystick = @(Get-ContactPath $begins[4])
if ([int]$begins[2].count -lt 2 -or [int]$begins[3].count -lt 2 -or @($joystick | Where-Object { [int]$_.count -ge 2 }).Count -eq 0 -or ([double]$joystick[-1].x - [double]$joystick[0].x) -lt 40) {
    throw 'Unity did not receive the two-contact joystick drag.'
}
[pscustomobject]@{ ok = $true; s_samples = $s.Count; circle_samples = $circle.Count; clockwise_circle_area = -$area / 2; contacts = $begins.Count; joystick_drag = [double]$joystick[-1].x - [double]$joystick[0].x; output = $OutputRoot } | ConvertTo-Json -Compress
