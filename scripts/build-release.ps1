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

function Resolve-WebView2Loader {
  param(
    [Parameter(Mandatory = $true)]
    [string]$PreferredPath,
    [Parameter(Mandatory = $true)]
    [string]$SearchRoot
  )

  if (Test-Path $PreferredPath) {
    return $PreferredPath
  }

  $found = Get-ChildItem -Path $SearchRoot -Recurse -Filter 'WebView2Loader.dll' -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($found) {
    return $found.FullName
  }

  # CI fallback: pull dll from official WebView2 NuGet package.
  $nugetVersion = '1.0.2903.40'
  $tmpRoot = Join-Path $env:TEMP ('webview2loader-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Force -Path $tmpRoot | Out-Null

  try {
    $nupkgPath = Join-Path $tmpRoot 'webview2.nupkg'
    $extractDir = Join-Path $tmpRoot 'pkg'
    $url = "https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/$nugetVersion"

    Write-Host "[build-release] Downloading WebView2Loader from NuGet: $url"
    Invoke-WebRequest -Uri $url -OutFile $nupkgPath -UseBasicParsing
    Expand-Archive -Path $nupkgPath -DestinationPath $extractDir -Force

    $candidate = Join-Path $extractDir 'build\native\x64\WebView2Loader.dll'
    if (Test-Path $candidate) {
      return $candidate
    }

    $candidateAny = Get-ChildItem -Path $extractDir -Recurse -Filter 'WebView2Loader.dll' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($candidateAny) {
      return $candidateAny.FullName
    }

    return $null
  }
  catch {
    Write-Warning "[build-release] Failed to download WebView2Loader fallback: $($_.Exception.Message)"
    return $null
  }
  finally {
    # Keep temp folder only for current script lifetime; caller copies file immediately.
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
    (Join-Path $distRoot 'WebView2Loader.dll')
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

  $resolvedDll = Resolve-WebView2Loader -PreferredPath $dllSource -SearchRoot (Join-Path $targetDir 'release')
  $hasDll = -not [string]::IsNullOrWhiteSpace($resolvedDll)

  if ($hasDll) {
    $dllSource = $resolvedDll
    Write-Host "[build-release] Using WebView2Loader.dll source: $dllSource"
  } else {
    Write-Warning "[build-release] WebView2Loader.dll not found; continuing without bare dll artifact."
  }

  $tauriConfig = Get-Content -Raw $tauriConfigPath | ConvertFrom-Json
  $version = $tauriConfig.version

  # Stage portable package in temporary folder, then zip into dist/release.
  $portableStage = Join-Path $env:TEMP ('codex-account-manager-portable-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Force -Path $portableStage | Out-Null

  $portableExe = Join-Path $portableStage 'codex-account-manager.exe'
  Copy-Item -Force $exeSource $portableExe

  if ($hasDll) {
    $portableDll = Join-Path $portableStage 'WebView2Loader.dll'
    Copy-Item -Force $dllSource $portableDll
  }

  $portableReadme = @"
Codex Account Manager (Portable)

Requirements:
- Microsoft Edge WebView2 Runtime installed on Windows.

Run:
- Start codex-account-manager.exe.
- If WebView2Loader.dll is present, keep it in the same folder as the executable.
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
  Copy-Item -Force $exeSource $exeDest

  if ($hasDll) {
    $dllDest = Join-Path $distRoot 'WebView2Loader.dll'
    Copy-Item -Force $dllSource $dllDest
    Write-Host "[build-release] Bare dll: $dllDest"
  }

  Write-Host "[build-release] Setup:    $setupPath"
  Write-Host "[build-release] Portable: $zipPath"
  Write-Host "[build-release] Bare exe: $exeDest"
}
finally {
  Pop-Location
}
