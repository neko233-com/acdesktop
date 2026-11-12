# acdesktop 一键安装脚本 —— Windows arm64
#
# 用法：
#   irm https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.ps1 | iex
#   irm https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.ps1 | iex -ArgumentList '--version','v1.2.0'
#   powershell -File scripts/install.ps1 -InstallDir C:\Tools\acdesktop
#
# 设计原则：
#   - 零 Node 依赖，直接下载原生二进制
#   - 单次下载、单次校验、原子替换
#   - 响应快：curl.exe + tar.exe 是 Windows 10+ 自带
#   - PATH 自动写入用户环境变量并刷新当前会话

[CmdletBinding()]
param(
  [string]$Version = '',
  [string]$InstallDir = ''
)

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'   # 关闭进度条加速 Invoke-WebRequest

$Repo       = 'neko233-com/acdesktop'
$GhReleases = "https://github.com/$Repo/releases"

# ---------- 平台检测 ----------
$IsArm64 = $env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or
           (Get-CimInstance Win32_OperatingSystem).OSArchitecture -like '*ARM*'
if (-not $IsArm64) {
  Write-Host '✗ acdesktop 一键安装脚本当前仅支持 Windows ARM64' -ForegroundColor Red
  Write-Host "  当前架构: $($env:PROCESSOR_ARCHITECTURE)" -ForegroundColor Red
  Write-Host "  请下载对应 release archive: $GhReleases" -ForegroundColor Red
  exit 1
}

$Target      = 'aarch64-pc-windows-msvc'
$BinaryName  = 'acdesktop.exe'
$ShimName    = 'acdesktop.cmd'
$DefaultDir  = Join-Path $HOME '.local\bin'
if (-not $InstallDir) { $InstallDir = $DefaultDir }

# ---------- 获取版本号 ----------
function Get-LatestTag {
  param([string]$ApiUrl)
  # 重定向头里就是 /releases/tag/vX.Y.Z —— 比完整 GET JSON 快
  try {
    $resp = curl.exe -fsSLI -o NUL -w '%{redirect_url}' "$ApiUrl/latest" 2>$null
    if ($LASTEXITCODE -eq 0 -and $resp -match '/tag/([v0-9.]+)') { return $Matches[1] }
  } catch {}
  # 回退到 API
  $latest = Invoke-RestMethod -Uri "$ApiUrl/latest" -Headers @{ Accept = 'application/vnd.github+json' }
  return $latest.tag_name
}

if ($Version) {
  $Tag = $Version
  if ($Tag -notmatch '^v') { $Tag = "v$Tag" }
} else {
  $Tag = Get-LatestTag -ApiUrl "https://api.github.com/repos/$Repo"
}
$Ver       = $Tag.TrimStart('v')
$Tarball   = "acdesktop-v$Ver-$Target.tar.gz"
$BaseUrl   = "$GhReleases/download/$Tag"
$TarballUrl    = "$BaseUrl/$Tarball"
$ChecksumsUrl  = "$BaseUrl/checksums.txt"

# ---------- 原子下载 + 校验 + 安装 ----------
$Staging = Join-Path ([IO.Path]::GetTempPath()) "acdesktop-install-$PID"
try {
  New-Item -ItemType Directory -Force -Path $Staging | Out-Null

  Write-Host "→ 下载 $Tarball …" -ForegroundColor Cyan
  curl.exe -fsSL --retry 3 --retry-delay 1 -o "$Staging\$Tarball"       "$TarballUrl"
  curl.exe -fsSL --retry 3 --retry-delay 1 -o "$Staging\checksums.txt"   "$ChecksumsUrl"

  Write-Host '→ SHA256 校验 …' -ForegroundColor Cyan
  $expected = (Select-String -Path "$Staging\checksums.txt" -Pattern "\s+$Tarball$" |
               Select-Object -First 1).Line.Split(' ')[0].ToLower()
  if (-not $expected) { throw "checksums.txt 中找不到 $Tarball" }
  $actual   = (Get-FileHash "$Staging\$Tarball" -Algorithm SHA256).Hash.ToLower()
  if ($actual -ne $expected) {
    throw "SHA256 不匹配！期望: $expected  实际: $actual"
  }

  Write-Host '→ 解压 …' -ForegroundColor Cyan
  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  tar.exe -xzf "$Staging\$Tarball" -C $Staging

  $SrcExe = Join-Path $Staging $BinaryName
  if (-not (Test-Path $SrcExe)) { throw "解压后找不到 $BinaryName" }

  # 原子替换：先写 .install 后缀，再 Move-Item 覆盖
  $DstExe   = Join-Path $InstallDir $BinaryName
  $DstShim  = Join-Path $InstallDir $ShimName
  $TmpExe   = Join-Path $InstallDir "$BinaryName.install.$PID"

  Copy-Item $SrcExe $TmpExe -Force
  Move-Item $TmpExe $DstExe -Force

  # 写 .cmd shim（直接转发，不走任何包装器）
  "@echo off`r`n`"$DstExe`" %*`r`nexit /b %errorlevel%" | Set-Content -Encoding Ascii $DstShim
}
finally {
  Remove-Item $Staging -Recurse -Force -ErrorAction SilentlyContinue
}

# ---------- PATH 处理 ----------
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$InstallDirNormalized = $InstallDir.TrimEnd('\')
$InPath = $false
if ($UserPath) {
  $paths = $UserPath -split ';'
  foreach ($p in $paths) {
    if (($p.TrimEnd('\') -ieq $InstallDirNormalized) -or
        ($p -ieq $InstallDir)) { $InPath = $true; break }
  }
}

$RefreshNeeded = $false
if (-not $InPath) {
  $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir }
             else { "$UserPath;$InstallDir" }
  [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
  $env:Path = "$InstallDir;$env:Path"
  $RefreshNeeded = $true
}

# ---------- 结果 ----------
Write-Host ''
Write-Host "✓ acdesktop v$Ver 安装成功 → $InstallDir" -ForegroundColor Green
Write-Host ''
if ($RefreshNeeded) {
  Write-Host "✓ 已将 $InstallDir 写入用户 PATH 并刷新当前会话" -ForegroundColor Green
} else {
  Write-Host "✓ $InstallDir 已在 PATH 中，可直接运行 acdesktop" -ForegroundColor Green
}
Write-Host ''
Write-Host '运行 acdesktop version 验证安装。'
