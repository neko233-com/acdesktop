function Invoke-VersionLeg {
    Start-Process -FilePath 'target\release\acdesktop.exe' -ArgumentList @('version')
}
