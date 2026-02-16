$ErrorActionPreference = "Stop"

$targetDir = "C:\Temp\codex-account-manager-target"
$repoRoot = Resolve-Path "$PSScriptRoot\.."
$exeSource = Join-Path $targetDir "release\codex-account-manager.exe"
$dllSource = Join-Path $targetDir "release\WebView2Loader.dll"

$legacyDestDir = Join-Path $repoRoot "src-tauri\target\release"
$artifactDir = Join-Path $repoRoot "dist\release"

$legacyExe = Join-Path $legacyDestDir "codex-account-manager.exe"
$legacyDll = Join-Path $legacyDestDir "WebView2Loader.dll"
$artifactExe = Join-Path $artifactDir "codex-account-manager.exe"
$artifactDll = Join-Path $artifactDir "WebView2Loader.dll"

Write-Host "[build-win] Using CARGO_TARGET_DIR=$targetDir"
$env:CARGO_TARGET_DIR = $targetDir

Push-Location $repoRoot
try {
  npm run build

  if (!(Test-Path $exeSource)) {
    throw "Built executable not found at $exeSource"
  }
  if (!(Test-Path $dllSource)) {
    throw "WebView2Loader.dll not found at $dllSource"
  }

  New-Item -ItemType Directory -Force $artifactDir | Out-Null

  try {
    Copy-Item -Force $exeSource $artifactExe
    Write-Host "[build-win] Copied executable to: $artifactExe"
  }
  catch {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $fallbackExe = Join-Path $artifactDir "codex-account-manager-$stamp.exe"
    Copy-Item -Force $exeSource $fallbackExe
    Write-Warning "Could not overwrite $artifactExe (probably locked by running app)."
    Write-Host "[build-win] Copied executable to fallback: $fallbackExe"
  }

  try {
    Copy-Item -Force $dllSource $artifactDll
    Write-Host "[build-win] Copied loader dll to: $artifactDll"
  }
  catch {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $fallbackDll = Join-Path $artifactDir "WebView2Loader-$stamp.dll"
    Copy-Item -Force $dllSource $fallbackDll
    Write-Warning "Could not overwrite $artifactDll (probably locked by running app)."
    Write-Host "[build-win] Copied loader dll to fallback: $fallbackDll"
  }

  New-Item -ItemType Directory -Force $legacyDestDir | Out-Null
  try {
    Copy-Item -Force $exeSource $legacyExe
    Copy-Item -Force $dllSource $legacyDll
    Write-Host "[build-win] Synced legacy target dir: $legacyDestDir"
  }
  catch {
    Write-Warning "Could not overwrite legacy target dir (file may be locked by running app): $legacyDestDir"
    Write-Warning "Use artifacts from: $artifactDir"
  }
}
finally {
  Pop-Location
}