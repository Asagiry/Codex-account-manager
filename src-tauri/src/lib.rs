use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::State;
use url::Url;
use uuid::Uuid;

const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OAUTH_ISSUER: &str = "https://auth.openai.com";
const OAUTH_SCOPE: &str = "openid profile email offline_access";
const OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const OAUTH_ORIGINATOR: &str = "codex_cli_rs";
const CALLBACK_ADDR: &str = "127.0.0.1:1455";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub used_percent: Option<f64>,
    pub limit_window_seconds: Option<i64>,
    pub reset_at: Option<i64>,
    pub fetched_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfo {
    pub plan_type: Option<String>,
    pub primary: QuotaWindow,
    pub secondary: QuotaWindow,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub tokens: Tokens,
    pub quota: Option<QuotaInfo>,
    pub created_at: i64,
    pub last_login_at: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyEntry {
    pub id: String,
    pub login: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub raw: String,
    pub last_latency_ms: Option<u64>,
    pub last_status: Option<String>,
    pub last_checked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
    pub proxies: Vec<ProxyEntry>,
    pub active_proxy_id: Option<String>,
    pub limits_base_url: String,
    #[serde(default)]
    pub preferred_ide: Option<String>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            active_account_id: None,
            proxies: Vec::new(),
            active_proxy_id: None,
            limits_base_url: "https://chatgpt.com/backend-api".to_string(),
            preferred_ide: None,
        }
    }
}

#[derive(Debug, Clone)]
enum OauthFlowStatus {
    WaitingCallback,
    Exchanging,
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
struct OauthFlow {
    id: String,
    state: String,
    code_verifier: String,
    created_at: i64,
    authorization_url: String,
    callback_url: Option<String>,
    result_account_id: Option<String>,
    status: OauthFlowStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OauthStartResponse {
    flow_id: String,
    authorization_url: String,
    redirect_uri: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OauthFlowResponse {
    flow_id: String,
    authorization_url: String,
    callback_url: Option<String>,
    created_at: i64,
    status: String,
    error: Option<String>,
    account: Option<Account>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyTestResult {
    proxy_id: String,
    reachable: bool,
    latency_ms: Option<u64>,
    checked_at: i64,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchAccountResponse {
    state: AppData,
    ide: Option<String>,
    reloaded: bool,
    warning: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedProxy {
    login: String,
    password: String,
    host: String,
    port: u16,
}

struct SharedState {
    data: Mutex<AppData>,
    flows: Mutex<HashMap<String, OauthFlow>>,
    callback_server_started: AtomicBool,
}

impl SharedState {
    fn new(initial: AppData) -> Self {
        Self {
            data: Mutex::new(initial),
            flows: Mutex::new(HashMap::new()),
            callback_server_started: AtomicBool::new(false),
        }
    }
}

fn now_ts() -> i64 {
    Utc::now().timestamp()
}

fn app_storage_dir() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Cannot determine data directory".to_string())?;
    let dir = base.join("CodexAccountManager");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create data directory: {e}"))?;
    Ok(dir)
}

fn app_storage_file() -> Result<PathBuf, String> {
    Ok(app_storage_dir()?.join("state.json"))
}

fn load_app_data() -> Result<AppData, String> {
    let path = app_storage_file()?;
    if !path.exists() {
        return Ok(AppData::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("Failed to read state file: {e}"))?;
    let parsed: AppData =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse state file: {e}"))?;
    Ok(parsed)
}

fn save_app_data(data: &AppData) -> Result<(), String> {
    let path = app_storage_file()?;
    let text = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize state: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("Failed to write state file: {e}"))?;
    Ok(())
}

fn lock_data<'a>(
    state: &'a Arc<SharedState>,
) -> Result<std::sync::MutexGuard<'a, AppData>, String> {
    state
        .data
        .lock()
        .map_err(|_| "State lock poisoned (data)".to_string())
}

fn lock_flows<'a>(
    state: &'a Arc<SharedState>,
) -> Result<std::sync::MutexGuard<'a, HashMap<String, OauthFlow>>, String> {
    state
        .flows
        .lock()
        .map_err(|_| "State lock poisoned (oauth flows)".to_string())
}

fn random_urlsafe(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn build_pkce() -> (String, String) {
    let verifier = random_urlsafe(64);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn build_authorize_url(state: &str, challenge: &str) -> Result<String, String> {
    let mut url = Url::parse(&format!("{OAUTH_ISSUER}/oauth/authorize"))
        .map_err(|e| format!("Failed to build OAuth URL: {e}"))?;

    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", OAUTH_CLIENT_ID)
        .append_pair("redirect_uri", OAUTH_REDIRECT_URI)
        .append_pair("scope", OAUTH_SCOPE)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("originator", OAUTH_ORIGINATOR)
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true");

    Ok(url.to_string())
}
fn parse_proxy_input(raw: &str) -> Result<ParsedProxy, String> {
    let mut text = raw.trim().to_string();
    if text.starts_with("http://") {
        text = text.trim_start_matches("http://").to_string();
    }
    if text.starts_with("https://") {
        text = text.trim_start_matches("https://").to_string();
    }

    let (credentials, host_part) = text
        .split_once('@')
        .ok_or_else(|| "Proxy must be in login:pass@ip:port format".to_string())?;
    let (login, password) = credentials
        .split_once(':')
        .ok_or_else(|| "Proxy must include login and password".to_string())?;
    let (host, port_text) = host_part
        .rsplit_once(':')
        .ok_or_else(|| "Proxy must include ip and port".to_string())?;

    let login = login.trim();
    let password = password.trim();
    let host = host.trim();
    let port: u16 = port_text
        .trim()
        .parse()
        .map_err(|_| "Proxy port must be a valid number".to_string())?;

    if login.is_empty() || password.is_empty() || host.is_empty() {
        return Err("Proxy fields cannot be empty".to_string());
    }

    Ok(ParsedProxy {
        login: login.to_string(),
        password: password.to_string(),
        host: host.to_string(),
        port,
    })
}

fn proxy_to_url(proxy: &ProxyEntry) -> Result<String, String> {
    let mut url = Url::parse(&format!("http://{}:{}", proxy.host, proxy.port))
        .map_err(|e| format!("Failed to build proxy url: {e}"))?;
    url.set_username(&proxy.login)
        .map_err(|_| "Invalid proxy login".to_string())?;
    url.set_password(Some(&proxy.password))
        .map_err(|_| "Invalid proxy password".to_string())?;
    Ok(url.to_string())
}

fn active_proxy(data: &AppData) -> Option<ProxyEntry> {
    let active_id = data.active_proxy_id.as_ref()?;
    data.proxies.iter().find(|p| &p.id == active_id).cloned()
}

fn normalize_ide_target(input: &str) -> Option<String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "vscode" | "code" => Some("vscode".to_string()),
        "cursor" => Some("cursor".to_string()),
        "windsurf" => Some("windsurf".to_string()),
        "trae" => Some("trae".to_string()),
        "vscodium" | "codium" => Some("vscodium".to_string()),
        "zed" => Some("zed".to_string()),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn run_hidden_powershell(script: &str) -> Result<Output, String> {
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to execute PowerShell command: {e}"))
}

#[cfg(target_os = "windows")]
fn trigger_ide_reload_command(ide: &str) -> Result<bool, String> {
    let cli_candidates: &[&str] = match ide {
        "vscode" => &["code", "code-insiders"],
        "cursor" => &["cursor"],
        "windsurf" => &["windsurf"],
        "trae" => &["trae"],
        "vscodium" => &["codium"],
        "zed" => &["zed"],
        _ => return Err("Unsupported IDE target".to_string()),
    };

    let quoted = cli_candidates
        .iter()
        .map(|name| format!("'{}'", name))
        .collect::<Vec<_>>()
        .join(",");

    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; \
$cmds=@({quoted}); \
$ok=$false; \
foreach ($cmd in $cmds) {{ \
  if (Get-Command $cmd -ErrorAction SilentlyContinue) {{ \
    & $cmd --reuse-window --command workbench.action.reloadWindow | Out-Null; \
    $ok=$true; \
  }} \
}}; \
if ($ok) {{ exit 0 }} else {{ exit 2 }}"
    );

    let output = run_hidden_powershell(&script)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Err("IDE reload command failed".to_string())
            } else {
                Err(format!("IDE reload command failed: {stderr}"))
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn trigger_ide_reload_command(_ide: &str) -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
fn restart_ide_processes(process_names: &[&str]) -> Result<bool, String> {
    let quoted = process_names
        .iter()
        .map(|name| format!("'{}'", name))
        .collect::<Vec<_>>()
        .join(",");

    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; \
$names=@({quoted}); \
$found=$false; \
foreach ($name in $names) {{ \
  $procs=Get-Process -Name $name -ErrorAction SilentlyContinue; \
  foreach ($p in $procs) {{ \
    $found=$true; \
    $path=$p.Path; \
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue; \
    if ($path) {{ Start-Process -WindowStyle Hidden -FilePath $path | Out-Null }} \
  }} \
}}; \
if ($found) {{ exit 0 }} else {{ exit 2 }}"
    );

    let output = run_hidden_powershell(&script)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Err("IDE process restart command failed".to_string())
            } else {
                Err(format!("IDE process restart command failed: {stderr}"))
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn restart_ide_processes(_process_names: &[&str]) -> Result<bool, String> {
    Ok(false)
}

fn reload_ide_windows(ide: &str) -> Result<bool, String> {
    match trigger_ide_reload_command(ide) {
        Ok(true) => Ok(true),
        Ok(false) => {
            let process_names: &[&str] = match ide {
                "vscode" => &["Code", "Code - Insiders"],
                "cursor" => &["Cursor"],
                "windsurf" => &["Windsurf"],
                "trae" => &["Trae"],
                "vscodium" => &["VSCodium"],
                "zed" => &["Zed"],
                _ => return Err("Unsupported IDE target".to_string()),
            };
            restart_ide_processes(process_names)
        }
        Err(reload_err) => {
            let process_names: &[&str] = match ide {
                "vscode" => &["Code", "Code - Insiders"],
                "cursor" => &["Cursor"],
                "windsurf" => &["Windsurf"],
                "trae" => &["Trae"],
                "vscodium" => &["VSCodium"],
                "zed" => &["Zed"],
                _ => return Err("Unsupported IDE target".to_string()),
            };

            match restart_ide_processes(process_names) {
                Ok(result) => Ok(result),
                Err(restart_err) => Err(format!(
                    "IDE reload failed ({reload_err}) and restart fallback failed ({restart_err})"
                )),
            }
        }
    }
}
fn build_http_client(
    timeout: Duration,
    proxy: Option<&ProxyEntry>,
) -> Result<reqwest::blocking::Client, String> {
    let mut builder = reqwest::blocking::Client::builder().timeout(timeout);
    if let Some(proxy_entry) = proxy {
        let proxy_url = proxy_to_url(proxy_entry)?;
        let proxy_cfg =
            reqwest::Proxy::all(proxy_url).map_err(|e| format!("Invalid proxy: {e}"))?;
        builder = builder.proxy(proxy_cfg);
    }

    builder
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

fn parse_quota_window(win: Option<&Value>) -> QuotaWindow {
    let Some(win) = win else {
        return QuotaWindow::default();
    };

    QuotaWindow {
        used_percent: win.get("used_percent").and_then(Value::as_f64),
        limit_window_seconds: win.get("limit_window_seconds").and_then(Value::as_i64),
        reset_at: win.get("reset_at").and_then(Value::as_i64),
        fetched_at: Some(now_ts()),
    }
}

fn fetch_quota(
    base_url: &str,
    tokens: &Tokens,
    account_id: Option<&str>,
    proxy: Option<&ProxyEntry>,
) -> Result<QuotaInfo, String> {
    if tokens.access_token.trim().is_empty() {
        return Err("Missing access_token".to_string());
    }

    let base = base_url.trim_end_matches('/');
    let endpoint = if base.contains("/backend-api") {
        format!("{base}/wham/usage")
    } else {
        format!("{base}/api/codex/usage")
    };

    let client = build_http_client(Duration::from_secs(30), proxy)?;

    let mut request = client
        .get(endpoint)
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .header("User-Agent", "codex-cli");

    if let Some(account_id) = account_id {
        if !account_id.trim().is_empty() {
            request = request.header("ChatGPT-Account-Id", account_id);
        }
    }

    let response = request
        .send()
        .map_err(|e| format!("Quota request failed: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|e| format!("Failed to read quota response: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "Quota request failed ({status}): {}",
            body.chars().take(240).collect::<String>()
        ));
    }

    let payload: Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid quota payload: {e}"))?;

    let rate_limit = payload.get("rate_limit");
    let primary = parse_quota_window(rate_limit.and_then(|v| v.get("primary_window")));
    let secondary = parse_quota_window(rate_limit.and_then(|v| v.get("secondary_window")));

    Ok(QuotaInfo {
        plan_type: payload
            .get("plan_type")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        primary,
        secondary,
        fetched_at: now_ts(),
    })
}

fn decode_jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let mut padded = payload.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let decoded = base64::engine::general_purpose::URL_SAFE
        .decode(padded.as_bytes())
        .ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn extract_account_id(id_token: &str) -> Option<String> {
    let payload = decode_jwt_payload(id_token)?;
    let auth_claim = payload
        .get("https://api.openai.com/auth")
        .and_then(Value::as_object);

    auth_claim
        .and_then(|auth| {
            auth.get("chatgpt_account_id")
                .or_else(|| auth.get("account_id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            payload
                .get("sub")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn extract_email(id_token: &str) -> Option<String> {
    let payload = decode_jwt_payload(id_token)?;
    payload
        .get("email")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn exchange_code_for_tokens(
    code: &str,
    code_verifier: &str,
    proxy: Option<&ProxyEntry>,
) -> Result<Tokens, String> {
    let client = build_http_client(Duration::from_secs(45), proxy)
        .map_err(|e| format!("Failed to create OAuth client: {e}"))?;

    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", OAUTH_CLIENT_ID),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", OAUTH_REDIRECT_URI),
    ];

    let response = client
        .post(format!("{OAUTH_ISSUER}/oauth/token"))
        .header("Accept", "application/json")
        .header("User-Agent", "codex-cli")
        .form(&form)
        .send()
        .map_err(|e| format!("OAuth token request failed: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|e| format!("Failed to read OAuth response: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "OAuth exchange failed ({status}): {}",
            body.chars().take(240).collect::<String>()
        ));
    }

    let payload: Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid OAuth payload: {e}"))?;

    let access_token = payload
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "OAuth payload missing access_token".to_string())?
        .to_string();

    Ok(Tokens {
        id_token: payload
            .get("id_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        access_token,
        refresh_token: payload
            .get("refresh_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn codex_auth_path() -> Result<PathBuf, String> {
    let home =
        dirs::home_dir().ok_or_else(|| "Cannot determine user home directory".to_string())?;
    let codex_dir = home.join(".codex");
    fs::create_dir_all(&codex_dir)
        .map_err(|e| format!("Failed to create .codex directory: {e}"))?;
    Ok(codex_dir.join("auth.json"))
}

fn write_codex_auth(tokens: &Tokens, account_id: Option<&str>) -> Result<(), String> {
    let path = codex_auth_path()?;
    let data = json!({
        "OPENAI_API_KEY": Value::Null,
        "tokens": {
            "id_token": tokens.id_token,
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "account_id": account_id,
        },
        "last_refresh": Utc::now().to_rfc3339(),
    });

    let text = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize auth.json: {e}"))?;
    fs::write(path, text).map_err(|e| format!("Failed to write auth.json: {e}"))?;
    Ok(())
}
fn upsert_account(
    data: &mut AppData,
    tokens: Tokens,
    email: Option<String>,
    account_id: Option<String>,
) -> Account {
    let now = now_ts();

    let account = Account {
        id: Uuid::new_v4().to_string(),
        email,
        account_id,
        tokens,
        quota: None,
        created_at: now,
        last_login_at: now,
        last_error: None,
    };

    data.accounts.push(account.clone());

    if data.active_account_id.is_none() {
        data.active_account_id = Some(account.id.clone());
    }

    account
}

fn flow_status_text(status: &OauthFlowStatus) -> String {
    match status {
        OauthFlowStatus::WaitingCallback => "waiting_callback".to_string(),
        OauthFlowStatus::Exchanging => "exchanging".to_string(),
        OauthFlowStatus::Completed => "completed".to_string(),
        OauthFlowStatus::Error(_) => "error".to_string(),
    }
}

fn flow_to_response(flow: &OauthFlow, data: &AppData) -> OauthFlowResponse {
    let error = match &flow.status {
        OauthFlowStatus::Error(msg) => Some(msg.clone()),
        _ => None,
    };

    let account = flow
        .result_account_id
        .as_ref()
        .and_then(|account_id| data.accounts.iter().find(|a| &a.id == account_id).cloned());

    OauthFlowResponse {
        flow_id: flow.id.clone(),
        authorization_url: flow.authorization_url.clone(),
        callback_url: flow.callback_url.clone(),
        created_at: flow.created_at,
        status: flow_status_text(&flow.status),
        error,
        account,
    }
}

fn parse_callback_input(input: &str) -> Result<(String, String, String), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Callback URL is empty".to_string());
    }

    let normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with("/auth/callback") {
        format!("http://localhost:1455{trimmed}")
    } else if trimmed.contains("code=") && trimmed.contains("state=") {
        let query = trimmed.trim_start_matches('?');
        format!("{OAUTH_REDIRECT_URI}?{query}")
    } else {
        return Err(
            "Invalid callback format. Paste full callback URL or query with code/state".to_string(),
        );
    };

    let parsed = Url::parse(&normalized).map_err(|e| format!("Invalid callback URL: {e}"))?;
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;

    for (k, v) in parsed.query_pairs() {
        if k == "code" {
            code = Some(v.to_string());
        } else if k == "state" {
            state = Some(v.to_string());
        }
    }

    let code = code.ok_or_else(|| "Callback does not contain code".to_string())?;
    let state = state.ok_or_else(|| "Callback does not contain state".to_string())?;

    Ok((code, state, normalized))
}

fn test_proxy_latency(proxy: &ProxyEntry) -> Result<u64, String> {
    let addr_text = format!("{}:{}", proxy.host, proxy.port);
    let socket = addr_text
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed: {e}"))?
        .next()
        .ok_or_else(|| "DNS resolution returned no address".to_string())?;

    let started = Instant::now();
    TcpStream::connect_timeout(&socket, Duration::from_secs(4))
        .map_err(|e| format!("TCP connection failed: {e}"))?;
    Ok(started.elapsed().as_millis() as u64)
}

fn ensure_callback_server(state: &Arc<SharedState>) {
    if state.callback_server_started.swap(true, Ordering::SeqCst) {
        return;
    }

    let shared = Arc::clone(state);
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(CALLBACK_ADDR) {
            Ok(listener) => listener,
            Err(err) => {
                log::error!(
                    "OAuth callback listener failed to bind on {}: {}",
                    CALLBACK_ADDR,
                    err
                );
                return;
            }
        };

        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    if let Err(err) = handle_callback_stream(stream, &shared) {
                        log::warn!("OAuth callback handling failed: {}", err);
                    }
                }
                Err(err) => {
                    log::warn!("OAuth callback incoming connection failed: {}", err);
                }
            }
        }
    });
}

fn write_http_response(stream: &mut TcpStream, status: &str, body: &str) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("HTTP response write failed: {e}"))
}

fn html_message(title: &str, message: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>body{{font-family:Segoe UI,Arial,sans-serif;background:#f6f8fb;color:#1d2733;padding:30px}}.card{{max-width:640px;margin:0 auto;background:white;border-radius:14px;padding:24px;box-shadow:0 10px 30px rgba(20,37,63,.08)}}h1{{margin:0 0 12px 0;font-size:22px}}p{{margin:0;font-size:15px;line-height:1.45}}</style></head><body><div class=\"card\"><h1>{title}</h1><p>{message}</p></div></body></html>"
    )
}
fn handle_callback_stream(mut stream: TcpStream, shared: &Arc<SharedState>) -> Result<(), String> {
    let mut buffer = [0u8; 8192];
    let read = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read callback request: {e}"))?;
    if read == 0 {
        return Err("Callback request was empty".to_string());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path_with_query = parts.next().unwrap_or("/");

    if method != "GET" {
        let body = html_message(
            "Method Not Allowed",
            "Only GET is supported for OAuth callback.",
        );
        return write_http_response(&mut stream, "405 Method Not Allowed", &body);
    }

    let callback_url = format!("http://localhost:1455{path_with_query}");
    let parsed = match Url::parse(&callback_url) {
        Ok(url) => url,
        Err(_) => {
            let body = html_message("Invalid Request", "Could not parse callback URL.");
            return write_http_response(&mut stream, "400 Bad Request", &body);
        }
    };

    if parsed.path() != "/auth/callback" {
        let body = html_message(
            "Not Found",
            "This endpoint is only used for OAuth callback.",
        );
        return write_http_response(&mut stream, "404 Not Found", &body);
    }

    let mut code: Option<String> = None;
    let mut state_value: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        if k == "code" {
            code = Some(v.to_string());
        } else if k == "state" {
            state_value = Some(v.to_string());
        }
    }

    let Some(code) = code else {
        let body = html_message("Callback Error", "Query does not contain OAuth code.");
        return write_http_response(&mut stream, "400 Bad Request", &body);
    };

    let Some(state_value) = state_value else {
        let body = html_message("Callback Error", "Query does not contain OAuth state.");
        return write_http_response(&mut stream, "400 Bad Request", &body);
    };

    let flow_id = {
        let mut flows = lock_flows(shared)?;
        let target = flows
            .iter_mut()
            .find(|(_, flow)| flow.state == state_value)
            .map(|(id, flow)| {
                flow.callback_url = Some(callback_url.clone());
                flow.status = OauthFlowStatus::Exchanging;
                id.clone()
            });
        target
    };

    let Some(flow_id) = flow_id else {
        let body = html_message("Callback Error", "No active OAuth flow matched this state.");
        return write_http_response(&mut stream, "400 Bad Request", &body);
    };

    match complete_oauth_code(shared, &flow_id, &code, Some(callback_url)) {
        Ok(_) => {
            let body = html_message(
                "Login Completed",
                "OAuth completed successfully. You can return to the app now.",
            );
            write_http_response(&mut stream, "200 OK", &body)
        }
        Err(err) => {
            let body = html_message("OAuth Failed", &format!("Token exchange failed: {err}"));
            write_http_response(&mut stream, "400 Bad Request", &body)
        }
    }
}

fn complete_oauth_code(
    shared: &Arc<SharedState>,
    flow_id: &str,
    code: &str,
    callback_url: Option<String>,
) -> Result<Account, String> {
    let code_verifier = {
        let mut flows = lock_flows(shared)?;
        let flow = flows
            .get_mut(flow_id)
            .ok_or_else(|| "OAuth flow not found".to_string())?;
        flow.status = OauthFlowStatus::Exchanging;
        if let Some(callback_url) = &callback_url {
            flow.callback_url = Some(callback_url.clone());
        }
        flow.code_verifier.clone()
    };

    let proxy = {
        let data = lock_data(shared)?;
        active_proxy(&data)
    };

    let exchange = exchange_code_for_tokens(code, &code_verifier, proxy.as_ref());

    let result = match exchange {
        Ok(tokens) => {
            let account_id = extract_account_id(&tokens.id_token);
            let email = extract_email(&tokens.id_token);

            let (limits_base_url, quota_proxy) = {
                let data = lock_data(shared)?;
                (data.limits_base_url.clone(), active_proxy(&data))
            };

            let quota_result = fetch_quota(
                &limits_base_url,
                &tokens,
                account_id.as_deref(),
                quota_proxy.as_ref(),
            );

            let account = {
                let mut data = lock_data(shared)?;
                let account = upsert_account(&mut data, tokens, email, account_id);

                let account_mut = data
                    .accounts
                    .iter_mut()
                    .find(|entry| entry.id == account.id)
                    .ok_or_else(|| "Account disappeared during OAuth completion".to_string())?;

                match quota_result {
                    Ok(quota) => {
                        account_mut.quota = Some(quota);
                        account_mut.last_error = None;
                    }
                    Err(err) => {
                        account_mut.last_error = Some(err);
                    }
                }

                let updated = account_mut.clone();
                save_app_data(&data)?;
                updated
            };

            let mut flows = lock_flows(shared)?;
            if let Some(flow) = flows.get_mut(flow_id) {
                flow.status = OauthFlowStatus::Completed;
                flow.result_account_id = Some(account.id.clone());
            }

            Ok(account)
        }
        Err(err) => {
            let mut flows = lock_flows(shared)?;
            if let Some(flow) = flows.get_mut(flow_id) {
                flow.status = OauthFlowStatus::Error(err.clone());
            }
            Err(err)
        }
    };

    result
}

#[tauri::command]
fn get_app_state(state: State<'_, Arc<SharedState>>) -> Result<AppData, String> {
    let data = lock_data(state.inner())?;
    Ok(data.clone())
}

#[tauri::command]
fn get_storage_path() -> Result<String, String> {
    app_storage_file().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn start_oauth_flow(state: State<'_, Arc<SharedState>>) -> Result<OauthStartResponse, String> {
    ensure_callback_server(state.inner());

    let (code_verifier, code_challenge) = build_pkce();
    let flow_state = random_urlsafe(32);
    let auth_url = build_authorize_url(&flow_state, &code_challenge)?;
    let flow_id = Uuid::new_v4().to_string();

    let flow = OauthFlow {
        id: flow_id.clone(),
        state: flow_state,
        code_verifier,
        created_at: now_ts(),
        authorization_url: auth_url.clone(),
        callback_url: None,
        result_account_id: None,
        status: OauthFlowStatus::WaitingCallback,
    };

    let mut flows = lock_flows(state.inner())?;
    flows.insert(flow_id.clone(), flow);

    Ok(OauthStartResponse {
        flow_id,
        authorization_url: auth_url,
        redirect_uri: OAUTH_REDIRECT_URI.to_string(),
    })
}

#[tauri::command]
fn get_oauth_flow_status(
    flow_id: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<OauthFlowResponse, String> {
    let flow = {
        let flows = lock_flows(state.inner())?;
        flows
            .get(&flow_id)
            .cloned()
            .ok_or_else(|| "OAuth flow not found".to_string())?
    };

    let data = lock_data(state.inner())?;
    Ok(flow_to_response(&flow, &data))
}

#[tauri::command]
fn complete_oauth_with_callback(
    flow_id: String,
    callback_url: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<OauthFlowResponse, String> {
    let (code, callback_state, normalized) = parse_callback_input(&callback_url)?;

    {
        let mut flows = lock_flows(state.inner())?;
        let flow = flows
            .get_mut(&flow_id)
            .ok_or_else(|| "OAuth flow not found".to_string())?;

        if flow.state != callback_state {
            flow.status = OauthFlowStatus::Error(
                "State mismatch. Callback belongs to another session.".to_string(),
            );
            return Err(
                "State mismatch. Ensure callback belongs to the current login session.".to_string(),
            );
        }

        flow.callback_url = Some(normalized.clone());
        flow.status = OauthFlowStatus::Exchanging;
    }

    let _ = complete_oauth_code(state.inner(), &flow_id, &code, Some(normalized));

    let flow = {
        let flows = lock_flows(state.inner())?;
        flows
            .get(&flow_id)
            .cloned()
            .ok_or_else(|| "OAuth flow not found after completion".to_string())?
    };
    let data = lock_data(state.inner())?;

    Ok(flow_to_response(&flow, &data))
}
#[tauri::command]
fn remove_account(
    account_id: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppData, String> {
    let mut data = lock_data(state.inner())?;
    data.accounts.retain(|a| a.id != account_id);

    if data.active_account_id.as_ref() == Some(&account_id) {
        data.active_account_id = data.accounts.first().map(|a| a.id.clone());
    }

    save_app_data(&data)?;
    Ok(data.clone())
}

#[tauri::command]
fn set_active_account(
    account_id: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppData, String> {
    let account = {
        let data = lock_data(state.inner())?;
        data.accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| "Account not found".to_string())?
    };

    write_codex_auth(&account.tokens, account.account_id.as_deref())?;

    let mut data = lock_data(state.inner())?;
    data.active_account_id = Some(account_id);
    save_app_data(&data)?;

    Ok(data.clone())
}

#[tauri::command]
fn set_preferred_ide(
    ide: Option<String>,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppData, String> {
    let normalized = match ide {
        Some(value) => Some(
            normalize_ide_target(&value)
                .ok_or_else(|| "Invalid IDE target".to_string())?,
        ),
        None => None,
    };

    let mut data = lock_data(state.inner())?;
    data.preferred_ide = normalized;
    save_app_data(&data)?;

    Ok(data.clone())
}

#[tauri::command]
fn switch_account_for_ide(
    account_id: String,
    ide: Option<String>,
    state: State<'_, Arc<SharedState>>,
) -> Result<SwitchAccountResponse, String> {
    let requested_ide = match ide {
        Some(value) => Some(
            normalize_ide_target(&value)
                .ok_or_else(|| "Invalid IDE target".to_string())?,
        ),
        None => None,
    };

    let (account, fallback_ide) = {
        let data = lock_data(state.inner())?;
        let account = data
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| "Account not found".to_string())?;
        (account, data.preferred_ide.clone())
    };

    write_codex_auth(&account.tokens, account.account_id.as_deref())?;

    let selected_ide = requested_ide.clone().or(fallback_ide);

    let snapshot = {
        let mut data = lock_data(state.inner())?;
        data.active_account_id = Some(account_id);
        if let Some(ide_name) = &requested_ide {
            data.preferred_ide = Some(ide_name.clone());
        }
        save_app_data(&data)?;
        data.clone()
    };

    let (reloaded, warning) = if let Some(ide_name) = selected_ide.as_deref() {
        match reload_ide_windows(ide_name) {
            Ok(true) => (true, None),
            Ok(false) => (
                false,
                Some(format!(
                    "Account switched. No running {ide_name} process was found to reload."
                )),
            ),
            Err(err) => (false, Some(format!("Account switched, but IDE reload failed: {err}"))),
        }
    } else {
        (
            false,
            Some("Account switched. Choose an IDE target to enable auto reload.".to_string()),
        )
    };

    Ok(SwitchAccountResponse {
        state: snapshot,
        ide: selected_ide,
        reloaded,
        warning,
    })
}

#[tauri::command]
fn refresh_account_quota(
    account_id: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<Account, String> {
    let (base_url, account_snapshot, proxy) = {
        let data = lock_data(state.inner())?;
        let account = data
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| "Account not found".to_string())?;

        (data.limits_base_url.clone(), account, active_proxy(&data))
    };

    let quota_result = fetch_quota(
        &base_url,
        &account_snapshot.tokens,
        account_snapshot.account_id.as_deref(),
        proxy.as_ref(),
    );

    let mut data = lock_data(state.inner())?;
    let account = data
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "Account disappeared during update".to_string())?;

    match quota_result {
        Ok(quota) => {
            account.quota = Some(quota);
            account.last_error = None;
        }
        Err(err) => {
            account.last_error = Some(err);
        }
    }

    let result = account.clone();
    save_app_data(&data)?;
    Ok(result)
}

#[tauri::command]
fn refresh_all_quotas(state: State<'_, Arc<SharedState>>) -> Result<AppData, String> {
    let (base_url, accounts, proxy) = {
        let data = lock_data(state.inner())?;
        (
            data.limits_base_url.clone(),
            data.accounts.clone(),
            active_proxy(&data),
        )
    };

    let mut updates: HashMap<String, Result<QuotaInfo, String>> = HashMap::new();
    for account in &accounts {
        let result = fetch_quota(
            &base_url,
            &account.tokens,
            account.account_id.as_deref(),
            proxy.as_ref(),
        );
        updates.insert(account.id.clone(), result);
    }

    let mut data = lock_data(state.inner())?;
    for account in &mut data.accounts {
        if let Some(result) = updates.remove(&account.id) {
            match result {
                Ok(quota) => {
                    account.quota = Some(quota);
                    account.last_error = None;
                }
                Err(err) => {
                    account.last_error = Some(err);
                }
            }
        }
    }

    save_app_data(&data)?;
    Ok(data.clone())
}

#[tauri::command]
fn save_proxy(
    proxy_id: Option<String>,
    proxy_value: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppData, String> {
    let parsed = parse_proxy_input(&proxy_value)?;
    let raw = format!(
        "{}:{}@{}:{}",
        parsed.login, parsed.password, parsed.host, parsed.port
    );

    let mut data = lock_data(state.inner())?;

    if let Some(proxy_id) = proxy_id {
        let proxy = data
            .proxies
            .iter_mut()
            .find(|p| p.id == proxy_id)
            .ok_or_else(|| "Proxy not found".to_string())?;

        proxy.login = parsed.login;
        proxy.password = parsed.password;
        proxy.host = parsed.host;
        proxy.port = parsed.port;
        proxy.raw = raw;
    } else {
        let entry = ProxyEntry {
            id: Uuid::new_v4().to_string(),
            login: parsed.login,
            password: parsed.password,
            host: parsed.host,
            port: parsed.port,
            raw,
            last_latency_ms: None,
            last_status: None,
            last_checked_at: None,
        };
        data.proxies.push(entry);
    }

    save_app_data(&data)?;
    Ok(data.clone())
}

#[tauri::command]
fn delete_proxy(proxy_id: String, state: State<'_, Arc<SharedState>>) -> Result<AppData, String> {
    let mut data = lock_data(state.inner())?;
    data.proxies.retain(|proxy| proxy.id != proxy_id);

    if data.active_proxy_id.as_ref() == Some(&proxy_id) {
        data.active_proxy_id = None;
    }

    save_app_data(&data)?;
    Ok(data.clone())
}

#[tauri::command]
fn set_active_proxy(
    proxy_id: Option<String>,
    state: State<'_, Arc<SharedState>>,
) -> Result<AppData, String> {
    let mut data = lock_data(state.inner())?;

    if let Some(proxy_id) = proxy_id {
        if !data.proxies.iter().any(|proxy| proxy.id == proxy_id) {
            return Err("Proxy not found".to_string());
        }
        data.active_proxy_id = Some(proxy_id);
    } else {
        data.active_proxy_id = None;
    }

    save_app_data(&data)?;
    Ok(data.clone())
}

#[tauri::command]
fn test_proxy(
    proxy_id: String,
    state: State<'_, Arc<SharedState>>,
) -> Result<ProxyTestResult, String> {
    let proxy = {
        let data = lock_data(state.inner())?;
        data.proxies
            .iter()
            .find(|proxy| proxy.id == proxy_id)
            .cloned()
            .ok_or_else(|| "Proxy not found".to_string())?
    };

    let checked_at = now_ts();
    let ping = test_proxy_latency(&proxy);

    let mut data = lock_data(state.inner())?;
    let proxy = data
        .proxies
        .iter_mut()
        .find(|proxy| proxy.id == proxy_id)
        .ok_or_else(|| "Proxy disappeared during update".to_string())?;

    let result = match ping {
        Ok(latency) => {
            proxy.last_latency_ms = Some(latency);
            proxy.last_status = Some("ok".to_string());
            proxy.last_checked_at = Some(checked_at);

            ProxyTestResult {
                proxy_id,
                reachable: true,
                latency_ms: Some(latency),
                checked_at,
                error: None,
            }
        }
        Err(err) => {
            proxy.last_latency_ms = None;
            proxy.last_status = Some("error".to_string());
            proxy.last_checked_at = Some(checked_at);

            ProxyTestResult {
                proxy_id,
                reachable: false,
                latency_ms: None,
                checked_at,
                error: Some(err),
            }
        }
    };

    save_app_data(&data)?;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_data = load_app_data().unwrap_or_else(|err| {
        log::warn!("Failed to load persisted state, using defaults: {}", err);
        AppData::default()
    });

    let shared_state = Arc::new(SharedState::new(initial_data));

    tauri::Builder::default()
        .manage(shared_state)
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            get_storage_path,
            start_oauth_flow,
            get_oauth_flow_status,
            complete_oauth_with_callback,
            remove_account,
            set_active_account,
            set_preferred_ide,
            switch_account_for_ide,
            refresh_account_quota,
            refresh_all_quotas,
            save_proxy,
            delete_proxy,
            set_active_proxy,
            test_proxy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
