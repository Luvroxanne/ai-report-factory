$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

Push-Location $Root
try {
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  cargo test --manifest-path src-tauri\Cargo.toml --test local_generation -- --nocapture
} finally {
  Pop-Location
}
