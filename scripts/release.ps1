param(
  [Parameter(Mandatory = $true)]
  [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$version = $Version.TrimStart("v")
$tag = "v$version"

Push-Location $root
try {
  Write-Host "Release helper for $tag" -ForegroundColor Cyan
  Write-Host "本脚本只做本地检查，不会推送远程仓库。"
  npm.cmd --prefix frontend run type-check
  cargo check --manifest-path src-tauri\Cargo.toml
  Write-Host ""
  Write-Host "如需发布，请手动执行：" -ForegroundColor Green
  Write-Host "git tag $tag"
  Write-Host "git push origin $tag"
  Write-Host "推送 v* tag 后 GitHub Actions 会构建 Windows 便携式 exe 并发布 Release。"
} finally {
  Pop-Location
}
