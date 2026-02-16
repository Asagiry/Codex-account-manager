import { useState } from 'react'
import { CheckCircle2, Loader2, Plus, RefreshCw, Trash2, WifiOff } from 'lucide-react'
import { api } from '../api'
import type { AppData, ProxyEntry } from '../types'

type ProxyTabProps = {
  data: AppData
  setData: (next: AppData) => void
}

export function ProxyTab({ data, setData }: ProxyTabProps) {
  const [proxyInput, setProxyInput] = useState('')
  const [busyId, setBusyId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const addProxy = async () => {
    if (!proxyInput.trim()) return

    try {
      setError(null)
      const next = await api.saveProxy(proxyInput.trim())
      setData(next)
      setProxyInput('')
    } catch (err) {
      setError(String(err))
    }
  }

  const activate = async (proxyId?: string) => {
    try {
      setBusyId(proxyId ?? 'disable')
      setError(null)
      const next = await api.setActiveProxy(proxyId)
      setData(next)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyId(null)
    }
  }

  const runTest = async (proxy: ProxyEntry) => {
    try {
      setBusyId(proxy.id)
      setError(null)
      await api.testProxy(proxy.id)
      const next = await api.getState()
      setData(next)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyId(null)
    }
  }

  const remove = async (proxyId: string) => {
    try {
      setBusyId(proxyId)
      setError(null)
      const next = await api.deleteProxy(proxyId)
      setData(next)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyId(null)
    }
  }

  return (
    <div className="page-fade h-full flex flex-col gap-4">
      <div className="rounded-2xl border border-ag-border bg-white shadow-ag p-4">
        <div className="text-sm font-semibold text-ag-text mb-2">Add proxy</div>
        <div className="text-xs text-ag-muted mb-3">
          Required format: <span className="font-semibold">login:pass@ip:port</span>
        </div>

        <div className="flex items-center gap-2">
          <input
            className="flex-1 h-10 rounded-xl border border-ag-border px-3 text-sm outline-none focus:border-blue-500"
            placeholder="example: user123:pass123@1.2.3.4:8080"
            value={proxyInput}
            onChange={(event) => setProxyInput(event.target.value)}
          />
          <button
            className="h-10 px-4 rounded-xl bg-ag-primary text-white text-sm font-semibold hover:bg-blue-700 inline-flex items-center gap-2"
            onClick={addProxy}
          >
            <Plus size={16} /> Save
          </button>
          <button
            className="h-10 px-4 rounded-xl border border-ag-border text-sm font-semibold text-ag-text hover:bg-slate-50"
            onClick={() => activate(undefined)}
            disabled={busyId === 'disable'}
          >
            Disable proxy
          </button>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</div>
      )}

      <div className="flex-1 min-h-0 rounded-2xl border border-ag-border bg-white shadow-ag overflow-hidden">
        <div className="h-full overflow-auto">
          <table className="w-full border-collapse text-sm">
            <thead className="sticky top-0 bg-slate-50 z-10">
              <tr className="text-left text-xs uppercase tracking-wide text-ag-muted border-b border-ag-border">
                <th className="px-4 py-3">Proxy</th>
                <th className="px-4 py-3">Status</th>
                <th className="px-4 py-3">Ping</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {data.proxies.length === 0 && (
                <tr>
                  <td colSpan={4} className="px-4 py-12 text-center text-ag-muted">
                    No proxies yet.
                  </td>
                </tr>
              )}

              {data.proxies.map((proxy) => {
                const active = data.activeProxyId === proxy.id
                const loading = busyId === proxy.id

                return (
                  <tr key={proxy.id} className={`border-b border-ag-border/70 ${active ? 'bg-blue-50/40' : ''}`}>
                    <td className="px-4 py-3 align-top">
                      <div className="font-semibold text-ag-text">{proxy.raw}</div>
                      <div className="text-xs text-ag-muted mt-1">
                        login: {proxy.login} / host: {proxy.host}:{proxy.port}
                      </div>
                    </td>
                    <td className="px-4 py-3 align-top">
                      {proxy.lastStatus === 'ok' ? (
                        <span className="inline-flex items-center gap-1 text-emerald-600 text-xs">
                          <CheckCircle2 size={14} /> healthy
                        </span>
                      ) : proxy.lastStatus === 'error' ? (
                        <span className="inline-flex items-center gap-1 text-red-600 text-xs">
                          <WifiOff size={14} /> error
                        </span>
                      ) : (
                        <span className="text-xs text-ag-muted">not tested</span>
                      )}
                    </td>
                    <td className="px-4 py-3 align-top">
                      {proxy.lastLatencyMs != null ? `${proxy.lastLatencyMs} ms` : '-'}
                    </td>
                    <td className="px-4 py-3 align-top">
                      <div className="flex justify-end gap-2">
                        <button
                          className={`h-8 px-3 rounded-lg text-xs font-semibold border ${
                            active
                              ? 'border-emerald-200 bg-emerald-50 text-emerald-700'
                              : 'border-ag-border text-ag-text hover:bg-slate-50'
                          }`}
                          onClick={() => activate(proxy.id)}
                          disabled={loading}
                        >
                          {active ? 'Active' : 'Set active'}
                        </button>

                        <button
                          className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-ag-border text-ag-muted hover:text-ag-text hover:bg-slate-50"
                          onClick={() => runTest(proxy)}
                          disabled={loading}
                          title="Test proxy"
                        >
                          {loading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
                        </button>

                        <button
                          className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-red-200 text-red-600 hover:bg-red-50"
                          onClick={() => remove(proxy.id)}
                          disabled={loading}
                          title="Delete proxy"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}