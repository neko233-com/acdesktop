# Unity touch fixture

This fixture records Unity's legacy input events independently of the
`acdesktop game` report. Use a disposable Unity 2022.3 Windows project.
The fixture resets `%TEMP%\acdesktop-unity-gesture-input.csv` on launch
and writes a development player to `%TEMP%\acdesktop-gesture-build`.

To run the focused game, gesture, touch, capture and CLI tests and build a
local x64 preview archive, use a new output directory with 64-bit PowerShell 7:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-preview-' + [guid]::NewGuid().ToString('N'))
./scripts/package-windows-game-preview.ps1 -OutputDirectory $out
```

This is a local preview, not a release or a substitute for the full platform
test suite and target-game acceptance.

For an additional Windows application check, open Calculator and leave it
showing zero. Find its window id with `acdesktop list-windows --app
CalculatorApp.exe`, then run from the repository root:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-calculator-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/game-calculator-check.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```

The check taps 7 and 2, reads Calculator's display independently, and saves
cursor-enabled frames. It requires the target window to stay unobstructed and
in the foreground while the plan runs.

Create a project and install the fixture from PowerShell 7:

```powershell
$lab = Join-Path $env:TEMP 'acdesktop-gesture-lab'
unity projects create acdesktop-gesture-lab --path $env:TEMP --editor-version 2022.3.51f1 --template 2d --no-cloud --bare --non-interactive
New-Item -ItemType Directory -Force -Path (Join-Path $lab 'Assets/Editor') | Out-Null
Copy-Item ./tests/e2e-windows/unity-gesture-fixture/GestureProbe.cs (Join-Path $lab 'Assets/')
Copy-Item ./tests/e2e-windows/unity-gesture-fixture/GameplayArena.cs (Join-Path $lab 'Assets/')
Copy-Item ./tests/e2e-windows/unity-gesture-fixture/GestureBuilder.cs (Join-Path $lab 'Assets/Editor/')
unity build $lab --editor-version 2022.3.51f1 --target StandaloneWindows64 --execute-method GestureBuilder.Build --timeout 600 --no-tail
```

Start `%TEMP%\acdesktop-gesture-build\GestureLab.exe` in windowed mode.
Use `acdesktop list-windows` to obtain its exact window id, then run:

```powershell
acdesktop game --window-id 'w-REPLACE' --plan ./tests/e2e-windows/unity-gesture-fixture/plan.json --out (Join-Path $env:TEMP 'acdesktop-unity-gesture-run') --headed
Import-Csv (Join-Path $env:TEMP 'acdesktop-unity-gesture-input.csv') | Group-Object phase | Select-Object Name, Count
```

The plan uses an 800 × 600 client area and exercises an S, a circle, a
two-finger tap, and a held joystick with a second tap. Check the CSV for
`Began`, `Moved`, `Ended`, and `count` values of at least 2. Unity uses a
bottom-left origin while the plan uses a top-left origin. Substitute the
window id and ensure the window is unobstructed. Command success records
injection, not receipt by Unity.

To check the recorded S, clockwise circle, two-finger tap and joystick drag
against Unity's touch log, use a fresh output directory:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-gestures-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/unity-gesture-fixture/verify-gestures.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```

To check pinch-in, pinch-out and both rotation directions against Unity's
two-finger touch log:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-pinch-rotate-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/unity-gesture-fixture/verify-pinch-rotate.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```

The fixture also contains a small joystick-and-fire game. To check the game
state independently, run the following against a freshly started fixture:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-gameplay-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/unity-gesture-fixture/verify-gameplay.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```

The script checks that firing before movement fails, then checks three
simultaneous joystick-and-fire actions, their scores and their intervals in
Unity's gameplay log. The intervals measure this test's rhythm; they do not
measure a player's subjective flow.

Run `three-touch.json` separately and check the CSV for `count = 3`.
On Windows 11, first turn off **Three- and four-finger touch gestures** in
**Settings → Bluetooth & devices → Touch**, and restore the setting afterward
if you need Windows gestures. The plan acknowledges the system-gesture risk;
the command cannot verify the setting. Windows or the target may consume
multi-finger gestures, so do not infer receipt from `ok: true`. This fixture
is for desktop-player coverage, not mobile device acceptance. To verify both a
three-finger hold and swipe against Unity's event log, after adjusting the
Windows setting run:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-three-touch-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/unity-gesture-fixture/verify-three-touch.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```

To verify that saved frames actually include a moving cursor, keep the fixture
window open at 800 × 600 and run the following with its window id. The script
uses two new output directories, compares cursor-on and cursor-off frames at
two positions, and checks the frame timestamp manifests:

```powershell
$out = Join-Path $env:TEMP ('acdesktop-cursor-' + [guid]::NewGuid().ToString('N'))
./tests/e2e-windows/unity-gesture-fixture/verify-cursor.ps1 -WindowId 'w-REPLACE' -OutputRoot $out
```
