# new-project.ps1 — Bootstrap a new project from the Execution Blueprint
# Usage:  .\new-project.ps1 -Target "V:\AI\MyNewProject"
# Copies CLAUDE.md, docs templates, and skills into the target, creates the
# standard folders, and initializes git if needed. Never overwrites existing files.

param(
    [Parameter(Mandatory = $true)]
    [string]$Target
)

$ErrorActionPreference = 'Stop'
$blueprint = $PSScriptRoot

if (-not (Test-Path $Target)) {
    New-Item -ItemType Directory -Path $Target -Force | Out-Null
    Write-Host "Created $Target"
}

function Copy-IfMissing($src, $dst) {
    if (Test-Path $dst) {
        Write-Host "SKIP (exists): $dst" -ForegroundColor Yellow
    } else {
        $parent = Split-Path $dst -Parent
        if (-not (Test-Path $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
        Copy-Item $src $dst
        Write-Host "COPIED: $dst" -ForegroundColor Green
    }
}

# Root files
Copy-IfMissing (Join-Path $blueprint 'CLAUDE.md')    (Join-Path $Target 'CLAUDE.md')
Copy-IfMissing (Join-Path $blueprint 'PLAYBOOK.md')  (Join-Path $Target 'PLAYBOOK.md')

# Docs templates
Get-ChildItem (Join-Path $blueprint 'docs') -Filter *.md | ForEach-Object {
    Copy-IfMissing $_.FullName (Join-Path $Target "docs\$($_.Name)")
}

# Skills
Get-ChildItem (Join-Path $blueprint '.claude\skills') -Directory | ForEach-Object {
    Copy-IfMissing (Join-Path $_.FullName 'SKILL.md') (Join-Path $Target ".claude\skills\$($_.Name)\SKILL.md")
}

# Standard folders
foreach ($dir in @('src', 'tests', 'spikes')) {
    $p = Join-Path $Target $dir
    if (-not (Test-Path $p)) { New-Item -ItemType Directory -Path $p | Out-Null; Write-Host "CREATED: $p" -ForegroundColor Green }
}

# .gitignore starter (only if missing)
$gi = Join-Path $Target '.gitignore'
if (-not (Test-Path $gi)) {
    @"
__pycache__/
*.pyc
.pytest_cache/
node_modules/
dist/
build/
.env
*.log
"@ | Out-File $gi -Encoding utf8
    Write-Host "CREATED: $gi" -ForegroundColor Green
}

# Git init
if (-not (Test-Path (Join-Path $Target '.git'))) {
    Push-Location $Target
    git init | Out-Null
    git add -A
    git commit -m "[SETUP] Bootstrap from Project Execution Blueprint" | Out-Null
    Pop-Location
    Write-Host "Git initialized with bootstrap commit." -ForegroundColor Green
}

Write-Host ""
Write-Host "Done. Next: open Claude Code in $Target and run /kickoff <your idea>" -ForegroundColor Cyan
