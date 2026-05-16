param(
  [Parameter(Mandatory = $true)]
  [string]$Version,
  [switch]$NoWait
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$version = $Version.TrimStart("v")
$tag = "v$version"
$repo = "Luvroxanne/ai-report-factory"

function Step([string]$Text) {
  Write-Host ""
  Write-Host "==> $Text" -ForegroundColor Cyan
}

Push-Location $root
try {
  Step "Sync version $tag"
  python scripts\bump_version.py $version

  Step "Run local checks"
  npm.cmd --prefix frontend run type-check
  python -m compileall backend\app

  Step "Build backend sidecar for Tauri resource check"
  powershell -ExecutionPolicy Bypass -File scripts\bundle-backend.ps1

  Step "Run Rust check"
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  cargo check --manifest-path src-tauri\Cargo.toml

  Step "Commit release changes"
  git fetch origin --tags
  git rev-parse $tag 1>$null 2>$null
  if ($LASTEXITCODE -eq 0) {
    throw "Version tag already exists: $tag. Please use a new version."
  }

  git add .
  $changes = git diff --cached --name-only
  if (-not $changes) {
    throw "No staged changes. Stop release."
  }

  git commit -m "chore: release $tag"
  git tag $tag

  Step "Push main and tag"
  git push origin main
  git push origin $tag

  Write-Host ""
  Write-Host "Release workflow has been triggered." -ForegroundColor Green
  Write-Host "Actions:  https://github.com/$repo/actions"
  Write-Host "Releases: https://github.com/$repo/releases"

  if (-not $NoWait -and (Get-Command gh -ErrorAction SilentlyContinue)) {
    Step "Wait for GitHub Actions"
    $runId = ""
    for ($i = 0; $i -lt 60; $i++) {
      $runId = gh run list --repo $repo --workflow "Release Windows" --limit 5 --json databaseId,headBranch,event --jq ".[] | select(.headBranch == `"$tag`") | .databaseId" | Select-Object -First 1
      if ($runId) { break }
      Start-Sleep -Seconds 5
    }

    if ($runId) {
      gh run watch $runId --repo $repo --exit-status
      Write-Host ""
      Write-Host "Release finished:" -ForegroundColor Green
      Write-Host "https://github.com/$repo/releases/tag/$tag"
    } else {
      Write-Host "Tag was pushed, but no workflow run was found yet. Open Actions page manually." -ForegroundColor Yellow
    }
  } elseif (-not $NoWait) {
    Write-Host ""
    Write-Host "GitHub CLI is not installed locally. The cloud release still runs automatically." -ForegroundColor Yellow
  }
} finally {
  Pop-Location
}
