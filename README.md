# Codex Account Manager

Codex Account Manager is a Windows desktop app for managing multiple Codex/OpenAI accounts with OAuth login, quota tracking, proxy support, and quick IDE account switching.

## Version

Current app version: `0.2.0`

## Key Features

- OAuth login flow (manual login URL copy + callback paste + callback auto-detection).
- Multi-account management (accounts are added as separate rows, not replaced).
- Quota tracking for 5-hour and weekly windows.
- Quota bars show **remaining** quota (`100 - used`).
- Auto refresh for quotas every 5 minutes.
- Manual refresh controls for one account or all accounts.
- Proxy management (`login:pass@ip:port`) with connectivity test.
- IDE-aware account switching with automatic reload/restart attempt.
- Light/Dark theme toggle.
- Local-only state storage on your machine.

## Platform

- Windows only.

## Tech Stack

- Rust
- Tauri v2
- React + TypeScript
- Tailwind CSS

## Local Data

Application state:

- `%LOCALAPPDATA%\CodexAccountManager\state.json`

Codex auth file used during account switch:

- `%USERPROFILE%\.codex\auth.json`

## Requirements

- Node.js LTS
- Rust toolchain
- Microsoft Edge WebView2 Runtime

## Development

Install dependencies:

```powershell
npm ci
npm --prefix ui ci
```

Run dev mode:

```powershell
npm run dev
```

## Build

Build bare executable (no installer):

```powershell
npm run build:win
```

Outputs:

- `dist/release/codex-account-manager.exe`
- `dist/release/WebView2Loader.dll`

Build release artifacts (setup + portable):

```powershell
npm run build:release
```

Outputs:

- `dist/release/codex_account_manager_v<version>_setup_x64.exe`
- `dist/release/codex_account_manager_v<version>_portable.zip`
- `dist/release/codex-account-manager.exe`
- `dist/release/WebView2Loader.dll`

## CI/CD and Releases

Workflow file:

- `.github/workflows/release.yml`

Behavior:

- Push to `main` builds artifacts and updates rolling pre-release tag `main-latest`.
- Push tag `v*` builds artifacts and publishes a versioned stable GitHub Release.
