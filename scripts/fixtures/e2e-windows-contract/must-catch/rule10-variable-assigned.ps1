function Invoke-VersionLeg {
    $binary = 'target\release\acdesktop.exe'
    & $binary version
}
