# dow 安装脚本 (Windows PowerShell)
# 用法: irm https://raw.githubusercontent.com/daphnee-ovo/dev-flow/main/install/install.ps1 | iex
$ErrorActionPreference = "Stop"

$REPO = "daphnee-ovo/dev-flow"
$BIN_DIR = "$env:USERPROFILE\.local\bin"
$DATA_DIR = "$env:USERPROFILE\.local\share\dow"
$BUNDLE_DIR = "$DATA_DIR\bundle"

function Info($msg) { Write-Host "[dow] $msg" -ForegroundColor Blue }
function Ok($msg)   { Write-Host "[dow] ✓ $msg" -ForegroundColor Green }
function Err($msg)  { Write-Host "[dow] ✗ $msg" -ForegroundColor Red; exit 1 }

# 检测平台
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "X64") {
    "x86_64"
} else {
    Err "不支持的架构: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)"
}
$platform = "windows-$arch"
Info "平台: $platform"

# 获取最新版本
Info "获取最新版本..."
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -Headers @{ "User-Agent" = "dow-installer" }
$version = $release.tag_name
if (-not $version) { Err "无法获取最新版本" }
Info "版本: $version"

# 下载
$filename = "dow-$version-$platform.tar.gz"
$url = "https://github.com/$REPO/releases/download/$version/$filename"
$tmpDir = Join-Path $env:TEMP "dow-install"
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tarball = Join-Path $tmpDir $filename

Info "下载 $filename..."
Invoke-WebRequest -Uri $url -OutFile $tarball -UseBasicParsing

# 解压
Info "安装中..."
tar -xzf $tarball -C $tmpDir

# 安装二进制
New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
$binSrc = if (Test-Path "$tmpDir\bin\dow.exe") { "$tmpDir\bin\dow.exe" }
           elseif (Test-Path "$tmpDir\dow.exe") { "$tmpDir\dow.exe" }
           else { Err "tarball 中未找到 dow.exe" }
Copy-Item $binSrc "$BIN_DIR\dow.exe" -Force

# 安装 bundle
if (Test-Path "$tmpDir\bundle") {
    if (Test-Path $BUNDLE_DIR) { Remove-Item -Recurse -Force $BUNDLE_DIR }
    Copy-Item "$tmpDir\bundle" $BUNDLE_DIR -Recurse
}

# 清理
Remove-Item -Recurse -Force $tmpDir

Ok "dow $version 已安装到 $BIN_DIR\dow.exe"

# 检查 PATH
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$BIN_DIR*") {
    Info "将 $BIN_DIR 添加到 PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$BIN_DIR;$userPath", "User")
    $env:PATH = "$BIN_DIR;$env:PATH"
    Info "已添加到 User PATH（新终端窗口生效）"
}

Write-Host ""
# 运行 setup
Info "启动设置引导..."
& "$BIN_DIR\dow.exe" setup
