param(
    [Parameter(Mandatory)] [string] $WindowId,
    [Parameter(Mandatory)] [string] $OutputRoot,
    [string] $Binary = (Join-Path $PSScriptRoot '../../target/x86_64-pc-windows-msvc/release/acdesktop.exe')
)

$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or -not [Environment]::Is64BitProcess -or -not $IsWindows) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
if (Test-Path -LiteralPath $OutputRoot) { throw 'Choose a new OutputRoot.' }
$windows = & $Binary list-windows --app CalculatorApp.exe | ConvertFrom-Json
if (-not $windows.ok) { throw "Cannot list calculator windows: $($windows.error.message)" }
$window = @($windows.data | Where-Object id -eq $WindowId)
if ($window.Count -ne 1 -or $WindowId -notmatch '^w-([0-9]+)$') { throw 'WindowId must identify one Calculator window.' }
$handle = [IntPtr]([long]$Matches[1])
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class GameCalculatorOrigin {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X; public int Y; }
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
}
'@
$origin = [GameCalculatorOrigin+Point]::new()
if (-not [GameCalculatorOrigin]::ClientToScreen($handle, [ref]$origin)) { throw 'Cannot determine calculator client origin.' }
function Get-Node($node, $id) {
    if ($null -eq $node) { return }
    if ($node.native_id.value -eq $id) { $node }
    foreach ($child in $node.children) { Get-Node $child $id }
}
$before = & $Binary snapshot --window-id $WindowId --include-bounds | ConvertFrom-Json
if (-not $before.ok) { throw "Cannot read calculator: $($before.error.message)" }
$seven = @(Get-Node $before.data.tree 'num7Button')
$two = @(Get-Node $before.data.tree 'num2Button')
$display = @(Get-Node $before.data.tree 'CalculatorResults')
if ($seven.Count -ne 1 -or $two.Count -ne 1 -or $display.Count -ne 1 -or $display[0].name -notmatch '(^|\D)0$') {
    throw 'Start with one visible calculator showing zero.'
}
function Center($node) {
    return @([int]($node.bounds.x + $node.bounds.width / 2 - $origin.X), [int]($node.bounds.y + $node.bounds.height / 2 - $origin.Y))
}
$p7 = Center $seven[0]
$p2 = Center $two[0]
$plan = @{
    duration_ms = 1900; fps = 15; capture_cursor = $true
    events = @(
        @{ at_ms = 100; action = 'mouse-move'; x = $p7[0]; y = $p7[1] },
        @{ at_ms = 300; action = 'gesture'; gesture = 'tap'; x = $p7[0]; y = $p7[1] },
        @{ at_ms = 800; action = 'mouse-move'; x = $p2[0]; y = $p2[1] },
        @{ at_ms = 1000; action = 'gesture'; gesture = 'tap'; x = $p2[0]; y = $p2[1] }
    )
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$planPath = Join-Path $OutputRoot 'plan.json'
$plan | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $planPath
$result = & $Binary game --window-id $WindowId --plan $planPath --out (Join-Path $OutputRoot 'session') --save-frames --headed | ConvertFrom-Json
if (-not $result.ok) { throw "Game session failed: $($result.error | ConvertTo-Json -Depth 6 -Compress)" }
$after = & $Binary snapshot --window-id $WindowId | ConvertFrom-Json
if (-not $after.ok) { throw "Cannot read the result: $($after.error.message)" }
$display = @(Get-Node $after.data.tree 'CalculatorResults')
if ($display.Count -ne 1 -or $display[0].name -notmatch '(^|\D)72$') { throw 'Calculator did not display 72 after the two taps.' }
if ($result.data.frames_saved -lt 1) { throw 'No game frames were saved.' }
[pscustomobject]@{ ok = $true; display = $display[0].name; frames_saved = $result.data.frames_saved; input_metrics = $result.data.summary.input_metrics; output = $OutputRoot } | ConvertTo-Json -Depth 6 -Compress
