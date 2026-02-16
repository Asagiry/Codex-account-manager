export type Tokens = {
  idToken: string
  accessToken: string
  refreshToken: string
}

export type QuotaWindow = {
  usedPercent: number | null
  limitWindowSeconds: number | null
  resetAt: number | null
  fetchedAt: number | null
}

export type QuotaInfo = {
  planType: string | null
  primary: QuotaWindow
  secondary: QuotaWindow
  fetchedAt: number
}

export type Account = {
  id: string
  email: string | null
  accountId: string | null
  tokens: Tokens
  quota: QuotaInfo | null
  createdAt: number
  lastLoginAt: number
  lastError: string | null
}

export type ProxyEntry = {
  id: string
  login: string
  password: string
  host: string
  port: number
  raw: string
  lastLatencyMs: number | null
  lastStatus: string | null
  lastCheckedAt: number | null
}

export type IdeTarget = 'vscode' | 'cursor' | 'windsurf' | 'trae' | 'vscodium' | 'zed'

export type AppData = {
  accounts: Account[]
  activeAccountId: string | null
  proxies: ProxyEntry[]
  activeProxyId: string | null
  limitsBaseUrl: string
  preferredIde: IdeTarget | null
}

export type OAuthStartResponse = {
  flowId: string
  authorizationUrl: string
  redirectUri: string
}

export type OAuthFlowResponse = {
  flowId: string
  authorizationUrl: string
  callbackUrl: string | null
  createdAt: number
  status: 'waiting_callback' | 'exchanging' | 'completed' | 'error'
  error: string | null
  account: Account | null
}

export type ProxyTestResult = {
  proxyId: string
  reachable: boolean
  latencyMs: number | null
  checkedAt: number
  error: string | null
}

export type SwitchAccountResponse = {
  state: AppData
  ide: IdeTarget | null
  reloaded: boolean
  warning: string | null
}