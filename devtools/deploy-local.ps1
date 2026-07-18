# Windows 本地开发部署：编译 + 组装 + 模拟安装 + dow setup
# 用法: powershell -ExecutionPolicy Bypass -File devtools/deploy-local.ps1 <claude|codex|kiro|all>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Agent
)

$ErrorActionPreference = "Stop"

$validAgents = @("claude", "codex", "kiro", "all")
if ([string]::IsNullOrWhiteSpace($Agent)) {
    Write-Error "用法: powershell -ExecutionPolicy Bypass -File devtools/deploy-local.ps1 <claude|codex|kiro|all>"
    exit 1
}

$Agent = $Agent.ToLowerInvariant()
if ($validAgents -notcontains $Agent) {
    Write-Error "未知 agent: $Agent（可选: claude, codex, kiro, all）"
    exit 1
}

$scriptDir = if (-not [string]::IsNullOrWhiteSpace($PSScriptRoot)) {
    $PSScriptRoot
} else {
    Split-Path -Parent $MyInvocation.MyCommand.Path
}
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
$userProfile = if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
    $env:USERPROFILE
} else {
    [Environment]::GetFolderPath("UserProfile")
}

if ([string]::IsNullOrWhiteSpace($userProfile)) {
    throw "无法确定用户主目录（USERPROFILE 未设置）"
}

$binDir = Join-Path $userProfile ".local\bin"
$dataDir = if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    Join-Path $env:LOCALAPPDATA "dow"
} else {
    Join-Path $userProfile "AppData\Local\dow"
}
$bundleDir = Join-Path $dataDir "bundle"
$dowPath = Join-Path $binDir "dow.exe"
$builtDowPath = Join-Path $projectRoot "dow\target\release\dow.exe"
$assembleScript = Join-Path $scriptDir "assemble.ps1"

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,
        [Parameter()]
        [string[]]$Arguments = @()
    )

    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "找不到命令: $Command。请确认它已安装并已加入 PATH。"
    }

    $displayCommand = $Command
    if ($Arguments.Count -gt 0) {
        $displayCommand += " " + ($Arguments -join " ")
    }

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "命令失败（退出码 $LASTEXITCODE）: $displayCommand"
    }
}

try {
    # 1. 编译 dow
    Write-Host "[deploy] 编译 dow..."
    Push-Location (Join-Path $projectRoot "dow")
    try {
        Invoke-NativeCommand -Command "cargo" -Arguments @("build", "--release")
    } finally {
        Pop-Location
    }

    if (-not (Test-Path -LiteralPath $builtDowPath -PathType Leaf)) {
        throw "编译完成但未找到 dow 二进制: $builtDowPath"
    }

    # 2. 组装 bundle
    Write-Host "[deploy] 组装插件..."
    if (-not (Test-Path -LiteralPath $assembleScript -PathType Leaf)) {
        throw "找不到组装脚本: $assembleScript"
    }
    & $assembleScript -Agent $Agent
    if (-not $?) {
        throw "PowerShell 组装脚本失败: $assembleScript"
    }

    # 3. 部署 dow 二进制（模拟 install.ps1 下载后的放置）
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    if (Test-Path -LiteralPath $dowPath) {
        Remove-Item -LiteralPath $dowPath -Force
    }
    Copy-Item -LiteralPath $builtDowPath -Destination $dowPath -Force
    Write-Host "[deploy] ✓ dow → $dowPath"

    # 4. 部署 bundle（模拟 install.ps1 解压后的放置）
    if (Test-Path -LiteralPath $bundleDir) {
        Remove-Item -LiteralPath $bundleDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null

    $bundleAgents = if ($Agent -eq "all") {
        @("claude", "codex", "kiro")
    } else {
        @($Agent)
    }

    foreach ($bundleAgent in $bundleAgents) {
        $sourceDir = Join-Path $projectRoot "dist\$bundleAgent"
        $destinationDir = Join-Path $bundleDir $bundleAgent

        if (-not (Test-Path -LiteralPath $sourceDir -PathType Container)) {
            throw "找不到已组装的 bundle: $sourceDir"
        }
        Copy-Item -LiteralPath $sourceDir -Destination $destinationDir -Recurse -Force
    }
    Write-Host "[deploy] ✓ bundle → $bundleDir"

    # 5. 调用 dow setup 完成正式注册
    Write-Host "[deploy] 运行 dow setup..."
    Invoke-NativeCommand -Command $dowPath -Arguments @("setup", "--agent", $Agent)

    # 6. Kiro: prompt user to set default agent (hooks require it)
    if ($Agent -eq "kiro" -or $Agent -eq "all") {
        Write-Host ""
        Write-Host "[deploy] ⚠ Kiro hooks require the dev-flow agent to be set as default." -ForegroundColor Yellow
        Write-Host "         (kiro-default does not support hook configuration)"
        Write-Host ""
        $answer = Read-Host "[deploy] Run 'kiro-cli agent set-default --name dev-flow' now? [Y/n]"
        if ([string]::IsNullOrWhiteSpace($answer) -or $answer -match '^[Yy]') {
            if (Get-Command "kiro-cli" -ErrorAction SilentlyContinue) {
                Invoke-NativeCommand -Command "kiro-cli" -Arguments @("agent", "set-default", "--name", "dev-flow")
                Write-Host "[deploy] ✓ dev-flow set as kiro default agent"
            } else {
                Write-Host "[deploy] ✗ kiro-cli not found in PATH — please run manually:" -ForegroundColor Yellow
                Write-Host "         kiro-cli agent set-default --name dev-flow"
            }
        } else {
            Write-Host "[deploy] Skipped. Run manually when ready:"
            Write-Host "         kiro-cli agent set-default --name dev-flow"
        }
    }

    Write-Host "[deploy] 完成！" -ForegroundColor Green
} catch {
    Write-Error "[deploy] ✗ 部署失败: $($_.Exception.Message)"
    exit 1
}
