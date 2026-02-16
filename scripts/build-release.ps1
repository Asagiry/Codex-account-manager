$ErrorActionPreference = 'Stop'

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Command,
    [string]$ErrorMessage
  )

  Invoke-Expression $Command
  if ($LASTEXITCODE -ne 0) {
    if ([string]::IsNullOrWhiteSpace($ErrorMessage)) {
      throw "Command failed: $Command"
    }
    throw $ErrorMessage
  }
}

$targetDir = 'C:\Temp\codex-account-manager-target'
$repoRoot = Resolve-Path "$PSScriptRoot\.."
$distRoot = Join-Path $repoRoot 'dist\release'

$exeSource = Join-Path $targetDir 'release\codex-account-manager.exe'
$dllSource = Join-Path $targetDir 'release\WebView2Loader.dll'
$nsisBundleDir = Join-Path $targetDir 'release\bundle\nsis'
$tauriConfigPath = Join-Path $repoRoot 'src-tauri\tauri.conf.json'

Write-Host "[build-release] Using CARGO_TARGET_DIR=$targetDir"
$env:CARGO_TARGET_DIR = $targetDir

Push-Location $repoRoot
try {
  New-Item -ItemType Directory -Force -Path $distRoot | Out-Null

  # Best-effort cleanup of old artifacts; ignore if locked.
  $cleanupItems = @(
    (Join-Path $distRoot '*_setup_x64.exe'),
    (Join-Path $distRoot '*_portable.zip'),
    (Join-Path $distRoot 'codex-account-manager.exe'),
    (Join-Path $distRoot 'WebView2Loader.dll'),
    (Join-Path $distRoot 'portable')
  )

  foreach ($item in $cleanupItems) {
    try {
      Remove-Item -Path $item -Recurse -Force -ErrorAction SilentlyContinue
    }
    catch {
      Write-Warning "[build-release] Could not remove old artifact: $item"
    }
  }

  Write-Host '[build-release] Building UI...'
  Invoke-Checked -Command 'npm --prefix ui run build' -ErrorMessage 'UI build failed'

  Write-Host '[build-release] Building NSIS setup...'
  Invoke-Checked -Command 'npm run tauri -- build --bundles nsis' -ErrorMessage 'Tauri NSIS build failed'

  if (!(Test-Path $exeSource)) {
    throw "Built executable not found at $exeSource"
  }

  if (!(Test-Path $dllSource)) {
    throw "WebView2Loader.dll not found at $dllSource"
  }

  $tauriConfig = Get-Content -Raw $tauriConfigPath | ConvertFrom-Json
  $version = $tauriConfig.version

  # Stage portable package in temporary folder, then zip into dist/release.
  $portableStage = Join-Path $env:TEMP ('codex-account-manager-portable-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Force -Path $portableStage | Out-Null

  $portableExe = Join-Path $portableStage 'codex-account-manager.exe'
  $portableDll = Join-Path $portableStage 'WebView2Loader.dll'

  Copy-Item -Force $exeSource $portableExe
  Copy-Item -Force $dllSource $portableDll

  $portableReadme = @"
Codex Account Manager (Portable)

Requirements:
- Microsoft Edge WebView2 Runtime installed on Windows.

Run:
- Keep codex-account-manager.exe and WebView2Loader.dll in the same folder.
- Start codex-account-manager.exe.
"@
  [System.IO.File]::WriteAllText((Join-Path $portableStage 'README_PORTABLE.txt'), $portableReadme, (New-Object System.Text.UTF8Encoding($false)))

  $zipName = "codex_account_manager_v$version`_portable.zip"
  $zipPath = Join-Path $distRoot $zipName
  if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
  }
  Compress-Archive -Path (Join-Path $portableStage '*') -DestinationPath $zipPath -Force

  Remove-Item -Path $portableStage -Recurse -Force -ErrorAction SilentlyContinue

  $setupSource = Get-ChildItem -Path $nsisBundleDir -Filter *.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if (-not $setupSource) {
    throw "NSIS setup not found in $nsisBundleDir"
  }

  $setupName = "codex_account_manager_v$version`_setup_x64.exe"
  $setupPath = Join-Path $distRoot $setupName
  Copy-Item -Force $setupSource.FullName $setupPath

  $exeDest = Join-Path $distRoot 'codex-account-manager.exe'
  $dllDest = Join-Path $distRoot 'WebView2Loader.dll'

  try {
    Copy-Item -Force $exeSource $exeDest
  }
  catch {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $fallbackExe = Join-Path $distRoot "codex-account-manager-$stamp.exe"
    Copy-Item -Force $exeSource $fallbackExe
    Write-Warning "[build-release] Could not overwrite $exeDest (probably locked)."
    Write-Host "[build-release] Copied executable to fallback: $fallbackExe"
  }

  try {
    Copy-Item -Force $dllSource $dllDest
  }
  catch {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $fallbackDll = Join-Path $distRoot "WebView2Loader-$stamp.dll"
    Copy-Item -Force $dllSource $fallbackDll
    Write-Warning "[build-release] Could not overwrite $dllDest (probably locked)."
    Write-Host "[build-release] Copied loader dll to fallback: $fallbackDll"
  }

  Write-Host "[build-release] Setup:    $setupPath"
  Write-Host "[build-release] Portable: $zipPath"
  Write-Host "[build-release] Bare exe: $exeDest"
  Write-Host "[build-release] Bare dll: $dllDest"
}
finally {
  Pop-Location
}