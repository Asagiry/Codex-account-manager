import { invoke } from '@tauri-apps/api/core'
import type {
  Account,
  AppData,
  OAuthFlowResponse,
  OAuthStartResponse,
  ProxyTestResult,
  SwitchAccountResponse,
  IdeTarget
} from './types'

export const api = {
  getState: () => invoke<AppData>('get_app_state'),
  getStoragePath: () => invoke<string>('get_storage_path'),

  startOAuthFlow: () => invoke<OAuthStartResponse>('start_oauth_flow'),
  getOAuthStatus: (flowId: string) =>
    invoke<OAuthFlowResponse>('get_oauth_flow_status', { flowId }),
  completeOAuthWithCallback: (flowId: string, callbackUrl: string) =>
    invoke<OAuthFlowResponse>('complete_oauth_with_callback', { flowId, callbackUrl }),

  removeAccount: (accountId: string) =>
    invoke<AppData>('remove_account', { accountId }),
  setActiveAccount: (accountId: string) =>
    invoke<AppData>('set_active_account', { accountId }),
  setPreferredIde: (ide?: IdeTarget) =>
    invoke<AppData>('set_preferred_ide', { ide: ide ?? null }),
  switchAccountForIde: (accountId: string, ide?: IdeTarget) =>
    invoke<SwitchAccountResponse>('switch_account_for_ide', {
      accountId,
      ide: ide ?? null
    }),

  refreshAccountQuota: (accountId: string) =>
    invoke<Account>('refresh_account_quota', { accountId }),
  refreshAllQuotas: () => invoke<AppData>('refresh_all_quotas'),

  saveProxy: (proxyValue: string, proxyId?: string) =>
    invoke<AppData>('save_proxy', {
      proxyValue,
      proxyId: proxyId ?? null
    }),
  deleteProxy: (proxyId: string) =>
    invoke<AppData>('delete_proxy', { proxyId }),
  setActiveProxy: (proxyId?: string) =>
    invoke<AppData>('set_active_proxy', { proxyId: proxyId ?? null }),
  testProxy: (proxyId: string) =>
    invoke<ProxyTestResult>('test_proxy', { proxyId })
}