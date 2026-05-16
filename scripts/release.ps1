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

function Step($Text) {
  Write-Host ""
  Write-Host "==> $Text" -ForegroundColor Cyan
}

Push-Location $root
try {
  Step "同步版本号：$tag"
  python scripts\bump_version.py $version

  Step "本地检查"
  npm.cmd --prefix frontend run type-check
  python -m compileall backend\app
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  cargo check --manifest-path src-tauri\Cargo.toml

  Step "提交代码"
  git fetch origin --tags
  git rev-parse $tag 1>$null 2>$null
  if ($LASTEXITCODE -eq 0) {
    throw "版本 $tag 已存在，请换一个新版本号，例如 $version 后面递增。"
  }

  git add .
  $changes = git diff --cached --name-only
  if (-not $changes) {
    throw "没有可提交的改动，已停止发版。"
  }
  git commit -m "chore: release $tag"
  git tag $tag

  Step "推送 main 和 tag，GitHub Actions 会自动创建 Release"
  git push origin main
  git push origin $tag

  Write-Host ""
  Write-Host "已触发自动发版：" -ForegroundColor Green
  Write-Host "Actions:  https://github.com/$repo/actions"
  Write-Host "Releases: https://github.com/$repo/releases"

  if (-not $NoWait -and (Get-Command gh -ErrorAction SilentlyContinue)) {
    Step "等待 GitHub Actions 完成"
    $runId = ""
    for ($i = 0; $i -lt 40; $i++) {
      $runId = gh run list --repo $repo --workflow "Release Windows" --limit 1 --json databaseId,headBranch,event --jq ".[] | select(.headBranch == `"$tag`" or .event == `"push`") | .databaseId"
      if ($runId) { break }
      Start-Sleep -Seconds 5
    }
    if ($runId) {
      gh run watch $runId --repo $repo --exit-status
      Write-Host ""
      Write-Host "Release 完成，请到这里下载安装包：" -ForegroundColor Green
      Write-Host "https://github.com/$repo/releases/tag/$tag"
    } else {
      Write-Host "没有等到 Actions run，但 tag 已推送。请打开 Actions 页面查看。" -ForegroundColor Yellow
    }
  } elseif (-not $NoWait) {
    Write-Host ""
    Write-Host "本机未安装 GitHub CLI，已跳过等待；Release 仍会在 GitHub 云端自动生成。" -ForegroundColor Yellow
  }
} finally {
  Pop-Location
}
