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
  python scripts\bump_version.py $version
  npm.cmd --prefix frontend run type-check
  python -m compileall backend\app
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  cargo check --manifest-path src-tauri\Cargo.toml

  git add .
  git commit -m "chore: release $tag"
  git tag $tag
  git push origin main
  git push origin $tag
} finally {
  Pop-Location
}
