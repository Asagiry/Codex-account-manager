import { useCallback, useEffect, useState, type MouseEvent } from 'react'
import { Minus, MoonStar, Network, Sun, Users, X } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getVersion } from '@tauri-apps/api/app'
import './App.css'
import { api } from './api'
import type { AppData } from './types'
import { AccountsTab } from './components/AccountsTab'
import { ProxyTab } from './components/ProxyTab'

type TabKey = 'accounts' | 'proxy'
type ThemeMode = 'light' | 'dark'

const appWindow = getCurrentWindow()
const THEME_STORAGE_KEY = 'codex_account_manager_theme'

function readInitialTheme(): ThemeMode {
  if (typeof window === 'undefined') return 'light'
  const saved = window.localStorage.getItem(THEME_STORAGE_KEY)
  return saved === 'dark' ? 'dark' : 'light'
}

function isInteractiveTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  return Boolean(
    target.closest(
      'button, input, textarea, select, a, [role="button"], [contenteditable="true"], [data-no-drag], .allow-select'
    )
  )
}

function App() {
  const [activeTab, setActiveTab] = useState<TabKey>('accounts')
  const [data, setData] = useState<AppData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [theme, setTheme] = useState<ThemeMode>(readInitialTheme)
  const [version, setVersion] = useState<string>('0.2.0')

  const load = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const state = await api.getState()
      setData(state)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    window.localStorage.setItem(THEME_STORAGE_KEY, theme)
  }, [theme])

  useEffect(() => {
    void getVersion()
      .then((v) => setVersion(v))
      .catch(() => setVersion('0.2.0'))
  }, [])

  const handleShellMouseDown = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return
    if (isInteractiveTarget(event.target)) return
    event.preventDefault()
    void appWindow.startDragging()
  }

  const minimizeWindow = async () => {
    await appWindow.minimize()
  }

  const closeWindow = async () => {
    await appWindow.close()
  }

  const toggleTheme = () => {
    setTheme((prev) => (prev === 'light' ? 'dark' : 'light'))
  }

  return (
    <div className="app-outer h-full w-full text-ag-text">
      <div className="app-shell h-full w-full flex flex-col bg-ag-bg" onMouseDown={handleShellMouseDown}>
        <header className="h-24 px-6 border-b border-ag-border bg-ag-card/90 backdrop-blur-md sticky top-0 z-20">
          <div className="h-full max-w-[1440px] mx-auto flex items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <div className="text-3xl font-bold tracking-tight bg-gradient-to-r from-blue-700 via-blue-600 to-cyan-500 bg-clip-text text-transparent">
                Codex Account Manager
              </div>
              <span className="h-7 px-3 rounded-full border border-ag-border bg-ag-surface inline-flex items-center text-xs font-semibold text-ag-muted">
                v{version}
              </span>
            </div>

            <div className="flex items-center gap-2 rounded-2xl border border-ag-border bg-ag-card p-1.5 shadow-soft" data-no-drag>
              <button
                className={`h-9 px-4 rounded-xl text-sm font-semibold inline-flex items-center gap-2 ${
                  activeTab === 'accounts' ? 'bg-ag-primary text-white' : 'text-ag-text hover:bg-ag-surface'
                }`}
                onClick={() => setActiveTab('accounts')}
              >
                <Users size={15} /> Accounts
              </button>

              <button
                className={`h-9 px-4 rounded-xl text-sm font-semibold inline-flex items-center gap-2 ${
                  activeTab === 'proxy' ? 'bg-ag-primary text-white' : 'text-ag-text hover:bg-ag-surface'
                }`}
                onClick={() => setActiveTab('proxy')}
              >
                <Network size={15} /> Proxy
              </button>
            </div>

            <div className="flex items-center gap-2" data-no-drag>
              <button
                className="theme-toggle"
                onClick={toggleTheme}
                title={theme === 'light' ? 'Switch to dark theme' : 'Switch to light theme'}
              >
                <span className={`theme-chip ${theme === 'dark' ? 'theme-chip-right' : ''}`} />
                <span className="theme-side">
                  <Sun size={14} />
                  Light
                </span>
                <span className="theme-side">
                  <MoonStar size={14} />
                  Dark
                </span>
              </button>

              <button className="window-btn" onClick={() => void minimizeWindow()} title="Hide" aria-label="Hide">
                <Minus size={14} />
              </button>
              <button className="window-btn window-btn-close" onClick={() => void closeWindow()} title="Close" aria-label="Close">
                <X size={14} />
              </button>
            </div>
          </div>
        </header>

        <main className="flex-1 min-h-0 overflow-hidden px-6 py-5">
          <div className="max-w-[1440px] mx-auto h-full">
            {loading && (
              <div className="h-full rounded-2xl border border-ag-border bg-ag-card shadow-ag flex items-center justify-center text-ag-muted">
                Loading...
              </div>
            )}

            {!loading && error && (
              <div className="h-full rounded-2xl border border-red-200 bg-red-50 shadow-ag p-6 text-red-700">
                Error: {error}
              </div>
            )}

            {!loading && !error && data && (
              <>
                {activeTab === 'accounts' && <AccountsTab data={data} setData={setData} reload={load} />}
                {activeTab === 'proxy' && <ProxyTab data={data} setData={setData} />}
              </>
            )}
          </div>
        </main>
      </div>
    </div>
  )
}

export default App



