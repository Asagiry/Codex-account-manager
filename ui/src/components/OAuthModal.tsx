import { useEffect, useMemo, useState } from 'react'
import { Copy, Link2, Loader2, X } from 'lucide-react'
import { api } from '../api'
import type { OAuthFlowResponse, OAuthStartResponse } from '../types'

type OAuthModalProps = {
  open: boolean
  onClose: () => void
  onCompleted: () => Promise<void>
}

function statusText(status: OAuthFlowResponse['status'] | 'idle'): string {
  if (status === 'waiting_callback') return 'Waiting for callback after login'
  if (status === 'exchanging') return 'Exchanging code for tokens'
  if (status === 'completed') return 'Completed'
  if (status === 'error') return 'Error'
  return 'Not started'
}

export function OAuthModal({ open, onClose, onCompleted }: OAuthModalProps) {
  const [startData, setStartData] = useState<OAuthStartResponse | null>(null)
  const [flow, setFlow] = useState<OAuthFlowResponse | null>(null)
  const [callbackInput, setCallbackInput] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const status = useMemo(() => flow?.status ?? 'idle', [flow?.status])

  useEffect(() => {
    if (!open) {
      setStartData(null)
      setFlow(null)
      setCallbackInput('')
      setBusy(false)
      setError(null)
      return
    }
  }, [open])

  useEffect(() => {
    if (!open || !startData?.flowId) return
    if (status !== 'waiting_callback' && status !== 'exchanging') return

    const timer = setInterval(async () => {
      try {
        const next = await api.getOAuthStatus(startData.flowId)
        setFlow(next)
        if (next.status === 'completed') {
          await onCompleted()
        }
      } catch (err) {
        setError(String(err))
      }
    }, 1500)

    return () => clearInterval(timer)
  }, [open, startData?.flowId, status, onCompleted])

  if (!open) return null

  const startOAuth = async () => {
    try {
      setBusy(true)
      setError(null)
      const started = await api.startOAuthFlow()
      setStartData(started)
      const firstStatus = await api.getOAuthStatus(started.flowId)
      setFlow(firstStatus)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusy(false)
    }
  }

  const completeManual = async () => {
    if (!startData?.flowId) return

    try {
      setBusy(true)
      setError(null)
      const next = await api.completeOAuthWithCallback(startData.flowId, callbackInput)
      setFlow(next)
      if (next.status === 'completed') {
        await onCompleted()
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setBusy(false)
    }
  }

  const copyLink = async () => {
    if (!startData?.authorizationUrl) return
    await navigator.clipboard.writeText(startData.authorizationUrl)
  }

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/40 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="w-full max-w-3xl rounded-2xl border border-ag-border bg-white shadow-ag">
        <div className="flex items-center justify-between px-5 py-4 border-b border-ag-border">
          <div>
            <h2 className="m-0 text-lg font-semibold text-ag-text">OAuth Login</h2>
            <p className="m-0 mt-1 text-sm text-ag-muted">
              The app does not open browser links automatically. Copy and open manually.
            </p>
          </div>
          <button
            className="h-9 w-9 inline-flex items-center justify-center rounded-lg border border-ag-border text-ag-muted hover:text-ag-text hover:bg-slate-50"
            onClick={onClose}
          >
            <X size={16} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div className="rounded-xl border border-ag-border bg-slate-50 px-4 py-3 text-sm text-ag-muted">
            0. If your region is blocked, activate proxy in the Proxy tab first.<br />
            1. Click "Generate login URL".<br />
            2. Copy the URL and open it in browser.<br />
            3. After login, paste full callback URL (or query) below.
          </div>

          {!startData && (
            <button
              className="h-10 px-4 rounded-xl bg-ag-primary text-white text-sm font-semibold hover:bg-blue-700 inline-flex items-center gap-2"
              onClick={startOAuth}
              disabled={busy}
            >
              {busy ? <Loader2 size={16} className="animate-spin" /> : <Link2 size={16} />}
              Generate login URL
            </button>
          )}

          {startData && (
            <div className="space-y-3">
              <div className="rounded-xl border border-ag-border overflow-hidden">
                <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-ag-muted border-b border-ag-border bg-slate-50">
                  Authorization URL
                </div>
                <textarea
                  className="w-full h-24 p-3 text-xs text-slate-700 bg-white border-0 outline-none resize-none"
                  value={startData.authorizationUrl}
                  readOnly
                />
                <div className="px-3 py-2 border-t border-ag-border bg-slate-50 flex justify-end">
                  <button
                    className="h-8 px-3 rounded-lg border border-ag-border text-sm text-ag-text hover:bg-white inline-flex items-center gap-2"
                    onClick={copyLink}
                  >
                    <Copy size={14} /> Copy URL
                  </button>
                </div>
              </div>

              <div className="rounded-xl border border-ag-border p-3">
                <label className="block text-xs font-semibold uppercase tracking-wide text-ag-muted mb-2">
                  Callback URL / Query
                </label>
                <textarea
                  className="w-full h-24 rounded-lg border border-ag-border p-3 text-sm outline-none focus:border-blue-500"
                  placeholder="Paste full callback URL or query only: code=...&state=..."
                  value={callbackInput}
                  onChange={(event) => setCallbackInput(event.target.value)}
                />
                <div className="mt-3 flex items-center gap-2">
                  <button
                    className="h-9 px-4 rounded-lg bg-ag-primary text-white text-sm font-medium hover:bg-blue-700"
                    onClick={completeManual}
                    disabled={busy || !callbackInput.trim()}
                  >
                    Confirm callback
                  </button>
                  <span className="text-xs text-ag-muted">Status: {statusText(status)}</span>
                </div>
              </div>

              {flow?.error && (
                <div className="rounded-lg border border-red-200 bg-red-50 text-red-700 px-3 py-2 text-sm">
                  {flow.error}
                </div>
              )}

              {flow?.status === 'completed' && (
                <div className="rounded-lg border border-emerald-200 bg-emerald-50 text-emerald-700 px-3 py-2 text-sm">
                  Authorization completed: account added.
                </div>
              )}
            </div>
          )}

          {error && (
            <div className="rounded-lg border border-red-200 bg-red-50 text-red-700 px-3 py-2 text-sm">
              {error}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}