param(
  [string]$TargetDir = ".\resources\tools\ffmpeg"
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$TargetDir = Join-Path $Root $TargetDir
$TargetExe = Join-Path $TargetDir "ffmpeg.exe"

if (Test-Path $TargetExe) {
  Write-Host "ffmpeg already exists: $TargetExe" -ForegroundColor Green
  exit 0
}

New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
$ffmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
$workDir = Join-Path ([IO.Path]::GetTempPath()) ("ai-report-factory-ffmpeg-" + [Guid]::NewGuid().ToString("N"))
$download = Join-Path $workDir "ffmpeg-win64-lgpl.zip"
$extract = Join-Path $workDir "extract"

Write-Host "Downloading FFmpeg LGPL build:" -ForegroundColor Cyan
Write-Host $ffmpegUrl
try {
  New-Item -ItemType Directory -Force -Path $workDir | Out-Null
  Invoke-WebRequest -Uri $ffmpegUrl -OutFile $download
  Expand-Archive -Path $download -DestinationPath $extract -Force
  $ffmpegExe = Get-ChildItem $extract -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1
  if (-not $ffmpegExe) {
    throw "ffmpeg.exe not found in downloaded FFmpeg archive."
  }

  Copy-Item -LiteralPath $ffmpegExe.FullName -Destination $TargetExe -Force
} finally {
  Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
@"
FFmpeg is bundled from BtbN/FFmpeg-Builds latest LGPL Windows build:
$ffmpegUrl

FFmpeg is an optional local video composition tool used by AI Report Factory.
See https://ffmpeg.org/legal.html and the FFmpeg build package for license details.
"@ | Set-Content -Encoding UTF8 -Path (Join-Path $TargetDir "README-FFMPEG.txt")

Write-Host "ffmpeg prepared: $TargetExe" -ForegroundColor Green
