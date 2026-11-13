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
$process = Start-Process -FilePath (Resolve-Path -LiteralPath $Binary).Path -ArgumentList @('drag','--from-xy',$Point,'--to-xy',$Point,'--duration','2000','--headed') -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$found = [System.Collections.Generic.List[IntPtr]]::new()
$watch = [Diagnostics.Stopwatch]::StartNew()
while (-not $process.HasExited -and $watch.Elapsed.TotalSeconds -lt 10 -and $found.Count -eq 0) {
    [DragNoticeProbe]::EnumWindows({ param($h, $unused)
        $owner = [uint32]0
        [DragNoticeProbe]::GetWindowThreadProcessId($h, [ref]$owner) | Out-Null
        if ($owner -eq $process.Id -and [DragNoticeProbe]::IsWindowVisible($h)) { $found.Add($h) }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if ($found.Count -eq 0) { Start-Sleep -Milliseconds 25 }
}
if ($found.Count -ne 1) { throw 'The drag process did not display exactly one control notice.' }
if ([DragNoticeProbe]::GetForegroundWindow() -ne $foreground) { throw 'The notice stole focus.' }
if (-not $process.WaitForExit(15000)) { throw 'The drag command did not finish.' }
$result = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
if ($process.ExitCode -ne 0 -or -not $result.ok) { throw "Drag failed: $($result.error | ConvertTo-Json -Compress)" }
if ([DragNoticeProbe]::IsWindowVisible($found[0])) { throw 'The notice remained visible after drag ended.' }
[pscustomobject]@{ ok = $true; focused_window_preserved = $true; disappeared = $true; delivery = $result.data.delivery } | ConvertTo-Json -Compress
