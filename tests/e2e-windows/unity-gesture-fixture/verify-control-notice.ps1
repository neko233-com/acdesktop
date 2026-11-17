param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [Parameter(Mandatory)] [string] $Binary
)

Import-Module (Join-Path $PSScriptRoot '../Harness.psm1') -Force
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or -not [Environment]::Is64BitProcess -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
if (Test-Path -LiteralPath $OutputRoot) { throw 'Choose a new OutputRoot.' }
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class GameIndicatorProbe {
    public delegate bool Callback(IntPtr h, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int index);
}
'@
function IndicatorChildren([int] $ParentId) {
    @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $ParentId" |
        Where-Object { $_.CommandLine -match '(?:^|\s)--control-indicator-child(?:\s|$)' })
}
function VisibleWindows([uint32] $ProcessId) {
    $windows = [System.Collections.Generic.List[IntPtr]]::new()
    [GameIndicatorProbe]::EnumWindows({ param($h, $unused)
        $owner = [uint32]0
        [GameIndicatorProbe]::GetWindowThreadProcessId($h, [ref]$owner) | Out-Null
        if ($owner -eq $ProcessId -and [GameIndicatorProbe]::IsWindowVisible($h)) { $windows.Add($h) }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    return $windows.ToArray()
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$plan = (Resolve-Path (Join-Path $PSScriptRoot 'gameplay.json')).Path
$out = Join-Path $OutputRoot 'stdout.json'
$err = Join-Path $OutputRoot 'stderr.txt'
$oldSession = [Environment]::GetEnvironmentVariable('ACDESKTOP_SESSION', 'Process')
try {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $null, 'Process')
    $process = Start-Process -FilePath $binaryPath -ArgumentList @('game','--window-id',$WindowId,'--plan',"`"$plan`"",'--out',"`"$(Join-Path $OutputRoot 'session')`"",'--headed') -WindowStyle Hidden -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
} finally {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $oldSession, 'Process')
}
$watch = [Diagnostics.Stopwatch]::StartNew()
$children = @()
while ($watch.Elapsed.TotalSeconds -lt 10 -and -not $process.HasExited -and $children.Count -eq 0) {
    $children = @(IndicatorChildren $process.Id)
    if ($children.Count -eq 0) { Start-Sleep -Milliseconds 25 }
}
if ($children.Count -ne 1) { throw 'The game process did not start exactly one control indicator.' }
$indicatorId = [uint32]$children[0].ProcessId
$windows = @(VisibleWindows $indicatorId)
if ($windows.Count -lt 5) { throw 'The indicator did not show its screen edges and status.' }
foreach ($window in $windows) {
    $style = [GameIndicatorProbe]::GetWindowLongPtrW($window, -20)
    if (($style -band 0x20) -eq 0 -or ($style -band 0x08000000) -eq 0) { throw 'An indicator window is not click-through and non-activating.' }
    if ([GameIndicatorProbe]::GetForegroundWindow() -eq $window) { throw 'An indicator window stole foreground focus.' }
}
if (-not $process.WaitForExit(15000)) { throw 'The game command did not finish.' }
$result = Get-Content -LiteralPath $out -Raw | ConvertFrom-AgentJson
if ($process.ExitCode -ne 0 -or -not $result['ok']) { throw "Game command failed: $($result['error'] | ConvertTo-Json -Compress)" }
$watch.Restart()
while ((Get-Process -Id $indicatorId -ErrorAction SilentlyContinue) -and $watch.Elapsed.TotalSeconds -lt 5) { Start-Sleep -Milliseconds 25 }
if (Get-Process -Id $indicatorId -ErrorAction SilentlyContinue) { throw 'The control indicator remained after the command ended.' }
$observePlan = Join-Path $OutputRoot 'observation-only.json'
[System.IO.File]::WriteAllText($observePlan, '{"duration_ms":2000,"fps":10,"events":[]}')
$observeOut = Join-Path $OutputRoot 'observation.json'
try {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $null, 'Process')
    $observe = Start-Process -FilePath $binaryPath -ArgumentList @('game','--window-id',$WindowId,'--plan',"`"$observePlan`"",'--out',"`"$(Join-Path $OutputRoot 'observation')`"") -WindowStyle Hidden -PassThru -RedirectStandardOutput $observeOut
} finally {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $oldSession, 'Process')
}
$watch.Restart()
while (-not $observe.HasExited -and $watch.Elapsed.TotalSeconds -lt 10) {
    if (@(IndicatorChildren $observe.Id).Count -gt 0) { throw 'An observation-only session displayed a control indicator.' }
    Start-Sleep -Milliseconds 25
}
if (-not $observe.HasExited) { throw 'The observation-only command did not finish.' }
$observation = Get-Content -LiteralPath $observeOut -Raw | ConvertFrom-AgentJson
if ($observe.ExitCode -ne 0 -or -not $observation['ok']) { throw 'The observation-only session failed.' }
[pscustomobject]@{ ok = $true; indicator_windows = $windows.Count; click_through = $true; disappeared = $true; observation_has_no_indicator = $true } | ConvertTo-Json -Compress
