param(
    [Parameter(Mandatory)] [string] $Msi,
    [Parameter(Mandatory)] [string] $Binary,
    [switch] $Preview
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or -not [Environment]::Is64BitProcess -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw 'Run this fixture with 64-bit PowerShell 7 on Windows.'
}
$msiPath = (Resolve-Path -LiteralPath $Msi).Path
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$installer = New-Object -ComObject WindowsInstaller.Installer
function Read-MsiProperty([string] $Path, [string] $Name) {
    $db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Path, 0))
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @("SELECT ``Value`` FROM ``Property`` WHERE ``Property``='$Name'"))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
    try {
        $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(1))
    } finally {
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record)
        $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($db)
    }
}
function Product-State([string] $Product) {
    $installer.GetType().InvokeMember('ProductState', 'GetProperty', $null, $installer, @($Product))
}
$version = [version](Read-MsiProperty $msiPath 'ProductVersion')
$newProduct = Read-MsiProperty $msiPath 'ProductCode'
$marker = if ($Preview) { 'HKCU:\Software\acdesktop\GamePreview' } else { 'HKCU:\Software\acdesktop\Installation' }
if ((Product-State $newProduct) -ne -1 -or (Test-Path $marker)) {
    throw 'An acdesktop MSI installation already exists; it will not be modified.'
}
$previous = if ($Preview) { $version.ToString() }
elseif ($version.Build -gt 0) { "$($version.Major).$($version.Minor).$($version.Build - 1)" }
elseif ($version.Minor -gt 0) { "$($version.Major).$($version.Minor - 1).0" }
elseif ($version.Major -gt 0) { "$($version.Major - 1).0.0" }
else { throw 'A previous MSI version cannot be constructed.' }
$scratch = Join-Path $env:TEMP ('acdesktop-msi-upgrade-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $scratch | Out-Null
$oldMsi = Join-Path $scratch 'previous.msi'
$install = Join-Path $scratch 'installed'
$oldProduct = $null
$success = $false
$originalPath = [Environment]::GetEnvironmentVariable('Path', 'User')
try {
    $source = if ($Preview) { '../windows-game-preview.wxs' } else { '../windows-release.wxs' }
    & wix build (Join-Path $PSScriptRoot $source) -arch x64 -d "Version=$previous" -d "Binary=$binaryPath" -pdbtype none -out $oldMsi | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Could not build the previous-version MSI.' }
    $oldProduct = Read-MsiProperty $oldMsi 'ProductCode'
    if ((Product-State $oldProduct) -ne -1) { throw 'The synthetic previous product is already installed.' }
    $old = Start-Process msiexec.exe -ArgumentList @('/i', "`"$oldMsi`"", '/qn', '/norestart', "INSTALLFOLDER=`"$install`"") -WindowStyle Hidden -Wait -PassThru
    if ($old.ExitCode -ne 0 -or (Product-State $oldProduct) -ne 5) { throw 'Previous-version MSI installation failed.' }
    $upgrade = Start-Process msiexec.exe -ArgumentList @('/i', "`"$msiPath`"", '/qn', '/norestart') -WindowStyle Hidden -Wait -PassThru
    if ($upgrade.ExitCode -ne 0 -or (Product-State $oldProduct) -ne -1 -or (Product-State $newProduct) -ne 5) {
        throw 'MSI upgrade did not replace the previous product.'
    }
    $installed = Join-Path $install 'acdesktop.exe'
    if (-not (Test-Path -LiteralPath $installed) -or (Get-FileHash $installed).Hash -ne (Get-FileHash $binaryPath).Hash) {
        throw 'The upgrade did not preserve the install directory and binary.'
    }
    if (-not $Preview) {
        $entries = @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | ForEach-Object { $_.TrimEnd('\') } | Where-Object { $_ -eq $install.TrimEnd('\') })
        if ($entries.Count -ne 1) { throw 'The upgrade did not preserve exactly one PATH entry.' }
    }
    $remove = Start-Process msiexec.exe -ArgumentList @('/x', $newProduct, '/qn', '/norestart') -WindowStyle Hidden -Wait -PassThru
    $beforeEntries = @($originalPath -split ';' | Where-Object { $_ }) -join "`n"
    $afterEntries = @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ }) -join "`n"
    if ($remove.ExitCode -ne 0 -or (Product-State $newProduct) -ne -1 -or (Test-Path -LiteralPath $installed) -or
        $afterEntries -cne $beforeEntries) {
        throw 'The upgraded MSI did not uninstall cleanly.'
    }
    $success = $true
    [pscustomobject]@{ ok = $true; from = $previous; to = $version.ToString(); install_directory_preserved = $true; path_restored = $true } | ConvertTo-Json -Compress
} finally {
    foreach ($product in @($newProduct, $oldProduct)) {
        if ($product -and (Product-State $product) -eq 5) {
            Start-Process msiexec.exe -ArgumentList @('/x', $product, '/qn', '/norestart') -WindowStyle Hidden -Wait | Out-Null
        }
    }
    if ($success) {
        $info = Get-Item -LiteralPath $scratch -Force
        $tempRoot = [IO.Path]::GetFullPath($env:TEMP).TrimEnd('\') + '\'
        $resolved = [IO.Path]::GetFullPath($info.FullName)
        if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or ($info.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Temporary MSI test directory failed the cleanup safety check.'
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
