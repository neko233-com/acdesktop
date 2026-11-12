function Invoke-VersionLeg {
    Start-Job -ScriptBlock { acdesktop.exe version }
}
