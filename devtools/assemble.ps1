# Windows 原生插件组装器：生成 dist/<agent>/ 产物
# 用法: powershell -ExecutionPolicy Bypass -File devtools/assemble.ps1 <claude|codex|kiro|all>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Agent
)

$ErrorActionPreference = "Stop"
$validAgents = @("claude", "codex", "kiro", "pi", "all")

if ([string]::IsNullOrWhiteSpace($Agent)) {
    throw "用法: powershell -ExecutionPolicy Bypass -File devtools/assemble.ps1 <claude|codex|kiro|pi|all>"
}

$Agent = $Agent.ToLowerInvariant()
if ($validAgents -notcontains $Agent) {
    throw "未知 agent: $Agent（可选: claude, codex, kiro, pi, all）"
}

$scriptDir = if (-not [string]::IsNullOrWhiteSpace($PSScriptRoot)) {
    $PSScriptRoot
} else {
    Split-Path -Parent $MyInvocation.MyCommand.Path
}
$projectRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
$distDir = Join-Path $projectRoot "dist"
$utf8NoBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false

function Read-Utf8Text {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function Write-Utf8Text {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Content
    )

    [System.IO.File]::WriteAllText($Path, $Content, $script:utf8NoBom)
}

function Copy-DirectoryContents {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Source,
        [Parameter(Mandatory = $true)]
        [string]$Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        throw "找不到源目录: $Source"
    }

    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Split-FrontMatter {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text
    )

    $normalized = $Text -replace "`r`n", "`n"
    $fields = [ordered]@{}
    $body = $normalized

    if ($normalized.StartsWith("---`n")) {
        $end = $normalized.IndexOf("`n---", 4)
        if ($end -ge 0) {
            $frontMatter = $normalized.Substring(4, $end - 4)
            $bodyStart = $end + 4
            if ($bodyStart -lt $normalized.Length -and $normalized[$bodyStart] -eq "`n") {
                $bodyStart++
            }
            $body = $normalized.Substring($bodyStart)

            foreach ($line in ($frontMatter -split "`n")) {
                $colon = $line.IndexOf(":")
                if ($colon -lt 0) {
                    continue
                }
                $key = $line.Substring(0, $colon).Trim()
                $value = $line.Substring($colon + 1).Trim()
                $fields[$key] = $value
            }
        }
    }

    return [pscustomobject]@{
        Fields = $fields
        Body   = $body
    }
}

function Install-CommandSkills {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TargetDir,
        [Parameter(Mandatory = $true)]
        [bool]$ManagedMarker
    )

    $commandsDir = Join-Path $projectRoot "plugin\commands"
    $skillsDir = Join-Path $TargetDir "skills"
    $commandFiles = Get-ChildItem -LiteralPath $commandsDir -Filter "*.md" -File | Sort-Object Name

    foreach ($commandFile in $commandFiles) {
        $commandName = [System.IO.Path]::GetFileNameWithoutExtension($commandFile.Name)
        $parsed = Split-FrontMatter (Read-Utf8Text $commandFile.FullName)
        $description = $parsed.Fields["description"]
        if ([string]::IsNullOrWhiteSpace($description)) {
            $description = "执行 dev-flow /$commandName 流程"
        }

        $skillDescription = "$description。当用户要求执行 dev-flow $commandName 流程，或表达对应流程意图时使用。"
        $descriptionJson = ConvertTo-Json -InputObject $skillDescription -Compress
        $skillDir = Join-Path $skillsDir $commandName
        $skillFile = Join-Path $skillDir "SKILL.md"
        New-Item -ItemType Directory -Force -Path $skillDir | Out-Null

        $skillContent = "---`nname: $commandName`ndescription: $descriptionJson`n---`n`n$($parsed.Body.TrimStart())"
        Write-Utf8Text -Path $skillFile -Content $skillContent

        if ($ManagedMarker) {
            New-Item -ItemType File -Force -Path (Join-Path $skillDir ".dev-flow-managed") | Out-Null
        }
    }
}

function Update-ManifestVersion {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Source,
        [Parameter(Mandatory = $true)]
        [string]$Destination,
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    $content = Read-Utf8Text $Source
    $updated = [regex]::Replace($content, '("version"\s*:\s*")[^"]*(")', ('$1' + $Version + '$2'))
    Write-Utf8Text -Path $Destination -Content $updated
}

function Convert-AgentMarkdownToJson {
    param(
        [Parameter(Mandatory = $true)]
        [string]$MarkdownFile,
        [Parameter(Mandatory = $true)]
        [string]$JsonFile,
        [Parameter(Mandatory = $true)]
        [string]$HooksSource
    )

    $content = Read-Utf8Text $MarkdownFile
    $trimmed = $content.Trim()
    $lines = if ([string]::IsNullOrWhiteSpace($trimmed)) { @() } else { $trimmed -split "`r?`n" }
    $agentName = [System.IO.Path]::GetFileNameWithoutExtension($MarkdownFile)
    $description = if ($lines.Count -gt 0 -and $lines[0].StartsWith("#")) {
        $lines[0].TrimStart([char[]]"# ")
    } else {
        "$agentName agent"
    }

    $hooks = [ordered]@{}
    if (Test-Path -LiteralPath $HooksSource -PathType Leaf) {
        $hooksConfig = Read-Utf8Text $HooksSource | ConvertFrom-Json
        if ($null -ne $hooksConfig.hooks) {
            $hooks = $hooksConfig.hooks
        }
    }

    $config = [ordered]@{
        name         = $agentName
        description  = $description
        instructions = $content
        tools        = @("read", "write", "shell", "web_search", "web_fetch", "multi_tool_use.parallel")
        hooks        = $hooks
    }

    $json = $config | ConvertTo-Json -Depth 100
    Write-Utf8Text -Path $JsonFile -Content $json
}

function Assemble-Agent {
    param(
        [Parameter(Mandatory = $true)]
        [string]$AgentName,
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    $targetDir = Join-Path $distDir $AgentName
    if (Test-Path -LiteralPath $targetDir) {
        Remove-Item -LiteralPath $targetDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

    $sharedAgentsDir = Join-Path $projectRoot "plugin\agents"
    if ($AgentName -ne "kiro" -and $AgentName -ne "pi") {
        Copy-DirectoryContents -Source $sharedAgentsDir -Destination (Join-Path $targetDir "agents")
    }

    switch ($AgentName) {
        "claude" {
            Copy-DirectoryContents -Source (Join-Path $projectRoot "plugin\commands") -Destination (Join-Path $targetDir "commands")
            $pluginDir = Join-Path $targetDir ".claude-plugin"
            New-Item -ItemType Directory -Force -Path $pluginDir | Out-Null
            Update-ManifestVersion -Source (Join-Path $projectRoot "targets\claude\plugin.json") -Destination (Join-Path $pluginDir "plugin.json") -Version $Version
            Update-ManifestVersion -Source (Join-Path $projectRoot "targets\claude\marketplace.json") -Destination (Join-Path $pluginDir "marketplace.json") -Version $Version
            $hooksDir = Join-Path $targetDir "hooks"
            New-Item -ItemType Directory -Force -Path $hooksDir | Out-Null
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\claude\hooks.json") -Destination (Join-Path $hooksDir "hooks.json") -Force
        }
        "codex" {
            Install-CommandSkills -TargetDir $targetDir -ManagedMarker $false
            $pluginDir = Join-Path $targetDir ".codex-plugin"
            New-Item -ItemType Directory -Force -Path $pluginDir | Out-Null
            Update-ManifestVersion -Source (Join-Path $projectRoot "targets\codex\plugin.json") -Destination (Join-Path $pluginDir "plugin.json") -Version $Version
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\codex\app.json") -Destination (Join-Path $targetDir ".app.json") -Force
            $marketplaceDir = Join-Path $targetDir ".agents\plugins"
            New-Item -ItemType Directory -Force -Path $marketplaceDir | Out-Null
            Update-ManifestVersion -Source (Join-Path $projectRoot "targets\codex\personal-marketplace.json") -Destination (Join-Path $marketplaceDir "marketplace.json") -Version $Version
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\codex\hooks.json") -Destination (Join-Path $targetDir "hooks.json") -Force
            $hooksDir = Join-Path $targetDir "hooks"
            New-Item -ItemType Directory -Force -Path $hooksDir | Out-Null
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\codex\hooks.json") -Destination (Join-Path $hooksDir "hooks.json") -Force
        }
        "kiro" {
            Install-CommandSkills -TargetDir $targetDir -ManagedMarker $true
            $agentsDir = Join-Path $targetDir "agents"
            New-Item -ItemType Directory -Force -Path $agentsDir | Out-Null
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\kiro\agents\dev-flow\config.json") -Destination (Join-Path $agentsDir "dev-flow.json") -Force

            Get-ChildItem -LiteralPath $sharedAgentsDir -Filter "*.md" -File | Sort-Object Name | ForEach-Object {
                $jsonFile = Join-Path $agentsDir ($_.BaseName + ".json")
                Convert-AgentMarkdownToJson -MarkdownFile $_.FullName -JsonFile $jsonFile -HooksSource (Join-Path $projectRoot "targets\kiro\agents\dev-flow\config.json")
            }
        }
        "pi" {
            # Pi uses TypeScript extension + skills (same format as Codex)
            Copy-Item -LiteralPath (Join-Path $projectRoot "targets\pi\extension.ts") -Destination (Join-Path $targetDir "index.ts") -Force
            Install-CommandSkills -TargetDir $targetDir -ManagedMarker $false
        }
        default {
            throw "未知 agent: $AgentName"
        }
    }

    Write-Host "[assemble] ✓ $AgentName → dist/$AgentName/"
}

try {
    $versionRaw = if (Test-Path -LiteralPath (Join-Path $projectRoot "VERSION") -PathType Leaf) {
        (Read-Utf8Text (Join-Path $projectRoot "VERSION")).Trim()
    } else {
        "0.0.0"
    }

    $version = $versionRaw
    $lastClosingParen = $versionRaw.LastIndexOf(")")
    if ($lastClosingParen -ge 0 -and $lastClosingParen + 1 -lt $versionRaw.Length) {
        $version = $versionRaw.Substring($lastClosingParen + 1)
    }
    if ([string]::IsNullOrWhiteSpace($version)) {
        $version = $versionRaw
    }

    $agentsToAssemble = if ($Agent -eq "all") {
        @("claude", "codex", "kiro")
    } else {
        @($Agent)
    }

    foreach ($agentName in $agentsToAssemble) {
        Assemble-Agent -AgentName $agentName -Version $version
    }
} catch {
    Write-Host "[assemble] ✗ 组装失败: $($_.Exception.Message)" -ForegroundColor Red
    throw
}
