param(
  [Parameter(Mandatory = $true)]
  [string]$Version,
  [string]$NotesFile = "",
  [switch]$PushTag,
  [switch]$CreateRelease,
  [switch]$UploadAssets
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Version = $Version.Trim()
if ($Version -notmatch '^v\d+\.\d+\.\d+([-.+][0-9A-Za-z.-]+)?$') {
  throw "Version must look like v0.3.0, current: $Version"
}

$ReleaseDir = Join-Path $Root "release-assets"
$PortableDir = Join-Path $ReleaseDir "portable"
$ExeSource = Join-Path $Root "src-tauri\target\release\ai-report-factory.exe"
$ExeAsset = Join-Path $ReleaseDir "AI-Report-Factory-windows-x64.exe"
$SetupAsset = Join-Path $ReleaseDir "AI-Report-Factory-windows-x64-setup.exe"
$PortableExe = Join-Path $PortableDir "AI-Report-Factory-windows-x64.exe"
$PortableFfmpegDir = Join-Path $PortableDir "tools\ffmpeg"
$PortableFfmpeg = Join-Path $PortableFfmpegDir "ffmpeg.exe"
$ZipAsset = Join-Path $ReleaseDir "AI-Report-Factory-windows-x64-portable.zip"
$ShaAsset = Join-Path $ReleaseDir "SHA256SUMS.txt"
$NotesAsset = Join-Path $ReleaseDir "RELEASE_NOTES.md"
$Repo = "Luvroxanne/ai-report-factory"

function Step([string]$Text) {
  Write-Host ""
  Write-Host "==> $Text" -ForegroundColor Cyan
}

function Invoke-Checked([string]$File, [string[]]$CommandArgs, [string]$ErrorMessage) {
  & $File @CommandArgs
  if ($LASTEXITCODE -ne 0) {
    throw "$ErrorMessage ExitCode=$LASTEXITCODE"
  }
}

function Test-CommandExists([string]$Name) {
  return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-ReleaseNotes([string]$Version, [string]$ExplicitNotesFile) {
  if ($ExplicitNotesFile) {
    $Resolved = Resolve-Path $ExplicitNotesFile
    return Get-Content $Resolved -Raw
  }

  $Changelog = Join-Path $Root "CHANGELOG.md"
  if (Test-Path $Changelog) {
    $Text = Get-Content $Changelog -Raw
    $Escaped = [Regex]::Escape($Version)
    $Pattern = "(?ms)^##\s+$Escaped\s*(.*?)(?=^##\s+v|\z)"
    $Match = [Regex]::Match($Text, $Pattern)
    if ($Match.Success -and $Match.Groups[1].Value.Trim()) {
      return "# AI Report Factory $Version`n`n" + $Match.Groups[1].Value.Trim()
    }
    return "# AI Report Factory $Version`n`n" + $Text.Trim()
  }

  return "# AI Report Factory $Version`n`n- Windows portable exe build.`n"
}

function Assert-GhReady() {
  if (-not (Test-CommandExists "gh")) {
    throw "GitHub CLI 未安装。请安装 gh CLI 后重试：https://cli.github.com/"
  }
  gh auth status 1>$null
  if ($LASTEXITCODE -ne 0) {
    throw "GitHub CLI 未登录。请先执行：gh auth login"
  }
}

function Install-PortableFfmpeg {
  param([string]$TargetExe)
  if (Test-Path $TargetExe) {
    Write-Host "Using existing portable ffmpeg: $TargetExe"
    return
  }

  $PreparedFfmpeg = Join-Path $Root "resources\tools\ffmpeg\ffmpeg.exe"
  if (Test-Path $PreparedFfmpeg) {
    New-Item -ItemType Directory -Force -Path (Split-Path $TargetExe -Parent) | Out-Null
    Copy-Item -LiteralPath $PreparedFfmpeg -Destination $TargetExe -Force
    Copy-Item -LiteralPath (Join-Path $Root "resources\tools\ffmpeg\README-FFMPEG.txt") -Destination (Join-Path (Split-Path $TargetExe -Parent) "README-FFMPEG.txt") -Force
    Write-Host "Using prepared bundled ffmpeg: $PreparedFfmpeg"
    return
  }

  $ffmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
  $workDir = Join-Path ([IO.Path]::GetTempPath()) ("ai-report-factory-ffmpeg-" + [Guid]::NewGuid().ToString("N"))
  $download = Join-Path $workDir "ffmpeg-win64-lgpl.zip"
  $extract = Join-Path $workDir "extract"
  New-Item -ItemType Directory -Force -Path (Split-Path $TargetExe -Parent) | Out-Null
  Write-Host "Downloading FFmpeg LGPL build: $ffmpegUrl"
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
"@ | Set-Content -Encoding UTF8 -Path (Join-Path (Split-Path $TargetExe -Parent) "README-FFMPEG.txt")
}

Push-Location $Root
try {
  Step "Local checks"
  $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  Invoke-Checked "npm.cmd" @("--prefix", "frontend", "run", "type-check") "frontend type-check failed."
  Invoke-Checked "cargo" @("check", "--manifest-path", "src-tauri\Cargo.toml") "cargo check failed."

  Step "Prepare bundled FFmpeg"
  Invoke-Checked "powershell.exe" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts\prepare-ffmpeg.ps1") "prepare ffmpeg failed."

  Step "Build portable exe"
  Invoke-Checked "npm.cmd" @("--prefix", "frontend", "run", "desktop:build") "Tauri portable build failed."
  if (-not (Test-Path $ExeSource)) {
    throw "portable exe not found: $ExeSource"
  }

  Step "Build all-in-one installer"
  Invoke-Checked "npm.cmd" @("--prefix", "frontend", "run", "desktop:installer") "Tauri installer build failed."
  $BuiltInstaller = Get-ChildItem -Path "src-tauri\target\release\bundle\nsis" -Filter "*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if (-not $BuiltInstaller) {
    throw "NSIS installer not found under src-tauri\target\release\bundle\nsis"
  }

  Step "Prepare release assets"
  New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null
  New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null
  New-Item -ItemType Directory -Force -Path $PortableFfmpegDir | Out-Null
  Remove-Item -LiteralPath $ExeAsset -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $SetupAsset -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $ZipAsset -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $ShaAsset -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $NotesAsset -Force -ErrorAction SilentlyContinue

  Copy-Item -LiteralPath $ExeSource -Destination $ExeAsset -Force
  Copy-Item -LiteralPath $BuiltInstaller.FullName -Destination $SetupAsset -Force
  Copy-Item -LiteralPath $ExeSource -Destination $PortableExe -Force
  Install-PortableFfmpeg -TargetExe $PortableFfmpeg
  Get-ReleaseNotes -Version $Version -ExplicitNotesFile $NotesFile |
    Set-Content -Encoding UTF8 -Path $NotesAsset
  Copy-Item -LiteralPath $NotesAsset -Destination (Join-Path $PortableDir "RELEASE_NOTES.md") -Force
  Compress-Archive -Path (Join-Path $PortableDir "*") -DestinationPath $ZipAsset -Force
  Get-FileHash -LiteralPath $ExeAsset, $SetupAsset, $ZipAsset, $NotesAsset -Algorithm SHA256 |
    ForEach-Object { "$($_.Hash)  $([IO.Path]::GetFileName($_.Path))" } |
    Set-Content -Encoding UTF8 -Path $ShaAsset

  if ($PushTag) {
    Step "Create local git tag when needed"
    $ExistingTag = git tag --list $Version
    if (-not $ExistingTag) {
      Invoke-Checked "git" @("tag", $Version) "git tag failed."
      Write-Host "Created local tag: $Version"
    } else {
      Write-Host "Local tag already exists: $Version"
    }

    Step "Push tag"
    Invoke-Checked "git" @("push", "origin", $Version) "git push tag failed."
  }

  $ReleaseUrl = "https://github.com/$Repo/releases/tag/$Version"
  if ($CreateRelease -or $UploadAssets) {
    Step "GitHub Release"
    Assert-GhReady
    gh release view $Version --repo $Repo 1>$null 2>$null
    if ($LASTEXITCODE -eq 0) {
      Write-Host "Release already exists: $ReleaseUrl"
    } else {
      Invoke-Checked "gh" @("release", "create", $Version, "--repo", $Repo, "--title", "AI Report Factory $Version", "--notes-file", $NotesAsset) "gh release create failed."
    }

    if ($UploadAssets) {
      Invoke-Checked "gh" @("release", "upload", $Version, $ExeAsset, $SetupAsset, $ZipAsset, $ShaAsset, $NotesAsset, "--repo", $Repo, "--clobber") "gh release upload failed."
    }
  }

  Step "Done"
  Write-Host "exe: $ExeAsset" -ForegroundColor Green
  Write-Host "installer: $SetupAsset" -ForegroundColor Green
  Write-Host "zip: $ZipAsset" -ForegroundColor Green
  Write-Host "sha256: $ShaAsset" -ForegroundColor Green
  Write-Host "notes: $NotesAsset" -ForegroundColor Green
  Write-Host "release: $ReleaseUrl" -ForegroundColor Green
  Write-Host ""
  if (-not $PushTag) {
    Write-Host "下一步：如需触发 GitHub Actions，请执行 git push origin $Version"
  } elseif (-not $CreateRelease) {
    Write-Host "下一步：GitHub Actions 会基于 v* tag 自动构建并发布 Release。"
  } else {
    Write-Host "下一步：打开 Release 页面核对资产与说明。"
  }
} finally {
  Pop-Location
}
