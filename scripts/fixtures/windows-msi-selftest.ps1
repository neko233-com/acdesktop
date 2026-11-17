param(
    [Parameter(Mandatory)] [string] $Msi,
    [string] $ExpectedBinary,
    [switch] $Release
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or -not [Environment]::Is64BitProcess -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
$msiPath = (Resolve-Path -LiteralPath $Msi).Path
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($msiPath, 0))
$view = $database.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $database, @("SELECT ``Value`` FROM ``Property`` WHERE ``Property``='ProductCode'"))
$view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
$record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
$product = $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(1))
$marker = if ($Release) { 'HKCU:\Software\acdesktop\Installation' } else { 'HKCU:\Software\acdesktop\GamePreview' }
$originalPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$state = $installer.GetType().InvokeMember('ProductState', 'GetProperty', $null, $installer, @($product))
if ($state -ne -1 -or (Test-Path -LiteralPath $marker)) {
    throw 'A preview installation already exists; it will not be modified.'
}
$scratch = Join-Path $env:TEMP ('acdesktop-msi-selftest-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
$install = Join-Path $scratch 'installed'
$log = Join-Path $scratch 'install.log'
$installed = $false
$success = $false
try {
    $process = Start-Process -FilePath msiexec.exe -ArgumentList @('/i', "`"$msiPath`"", '/qn', '/norestart', "INSTALLFOLDER=`"$install`"", '/L*v', "`"$log`"") -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "MSI install failed with exit code $($process.ExitCode); log: $log" }
    $installed = $true
    $binary = Join-Path $install 'acdesktop.exe'
    $state = $installer.GetType().InvokeMember('ProductState', 'GetProperty', $null, $installer, @($product))
    if (-not (Test-Path -LiteralPath $binary) -or $state -ne 5) {
        throw "MSI did not install the binary and register the product; log: $log"
    }
    if ($ExpectedBinary -and (Get-FileHash -LiteralPath $binary).Hash -ne (Get-FileHash -LiteralPath $ExpectedBinary).Hash) {
        throw "The installed binary differs from the expected binary; log: $log"
    }
    if ($Release) {
        $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @($currentPath -split ';' | ForEach-Object { $_.TrimEnd('\') })
        if ($entries -notcontains $install.TrimEnd('\')) { throw 'MSI did not add its install directory to the user PATH.' }
    }
    $version = & $binary version | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or -not $version.ok -or $version.data.target -ne 'x86_64') {
        throw "The installed binary did not run; log: $log"
    }
    $uninstallLog = Join-Path $scratch 'uninstall.log'
    $remove = Start-Process -FilePath msiexec.exe -ArgumentList @('/x', $product, '/qn', '/norestart', '/L*v', "`"$uninstallLog`"") -WindowStyle Hidden -Wait -PassThru
    if ($remove.ExitCode -ne 0) { throw "MSI uninstall failed with exit code $($remove.ExitCode)." }
    $installed = $false
    $state = $installer.GetType().InvokeMember('ProductState', 'GetProperty', $null, $installer, @($product))
    if ((Test-Path -LiteralPath $binary) -or $state -ne -1) {
        throw "MSI uninstall left its binary or product registration behind; log: $uninstallLog"
    }
    $beforeEntries = @($originalPath -split ';' | Where-Object { $_ }) -join "`n"
    $afterEntries = @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ }) -join "`n"
    if ($Release -and $afterEntries -cne $beforeEntries) {
        throw 'MSI uninstall did not restore the original user PATH.'
    }
    $success = $true
    [pscustomobject]@{ ok = $true; install = 'per-user'; version = $version.data.version; uninstall = 'verified' } | ConvertTo-Json -Compress
} finally {
    if ($installed) {
        Start-Process -FilePath msiexec.exe -ArgumentList @('/x', $product, '/qn', '/norestart') -WindowStyle Hidden -Wait | Out-Null
    }
    if ($success) {
        $scratchInfo = Get-Item -LiteralPath $scratch -Force
        $tempRoot = [System.IO.Path]::GetFullPath($env:TEMP).TrimEnd('\') + '\'
        $resolvedScratch = [System.IO.Path]::GetFullPath($scratchInfo.FullName)
        if (-not $resolvedScratch.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
            ($scratchInfo.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
            throw 'Temporary MSI test directory failed the cleanup safety check.'
        }
        Remove-Item -LiteralPath $resolvedScratch -Recurse -Force
    }
}
