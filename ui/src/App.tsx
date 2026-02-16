import { useCallback, useEffect, useState } from 'react'
import { Minus, Network, RefreshCw, Users, X } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './App.css'
import { api } from './api'
import type { AppData } from './types'
import { AccountsTab } from './components/AccountsTab'
import { ProxyTab } from './components/ProxyTab'

type TabKey = 'accounts' | 'proxy'

const appWindow = getCurrentWindow()

function App() {
  const [activeTab, setActiveTab] = useState<TabKey>('accounts')
  const [data, setData] = useState<AppData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

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

  const minimizeWindow = async () => {
    await appWindow.minimize()
  }

  const closeWindow = async () => {
    await appWindow.close()
  }

  return (
    <div className="h-full w-full bg-ag-bg text-ag-text flex flex-col">
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-label" data-tauri-drag-region>
          Codex Account Manager
        </div>
        <div className="titlebar-controls">
          <button className="titlebar-btn" onClick={() => void minimizeWindow()} title="Minimize">
            <Minus size={14} />
          </button>
          <button className="titlebar-btn titlebar-btn-close" onClick={() => void closeWindow()} title="Close">
            <X size={14} />
          </button>
        </div>
      </div>

      <header className="h-24 px-6 border-b border-ag-border bg-white/90 backdrop-blur-md sticky top-0 z-20">
        <div className="h-full max-w-[1440px] mx-auto flex items-center justify-between gap-4">
          <div>
            <div className="text-3xl font-bold tracking-tight bg-gradient-to-r from-blue-700 via-blue-600 to-cyan-500 bg-clip-text text-transparent">
              Codex Account Manager
            </div>
          </div>

          <div className="flex items-center gap-2 rounded-xl border border-ag-border bg-white p-1 shadow-soft">
            <button
              className={`h-9 px-4 rounded-lg text-sm font-semibold inline-flex items-center gap-2 ${
                activeTab === 'accounts' ? 'bg-ag-primary text-white' : 'text-ag-text hover:bg-slate-50'
              }`}
              onClick={() => setActiveTab('accounts')}
            >
              <Users size={15} /> Accounts
            </button>

            <button
              className={`h-9 px-4 rounded-lg text-sm font-semibold inline-flex items-center gap-2 ${
                activeTab === 'proxy' ? 'bg-ag-primary text-white' : 'text-ag-text hover:bg-slate-50'
              }`}
              onClick={() => setActiveTab('proxy')}
            >
              <Network size={15} /> Proxy
            </button>
          </div>

          <button
            className="h-9 px-4 rounded-lg border border-ag-border text-sm font-semibold text-ag-text hover:bg-slate-50 inline-flex items-center gap-2"
            onClick={() => void load()}
            disabled={loading}
          >
            <RefreshCw size={15} className={loading ? 'animate-spin' : ''} />
            Reload
          </button>
        </div>
      </header>

      <main className="flex-1 min-h-0 overflow-hidden px-6 py-5">
        <div className="max-w-[1440px] mx-auto h-full">
          {loading && (
            <div className="h-full rounded-2xl border border-ag-border bg-white shadow-ag flex items-center justify-center text-ag-muted">
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
  )
}

export default App