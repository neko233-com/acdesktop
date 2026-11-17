param(
    [Parameter(Mandatory)] [string] $Binary,
    [Parameter(Mandatory)] [string] $Point,
    [Parameter(Mandatory)] [string] $OutputRoot
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or -not [Environment]::Is64BitProcess -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
if (Test-Path -LiteralPath $OutputRoot) { throw 'Choose a new OutputRoot.' }
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DragNoticeProbe {
    public delegate bool Callback(IntPtr h, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$stdout = Join-Path $OutputRoot 'stdout.json'
$stderr = Join-Path $OutputRoot 'stderr.txt'
$foreground = [DragNoticeProbe]::GetForegroundWindow()
$oldSession = [Environment]::GetEnvironmentVariable('ACDESKTOP_SESSION', 'Process')
try {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $null, 'Process')
    $process = Start-Process -FilePath (Resolve-Path -LiteralPath $Binary).Path -ArgumentList @('drag','--from-xy',$Point,'--to-xy',$Point,'--duration','2000','--headed') -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
} finally {
    [Environment]::SetEnvironmentVariable('ACDESKTOP_SESSION', $oldSession, 'Process')
}
$found = [System.Collections.Generic.List[IntPtr]]::new()
$watch = [Diagnostics.Stopwatch]::StartNew()
$children = @()
while (-not $process.HasExited -and $watch.Elapsed.TotalSeconds -lt 10 -and $children.Count -eq 0) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $($process.Id)" |
        Where-Object { $_.CommandLine -match '(?:^|\s)--control-indicator-child(?:\s|$)' })
    if ($children.Count -eq 0) { Start-Sleep -Milliseconds 25 }
}
if ($children.Count -ne 1) { throw 'The drag process did not start exactly one control indicator.' }
$indicatorId = [uint32]$children[0].ProcessId
while (-not $process.HasExited -and $watch.Elapsed.TotalSeconds -lt 10 -and $found.Count -eq 0) {
    [DragNoticeProbe]::EnumWindows({ param($h, $unused)
        $owner = [uint32]0
        [DragNoticeProbe]::GetWindowThreadProcessId($h, [ref]$owner) | Out-Null
        if ($owner -eq $indicatorId -and [DragNoticeProbe]::IsWindowVisible($h)) { $found.Add($h) }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if ($found.Count -eq 0) { Start-Sleep -Milliseconds 25 }
}
if ($found.Count -lt 5) { throw 'The indicator did not show its screen edges and status.' }
if ([DragNoticeProbe]::GetForegroundWindow() -ne $foreground) { throw 'The indicator stole focus.' }
if (-not $process.WaitForExit(15000)) { throw 'The drag command did not finish.' }
$result = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
if ($process.ExitCode -ne 0 -or -not $result.ok) { throw "Drag failed: $($result.error | ConvertTo-Json -Compress)" }
$watch.Restart()
while ((Get-Process -Id $indicatorId -ErrorAction SilentlyContinue) -and $watch.Elapsed.TotalSeconds -lt 5) { Start-Sleep -Milliseconds 25 }
if (Get-Process -Id $indicatorId -ErrorAction SilentlyContinue) { throw 'The indicator remained after drag ended.' }
[pscustomobject]@{ ok = $true; focused_window_preserved = $true; disappeared = $true; delivery = $result.data.delivery } | ConvertTo-Json -Compress
