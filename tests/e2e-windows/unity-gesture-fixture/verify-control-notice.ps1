param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [Parameter(Mandatory)] [string] $Binary
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or -not [Environment]::Is64BitProcess -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
if (Test-Path -LiteralPath $OutputRoot) { throw 'Choose a new OutputRoot.' }
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ControlNoticeProbe {
    public delegate bool Callback(IntPtr h, IntPtr data);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect rect);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int index);
}
'@
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$out = Join-Path $OutputRoot 'stdout.json'
$err = Join-Path $OutputRoot 'stderr.txt'
$plan = (Resolve-Path (Join-Path $PSScriptRoot 'gameplay.json')).Path
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$foreground = [ControlNoticeProbe]::GetForegroundWindow()
$process = Start-Process -FilePath $binaryPath -ArgumentList @('game','--window-id',$WindowId,'--plan',"`"$plan`"",'--out',"`"$(Join-Path $OutputRoot 'session')`"",'--headed') -WindowStyle Hidden -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
$watch = [Diagnostics.Stopwatch]::StartNew()
$found = [System.Collections.Generic.List[IntPtr]]::new()
while ($watch.Elapsed.TotalSeconds -lt 10 -and -not $process.HasExited -and $found.Count -eq 0) {
    [ControlNoticeProbe]::EnumWindows({ param($h, $unused)
        $owner = [uint32]0
        [ControlNoticeProbe]::GetWindowThreadProcessId($h, [ref]$owner) | Out-Null
        if ($owner -eq $process.Id -and [ControlNoticeProbe]::IsWindowVisible($h)) { $found.Add($h) }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if ($found.Count -eq 0) { Start-Sleep -Milliseconds 25 }
}
if ($found.Count -ne 1) { throw 'The game process did not display exactly one control notice.' }
$notice = $found[0]
$rect = [ControlNoticeProbe+Rect]::new()
if (-not [ControlNoticeProbe]::GetWindowRect($notice, [ref]$rect)) { throw 'Could not read the notice bounds.' }
$style = [ControlNoticeProbe]::GetWindowLongPtrW($notice, -20)
if (($style -band 0x20) -eq 0 -or ($style -band 0x08000000) -eq 0) { throw 'The notice is not click-through and non-activating.' }
if ([ControlNoticeProbe]::GetForegroundWindow() -ne $foreground) { throw 'The notice stole foreground focus.' }
$image = [System.Drawing.Bitmap]::new($rect.Right - $rect.Left, $rect.Bottom - $rect.Top)
$graphics = [System.Drawing.Graphics]::FromImage($image)
try {
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $image.Size)
    $light = 0
    for ($x = 0; $x -lt $image.Width; $x++) {
        for ($y = 0; $y -lt $image.Height; $y++) {
            $pixel = $image.GetPixel($x, $y)
            if ($pixel.R -gt 160 -and $pixel.G -gt 160 -and $pixel.B -gt 160) { $light++ }
        }
    }
    if ($light -lt 20) { throw 'The displayed notice contains no readable light text pixels.' }
    $image.Save((Join-Path $OutputRoot 'notice.png'))
} finally {
    $graphics.Dispose()
    $image.Dispose()
}
if (-not $process.WaitForExit(15000)) { throw 'The game command did not finish.' }
$result = Get-Content -LiteralPath $out -Raw | ConvertFrom-Json
if ($process.ExitCode -ne 0 -or -not $result.ok) { throw "Game command failed: $($result.error | ConvertTo-Json -Compress)" }
if ([ControlNoticeProbe]::IsWindowVisible($notice)) { throw 'The notice remained visible after control ended.' }
$observePlan = Join-Path $OutputRoot 'observation-only.json'
[System.IO.File]::WriteAllText($observePlan, '{"duration_ms":2000,"fps":10,"events":[]}')
$observeOut = Join-Path $OutputRoot 'observation.json'
$observe = Start-Process -FilePath $binaryPath -ArgumentList @('game','--window-id',$WindowId,'--plan',"`"$observePlan`"",'--out',"`"$(Join-Path $OutputRoot 'observation')`"") -WindowStyle Hidden -PassThru -RedirectStandardOutput $observeOut
$watch.Restart()
while (-not $observe.HasExited -and $watch.Elapsed.TotalSeconds -lt 10) {
    [ControlNoticeProbe]::EnumWindows({ param($h, $unused)
        $owner = [uint32]0
        [ControlNoticeProbe]::GetWindowThreadProcessId($h, [ref]$owner) | Out-Null
        if ($owner -eq $observe.Id -and [ControlNoticeProbe]::IsWindowVisible($h)) { $found.Add($h) }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if ($found.Count -gt 1) { throw 'An observation-only session displayed a control notice.' }
    Start-Sleep -Milliseconds 25
}
if (-not $observe.HasExited) { throw 'The observation-only command did not finish.' }
$observation = Get-Content -LiteralPath $observeOut -Raw | ConvertFrom-Json
if ($observe.ExitCode -ne 0 -or -not $observation.ok) { throw 'The observation-only session failed.' }
[pscustomobject]@{ ok = $true; text_pixels = $light; focused_window_preserved = $true; disappeared = $true; observation_has_no_notice = $true; screenshot = (Join-Path $OutputRoot 'notice.png') } | ConvertTo-Json -Compress
