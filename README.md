# Codex Account Manager

Codex Account Manager is a Windows desktop application for managing multiple Codex/OpenAI accounts with OAuth login, quota monitoring, and proxy support.

## Key Features

- OAuth login flow (manual URL open + callback paste + local callback listener).
- Multi-account table with quick account switching.
- Quota tracking for 5-hour and weekly windows.
- Automatic quota refresh every 5 minutes.
- Manual per-account and bulk quota refresh.
- Proxy management (`login:pass@ip:port`) with connectivity test.
- IDE-aware account switch workflow with automatic IDE reload attempt.
- Local-only data storage on the machine.

## Platform

- Windows only.

## Tech Stack

- Rust
- Tauri v2
- React + TypeScript
- Tailwind CSS

## Local Data

Application state is stored at:

- `%LOCALAPPDATA%\CodexAccountManager\state.json`

Codex auth file used during account switch:

- `%USERPROFILE%\.codex\auth.json`

## Requirements

- Node.js LTS
- Rust toolchain
- WebView2 Runtime

Optional (GNU toolchain setup):

```powershell
scoop install mingw
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

## Development

Install dependencies:

```powershell
npm ci
npm --prefix ui ci
```

Run in development mode:

```powershell
npm run dev
```

## Build

Build bare executable (no installer bundle):

```powershell
npm run build:win
```

Expected output:

- `dist/release/codex-account-manager.exe`
- `dist/release/WebView2Loader.dll`

Build release artifacts (setup + portable zip):

```powershell
npm run build:release
```

Expected output:

- `dist/release/CodexAccountManager-v<version>-windows-setup.exe`
- `dist/release/CodexAccountManager-v<version>-windows-portable.zip`
- `dist/release/codex-account-manager.exe`
- `dist/release/WebView2Loader.dll`

## CI / Release

GitHub Actions workflow:

- `.github/workflows/release.yml`

Release is triggered by tags matching `v*`.

Example:

```powershell
git tag v0.1.0
git push origin v0.1.0
```