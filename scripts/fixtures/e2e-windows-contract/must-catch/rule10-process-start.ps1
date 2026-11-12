function Invoke-VersionLeg {
    [System.Diagnostics.Process]::Start('acdesktop.exe', 'version')
}
