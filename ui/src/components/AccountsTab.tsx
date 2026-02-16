import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  ArrowRightLeft,
  CircleAlert,
  CircleCheck,
  Info,
  Loader2,
  RefreshCw,
  Trash2,
  UserPlus
} from 'lucide-react'
import { api } from '../api'
import { formatRemainingPercent, remainingPercent, formatTimeUntil } from '../format'
import type { Account, AppData, IdeTarget } from '../types'
import { OAuthModal } from './OAuthModal'

type AccountsTabProps = {
  data: AppData
  setData: (next: AppData) => void
  reload: () => Promise<void>
}

const IDE_OPTIONS: Array<{ value: IdeTarget; label: string }> = [
  { value: 'vscode', label: 'VS Code' },
  { value: 'cursor', label: 'Cursor' },
  { value: 'windsurf', label: 'Windsurf' },
  { value: 'trae', label: 'Trae' },
  { value: 'vscodium', label: 'VSCodium' },
  { value: 'zed', label: 'Zed' }
]

const AUTO_REFRESH_MS = 5 * 60 * 1000

function quotaClass(remaining: number): string {
  if (remaining <= 10) return 'quota-fill-danger'
  if (remaining <= 30) return 'quota-fill-warn'
  return 'quota-fill-good'
}

function QuotaCell({
  value,
  resetAt,
  title
}: {
  value: number | null | undefined
  resetAt: number | null | undefined
  title: string
}) {
  const remaining = remainingPercent(value)
  const barPercent = remaining ?? 0

  return (
    <div className="min-w-[180px]">
      <div className="flex items-center justify-between text-xs text-ag-muted mb-1">
        <span>{title}</span>
        <span className="font-semibold text-ag-text">{formatRemainingPercent(value)} left</span>
      </div>
      <div className="quota-track">
        <div className={`quota-fill ${quotaClass(barPercent)}`} style={{ width: `${barPercent}%` }} />
      </div>
      <div className="text-xs text-ag-muted mt-1">reset {formatTimeUntil(resetAt)}</div>
    </div>
  )
}

function AccountInfoModal({ account, onClose }: { account: Account | null; onClose: () => void }) {
  if (!account) return null

  return (
    <div className="fixed inset-0 z-50 bg-slate-900/45 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="w-full max-w-3xl rounded-2xl border border-ag-border bg-ag-card shadow-ag overflow-hidden">
        <div className="px-5 py-4 border-b border-ag-border flex items-center justify-between">
          <div>
            <div className="text-lg font-semibold text-ag-text">Account Info</div>
            <div className="text-xs text-ag-muted mt-1">Full account payload currently stored in app data</div>
          </div>
          <button
            className="h-9 px-3 rounded-lg border border-ag-border text-sm font-semibold text-ag-text hover:bg-ag-surface"
            onClick={onClose}
          >
            Close
          </button>
        </div>
        <div className="p-4 bg-ag-surface">
          <pre className="m-0 max-h-[65vh] overflow-auto rounded-xl border border-ag-border bg-ag-card p-4 text-xs leading-5 text-ag-text">
            {JSON.stringify(account, null, 2)}
          </pre>
        </div>
      </div>
    </div>
  )
}

export function AccountsTab({ data, setData, reload }: AccountsTabProps) {
  const [oauthOpen, setOauthOpen] = useState(false)
  const [infoAccount, setInfoAccount] = useState<Account | null>(null)
  const [busyKey, setBusyKey] = useState<string | null>(null)
  const [refreshingAll, setRefreshingAll] = useState(false)
  const [autoRefreshing, setAutoRefreshing] = useState(false)
  const [lastAutoRefresh, setLastAutoRefresh] = useState<number | null>(null)
  const [ideTarget, setIdeTarget] = useState<IdeTarget | null>(data.preferredIde)
  const [error, setError] = useState<string | null>(null)

  const accounts = useMemo(
    () => [...data.accounts].sort((a, b) => b.lastLoginAt - a.lastLoginAt),
    [data.accounts]
  )

  useEffect(() => {
    if (!ideTarget && data.preferredIde) {
      setIdeTarget(data.preferredIde)
    }
  }, [data.preferredIde, ideTarget])

  const changeIdeTarget = async (nextRaw: string) => {
    const next = nextRaw.length > 0 ? (nextRaw as IdeTarget) : undefined
    try {
      setError(null)
      setIdeTarget(next ?? null)
      const updated = await api.setPreferredIde(next)
      setData(updated)
    } catch (err) {
      setError(String(err))
    }
  }

  const remove = async (accountId: string) => {
    try {
      setBusyKey(`delete:${accountId}`)
      setError(null)
      const next = await api.removeAccount(accountId)
      setData(next)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyKey(null)
    }
  }

  const switchAccount = async (accountId: string) => {
    if (!ideTarget) {
      setError('Choose IDE target first. It will be remembered for future switches.')
      return
    }

    try {
      setBusyKey(`switch:${accountId}`)
      setError(null)
      const response = await api.switchAccountForIde(accountId, ideTarget)
      setData(response.state)
      setIdeTarget(response.ide)

      if (response.warning) {
        setError(response.warning)
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyKey(null)
    }
  }

  const refreshOne = async (accountId: string) => {
    try {
      setBusyKey(`quota:${accountId}`)
      setError(null)
      const updated = await api.refreshAccountQuota(accountId)
      const next: AppData = {
        ...data,
        accounts: data.accounts.map((account) => (account.id === accountId ? updated : account))
      }
      setData(next)
    } catch (err) {
      setError(String(err))
    } finally {
      setBusyKey(null)
    }
  }

  const refreshAll = async (silent = false) => {
    try {
      if (silent) {
        setAutoRefreshing(true)
      } else {
        setRefreshingAll(true)
      }
      setError(null)
      const next = await api.refreshAllQuotas()
      setData(next)
      if (silent) {
        setLastAutoRefresh(Date.now())
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setRefreshingAll(false)
      setAutoRefreshing(false)
    }
  }

  const runAutoRefresh = useCallback(async () => {
    if (data.accounts.length === 0) return
    await refreshAll(true)
  }, [data.accounts.length])

  useEffect(() => {
    const timer = setInterval(() => {
      void runAutoRefresh()
    }, AUTO_REFRESH_MS)

    return () => clearInterval(timer)
  }, [runAutoRefresh])

  return (
    <div className="page-fade h-full flex flex-col gap-4">
      <OAuthModal
        open={oauthOpen}
        onClose={() => setOauthOpen(false)}
        onCompleted={async () => {
          await reload()
        }}
      />

      <AccountInfoModal account={infoAccount} onClose={() => setInfoAccount(null)} />

      <div className="rounded-2xl border border-ag-border bg-ag-card shadow-ag p-4 flex items-center gap-3 flex-wrap">
        <button
          className="h-10 px-4 rounded-xl bg-ag-primary text-white text-sm font-semibold hover:bg-blue-700 inline-flex items-center gap-2"
          onClick={() => setOauthOpen(true)}
        >
          <UserPlus size={16} /> Add OAuth account
        </button>

        <button
          className="h-10 px-4 rounded-xl border border-ag-border text-sm font-semibold text-ag-text hover:bg-ag-surface inline-flex items-center gap-2"
          onClick={() => void refreshAll(false)}
          disabled={refreshingAll || data.accounts.length === 0}
        >
          {refreshingAll ? <Loader2 size={16} className="animate-spin" /> : <RefreshCw size={16} />}
          Refresh all quotas
        </button>

        <div className="h-10 px-3 rounded-xl border border-ag-border inline-flex items-center gap-2 bg-ag-card">
          <span className="text-xs text-ag-muted font-semibold">IDE for switch</span>
          <select
            className="h-8 rounded-lg border border-ag-border bg-ag-card px-2 text-sm text-ag-text outline-none"
            value={ideTarget ?? ''}
            onChange={(event) => void changeIdeTarget(event.target.value)}
          >
            <option value="">Choose IDE...</option>
            {IDE_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="ml-auto text-xs text-ag-muted flex items-center gap-3">
          <span>
            Auto refresh 5m: <span className="font-medium">{autoRefreshing ? 'running...' : 'enabled'}</span>
          </span>
          <span>
            Last auto refresh:{' '}
            <span className="font-medium">{lastAutoRefresh ? new Date(lastAutoRefresh).toLocaleTimeString() : '-'}</span>
          </span>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</div>
      )}

      <div className="flex-1 min-h-0 rounded-2xl border border-ag-border bg-ag-card shadow-ag overflow-hidden">
        <div className="h-full overflow-auto">
          <table className="w-full border-collapse text-sm">
            <thead className="sticky top-0 bg-ag-surface z-10">
              <tr className="text-left text-xs uppercase tracking-wide text-ag-muted border-b border-ag-border">
                <th className="px-4 py-3">Account</th>
                <th className="px-4 py-3">5h quota</th>
                <th className="px-4 py-3">Weekly quota</th>
                <th className="px-4 py-3">Status</th>
                <th className="px-4 py-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {accounts.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-12 text-center text-ag-muted">
                    No accounts yet. Add your first OAuth account.
                  </td>
                </tr>
              )}

              {accounts.map((account) => {
                const isActive = data.activeAccountId === account.id
                const quota = account.quota
                const switching = busyKey === `switch:${account.id}`
                const quotaLoading = busyKey === `quota:${account.id}`
                const removing = busyKey === `delete:${account.id}`

                return (
                  <tr
                    key={account.id}
                    className={`border-b border-ag-border/70 hover:bg-ag-surface/70 ${isActive ? 'bg-ag-surface/70' : ''}`}
                  >
                    <td className="px-4 py-3 align-top">
                      <div className="font-semibold text-ag-text inline-flex items-center gap-2">
                        <span>{account.email ?? 'Unknown email'}</span>
                        <span className="rounded-full border border-ag-border bg-ag-surface px-2 py-0.5 text-[10px] font-bold text-ag-muted">
                          (TEAM)
                        </span>
                      </div>
                      <div className="text-xs text-ag-muted mt-1">
                        last login: {new Date(account.lastLoginAt * 1000).toLocaleString()}
                      </div>
                      {isActive && (
                        <div className="mt-2 inline-flex items-center gap-1 rounded-full border border-ag-primary/35 bg-ag-surface px-2 py-0.5 text-[11px] font-semibold text-ag-primary">
                          Active in Codex
                        </div>
                      )}
                    </td>
                    <td className="px-4 py-3 align-top">
                      <QuotaCell
                        value={quota?.primary.usedPercent}
                        resetAt={quota?.primary.resetAt}
                        title="Primary"
                      />
                    </td>
                    <td className="px-4 py-3 align-top">
                      <QuotaCell
                        value={quota?.secondary.usedPercent}
                        resetAt={quota?.secondary.resetAt}
                        title="Secondary"
                      />
                    </td>
                    <td className="px-4 py-3 align-top">
                      {account.lastError ? (
                        <span className="inline-flex items-center gap-1 text-xs text-red-600">
                          <CircleAlert size={14} /> {account.lastError.slice(0, 96)}
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 text-xs text-emerald-600">
                          <CircleCheck size={14} /> healthy
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 align-top">
                      <div className="flex justify-start gap-2">
                        <button
                          className={`h-8 w-8 inline-flex items-center justify-center rounded-lg border ${
                            isActive
                              ? 'border-ag-primary/45 bg-ag-surface text-ag-primary'
                              : 'border-ag-border text-ag-muted hover:text-ag-text hover:bg-ag-surface'
                          }`}
                          onClick={() => void switchAccount(account.id)}
                          disabled={switching || refreshingAll || autoRefreshing}
                          title="Switch Codex account and reload selected IDE"
                        >
                          {switching ? <Loader2 size={14} className="animate-spin" /> : <ArrowRightLeft size={14} />}
                        </button>

                        <button
                          className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-ag-border text-ag-muted hover:text-ag-text hover:bg-ag-surface"
                          onClick={() => void refreshOne(account.id)}
                          disabled={quotaLoading || switching || removing}
                          title="Force refresh quota"
                        >
                          {quotaLoading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
                        </button>

                        <button
                          className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-ag-border text-ag-muted hover:text-ag-text hover:bg-ag-surface"
                          onClick={() => setInfoAccount(account)}
                          disabled={quotaLoading || switching || removing}
                          title="Account details"
                        >
                          <Info size={14} />
                        </button>

                        <button
                          className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-red-200 text-red-600 hover:bg-red-50"
                          onClick={() => void remove(account.id)}
                          disabled={removing || switching}
                          title="Delete account"
                        >
                          {removing ? <Loader2 size={14} className="animate-spin" /> : <Trash2 size={14} />}
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
