$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$venvPython = Join-Path $root ".venv\Scripts\python.exe"
$python = if (Test-Path $venvPython) { $venvPython } else { "python" }

Push-Location $root
try {
  & $python -m PyInstaller `
    --clean `
    --console `
    --onefile `
    --name ai-report-backend `
    --distpath backend `
    --workpath backend\build `
    --specpath backend\build `
    --paths backend `
    backend\desktop_backend.py
} finally {
  Pop-Location
}
