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
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Path $OutputRoot | Out-Null
$runs = @{}
foreach ($mode in @('off', 'on')) {
    $directory = Join-Path $OutputRoot $mode
    $plan = Join-Path $PSScriptRoot "cursor-$mode.json"
    $result = & $Binary game --window-id $WindowId --plan $plan --out $directory --headed --save-frames | ConvertFrom-Json
    if (-not $result.ok) {
        throw "The $mode session failed: $($result.error | ConvertTo-Json -Compress)"
    }
    $manifest = @(Get-Content -LiteralPath (Join-Path $directory 'frames/frames.jsonl') | ForEach-Object { $_ | ConvertFrom-Json })
    if ($manifest.Count -ne $result.data.frames_saved -or $manifest.Count -lt 3) {
        throw "The $mode frame manifest does not match the saved-frame count."
    }
    if ($result.data.summary.cursor_in_saved_frames -ne ($mode -eq 'on')) {
        throw "The $mode session reported the wrong cursor setting."
    }
    $runs[$mode] = @{ directory = $directory; manifest = $manifest; result = $result }
}

function Get-Frame([string] $mode, [int] $atMs) {
    $row = $runs[$mode].manifest | Sort-Object { [Math]::Abs($_.at_ms - $atMs) } | Select-Object -First 1
    if ([Math]::Abs($row.at_ms - $atMs) -gt 120) {
        throw "No $mode frame was captured near $atMs ms."
    }
    return [System.Drawing.Bitmap]::new((Join-Path $runs[$mode].directory "frames/$($row.file)"))
}

$checks = @()
foreach ($probe in @(@(550, 300), @(1250, 500))) {
    $off = Get-Frame 'off' $probe[0]
    $on = Get-Frame 'on' $probe[0]
    try {
        if ($off.Width -ne $on.Width -or $off.Height -ne $on.Height) {
            throw 'The fixture window changed size between captures.'
        }
        if ($off.Width -le $probe[1] -or $off.Height -le 280) {
            throw 'The captured window is too small for the cursor test coordinates.'
        }
        $changed = 0
        for ($y = 280; $y -lt [Math]::Min(410, $off.Height); $y++) {
            for ($x = [Math]::Max(0, $probe[1] - 40); $x -lt [Math]::Min($probe[1] + 70, $off.Width); $x++) {
                $a = $off.GetPixel($x, $y)
                $b = $on.GetPixel($x, $y)
                if ([Math]::Abs([int]$a.R - [int]$b.R) -gt 30 -or [Math]::Abs([int]$a.G - [int]$b.G) -gt 30 -or [Math]::Abs([int]$a.B - [int]$b.B) -gt 30) {
                    $changed++
                }
            }
        }
        if ($changed -lt 20) {
            throw "No cursor-shaped difference was observed near x=$($probe[1])."
        }
        $checks += [pscustomobject]@{ at_ms = $probe[0]; cursor_x = $probe[1]; changed_pixels = $changed }
    } finally {
        $off.Dispose()
        $on.Dispose()
    }
}
[pscustomobject]@{ ok = $true; checks = $checks; output = $OutputRoot } | ConvertTo-Json -Depth 4
