export function formatPercent(value: number | null | undefined): string {
  if (value == null || Number.isNaN(value)) return '-'
  return `${Math.round(value)}%`
}

export function clampPercent(value: number | null | undefined): number {
  if (value == null || Number.isNaN(value)) return 0
  if (value < 0) return 0
  if (value > 100) return 100
  return value
}

export function remainingPercent(value: number | null | undefined): number | null {
  if (value == null || Number.isNaN(value)) return null
  return clampPercent(100 - clampPercent(value))
}

export function formatRemainingPercent(value: number | null | undefined): string {
  const remaining = remainingPercent(value)
  if (remaining == null) return '-'
  return `${Math.round(remaining)}%`
}

export function formatTimeUntil(unixTsSeconds: number | null | undefined): string {
  if (!unixTsSeconds) return '-'

  const now = Date.now()
  const target = unixTsSeconds * 1000
  const deltaMs = target - now
  const absMs = Math.abs(deltaMs)

  const totalMinutes = Math.floor(absMs / 60000)
  const days = Math.floor(totalMinutes / (60 * 24))
  const hours = Math.floor((totalMinutes % (60 * 24)) / 60)
  const minutes = totalMinutes % 60

  const parts: string[] = []
  if (days > 0) parts.push(`${days}d`)
  if (hours > 0) parts.push(`${hours}h`)
  if (minutes > 0 || parts.length === 0) parts.push(`${minutes}m`)

  const body = parts.slice(0, 2).join(' ')
  return deltaMs >= 0 ? `in ${body}` : `${body} ago`
}

export function mask(value: string, visible = 6): string {
  if (!value) return ''
  if (value.length <= visible) return '*'.repeat(value.length)
  return `${value.slice(0, 3)}${'*'.repeat(Math.max(0, value.length - 6))}${value.slice(-3)}`
}
