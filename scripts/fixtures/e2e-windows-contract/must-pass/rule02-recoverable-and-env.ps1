function Remove-SuiteRoot {
    param([string]$Path)
    Remove-ItemRecoverable -Path $Path
    Remove-Item Env:\ACDESKTOP_SESSION -ErrorAction SilentlyContinue
}
