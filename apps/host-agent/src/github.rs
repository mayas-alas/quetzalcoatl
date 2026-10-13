//! Native GitHub Actions integration.
//!
//! This module deliberately has no shell fallback and never exposes a bearer
//! token to Setup, argv, an environment variable, or a status projection.
use gnx_control_protocol::{GitHubConnectionState, GitHubStatus, GitHubWorkflowRequest};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

const API_HOST: &str = "api.github.com";
const AUTH_HOST: &str = "github.com";
const API_PORT: u16 = 443;
const TOKEN_FILE: &str = "github.token.dpapi";
const CONFIG_FILE: &str = "github-workflow.json";
const PENDING_FILE: &str = "github-device.json";
const DEFAULT_OWNER: &str = "mayas-alas";
const DEFAULT_REPOSITORY: &str = "gnx-exit-node";
const WORKFLOW: &str = "gnx-ephemeral-exit-node.yml";
const REF: &str = "main";
const ACTIONS_SECRET_NAME: &str = "TAILSCALE_AUTHKEY";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedTarget {
    owner: String,
    repository: String,
    workflow: String,
    reference: String,
}

impl From<&GitHubWorkflowRequest> for SavedTarget {
    fn from(value: &GitHubWorkflowRequest) -> Self {
        Self {
            owner: value.owner.clone(),
            repository: value.repository.clone(),
            workflow: value.workflow.clone(),
            reference: value.reference.clone(),
        }
    }
}

impl SavedTarget {
    fn request(&self) -> GitHubWorkflowRequest {
        GitHubWorkflowRequest {
            owner: self.owner.clone(),
            repository: self.repository.clone(),
            workflow: self.workflow.clone(),
            reference: self.reference.clone(),
        }
    }
}

pub fn initial_status() -> GitHubStatus {
    GitHubStatus {
        connection: GitHubConnectionState::Disconnected,
        auth_method: None,
        repository: None,
        workflow: None,
        run_id: None,
        run_state: None,
        conclusion: None,
        verification_uri: None,
        user_code: None,
        message: None,
    }
}

fn target(
    state_dir: &Path,
    requested: Option<&GitHubWorkflowRequest>,
) -> io::Result<GitHubWorkflowRequest> {
    if let Some(requested) = requested {
        return Ok(requested.clone());
    }
    if let Ok(bytes) = std::fs::read(state_dir.join(CONFIG_FILE)) {
        if let Ok(saved) = serde_json::from_slice::<SavedTarget>(&bytes) {
            let saved = saved.request();
            if validate_target(&saved).is_ok() {
                return Ok(saved);
            }
        }
    }
    // These are fixed identifiers, not a URL or command. A caller may provide
    // another validated typed target in the protocol; no environment override
    // is consulted.
    let owner = DEFAULT_OWNER.to_owned();
    let repository = DEFAULT_REPOSITORY.to_owned();
    let target = GitHubWorkflowRequest {
        owner,
        repository,
        workflow: WORKFLOW.into(),
        reference: REF.into(),
    };
    validate_target(&target)?;
    Ok(target)
}

fn validate_target(target: &GitHubWorkflowRequest) -> io::Result<()> {
    for value in [
        &target.owner,
        &target.repository,
        &target.workflow,
        &target.reference,
    ] {
        if value.is_empty()
            || value.len() > 256
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
            || value.contains('\\')
            || value.contains("..")
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "GITHUB_TARGET_INVALID",
            ));
        }
    }
    if target
        .owner
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        || target
            .repository
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        || target
            .reference
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.')))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GITHUB_TARGET_INVALID",
        ));
    }
    if target.workflow != WORKFLOW {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GITHUB_TARGET_INVALID",
        ));
    }
    if target.owner.contains('/')
        || target.repository.contains('/')
        || target.reference.starts_with('-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GITHUB_TARGET_INVALID",
        ));
    }
    Ok(())
}

fn save_target(state_dir: &Path, target: &GitHubWorkflowRequest) -> io::Result<()> {
    std::fs::create_dir_all(state_dir)?;
    let bytes = serde_json::to_vec(&SavedTarget::from(target)).map_err(io::Error::other)?;
    std::fs::write(state_dir.join(CONFIG_FILE), bytes)?;
    #[cfg(windows)]
    crate::dedicated_account::restrict_system_admin_directory(state_dir)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Run {
    id: u64,
    status: Option<String>,
    conclusion: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}
#[derive(Debug, Deserialize)]
struct DeviceCode {
    device_code: String,
    user_code: String,
    #[allow(dead_code)]
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}
#[derive(Debug, Deserialize)]
struct OAuthToken {
    access_token: Option<String>,
    #[allow(dead_code)]
    error: Option<String>,
}

fn projection(
    target: &GitHubWorkflowRequest,
    connection: GitHubConnectionState,
    method: Option<&str>,
) -> GitHubStatus {
    let mut status = initial_status();
    status.connection = connection;
    status.auth_method = method.map(str::to_owned);
    status.repository = Some(format!("{}/{}", target.owner, target.repository));
    status.workflow = Some(target.workflow.clone());
    status
}

pub fn execute(
    operation: &str,
    state_dir: &Path,
    requested: Option<&GitHubWorkflowRequest>,
    current: Option<&GitHubStatus>,
    tailscale_auth_key: Option<&str>,
) -> io::Result<GitHubStatus> {
    let target = target(state_dir, requested)?;
    validate_target(&target)?;
    save_target(state_dir, &target)?;
    #[cfg(not(windows))]
    {
        let _ = (operation, current, tailscale_auth_key);
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "GitHub actions require the Windows host agent",
        ));
    }
    #[cfg(windows)]
    {
        let mut token = load_token(state_dir)?;
        if operation == "connect" && token.is_none() {
            if let Some(imported) = import_gh_token()? {
                save_token(state_dir, &imported)?;
                token = Some(imported);
            }
        }
        if operation == "connect" && token.is_none() {
            return device_connect(state_dir, &target, current, tailscale_auth_key);
        }
        if operation == "status" && token.is_none() {
            return Ok(projection(
                &target,
                GitHubConnectionState::Disconnected,
                None,
            ));
        }
        let token = token
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "GITHUB_CREDENTIAL_MISSING"))?;
        let method = if import_gh_token()?.is_some() {
            "gh"
        } else {
            "device"
        };
        match operation {
            "connect" => {
                let key = tailscale_auth_key.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "ACTIONS_TAILSCALE_CREDENTIAL_MISSING",
                    )
                })?;
                bootstrap(&token, &target)?;
                upload_tailscale_secret(&token, &target, key)?;
                Ok(projection(
                    &target,
                    GitHubConnectionState::Connected,
                    Some(method),
                ))
            }
            "start" => {
                bootstrap(&token, &target)?;
                dispatch(&token, &target, "start")
            }
            "stop" => {
                let mut status = workflow_status(&token, &target)?;
                if let Some(id) = status.run_id {
                    cancel(&token, &target, id)?;
                    status.run_state = Some("cancelled".into());
                }
                Ok(status)
            }
            "status" => workflow_status(&token, &target),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "GITHUB_ACTION_INVALID",
            )),
        }
    }
}

#[cfg(windows)]
fn device_connect(
    state_dir: &Path,
    target: &GitHubWorkflowRequest,
    _current: Option<&GitHubStatus>,
    tailscale_auth_key: Option<&str>,
) -> io::Result<GitHubStatus> {
    // The public client id identifies the GnX OAuth application, never a user
    // credential. It is compile-time configuration, not a command-line value.
    let client_id = option_env!("GNX_GITHUB_CLIENT_ID").unwrap_or("Ov23liYQnxCvPWDRkKwV");
    if let Ok(bytes) = std::fs::read(state_dir.join(PENDING_FILE)) {
        if let Ok(pending) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(device_code) = pending.get("device_code").and_then(|v| v.as_str()) {
                let body = format!("client_id={client_id}&device_code={device_code}&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code");
                if let Ok(response) =
                    request("POST", "/login/oauth/access_token", None, Some(&body))
                {
                    if let Ok(oauth) = serde_json::from_slice::<OAuthToken>(&response) {
                        if let Some(token) = oauth.access_token {
                            save_token(state_dir, &token)?;
                            let _ = std::fs::remove_file(state_dir.join(PENDING_FILE));
                            let key = tailscale_auth_key.ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "ACTIONS_TAILSCALE_CREDENTIAL_MISSING",
                                )
                            })?;
                            bootstrap(&token, target)?;
                            upload_tailscale_secret(&token, target, key)?;
                            return Ok(projection(
                                target,
                                GitHubConnectionState::Connected,
                                Some("device"),
                            ));
                        }
                    }
                }
                let mut status = projection(
                    target,
                    GitHubConnectionState::AwaitingAuthorization,
                    Some("device"),
                );
                status.verification_uri = Some("https://github.com/login/device".into());
                status.user_code = pending
                    .get("user_code")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
                status.message = Some("Open the verification URI and enter the displayed code, then run gnx connect again.".into());
                return Ok(status);
            }
        }
    }
    let body = format!("client_id={client_id}&scope=repo%20workflow");
    let response = request("POST", "/login/device/code", None, Some(&body))?;
    let code: DeviceCode = serde_json::from_slice(&response)
        .map_err(|_| io::Error::other("GITHUB_OAUTH_RESPONSE_INVALID"))?;
    let pending = serde_json::json!({ "device_code": code.device_code, "user_code": code.user_code, "expires_in": code.expires_in, "interval": code.interval.unwrap_or(5) });
    std::fs::write(
        state_dir.join(PENDING_FILE),
        serde_json::to_vec(&pending).map_err(io::Error::other)?,
    )?;
    let mut status = projection(
        target,
        GitHubConnectionState::AwaitingAuthorization,
        Some("device"),
    );
    // Do not reflect an arbitrary server-provided URI through the pipe.
    status.verification_uri = Some("https://github.com/login/device".into());
    status.user_code = Some(code.user_code);
    status.message = Some(
        "Open the verification URI and enter the displayed code, then run gnx connect again."
            .into(),
    );
    Ok(status)
}

#[cfg(windows)]
fn import_gh_token() -> io::Result<Option<String>> {
    // Only the service identity's explicitly configured APPDATA is trusted;
    // never scan C:\\Users and implicitly trust another user's credential.
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Ok(None);
    };
    let path = PathBuf::from(appdata).join("GitHub CLI").join("hosts.yml");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("oauth_token:") {
            let token = value.trim().trim_matches('"').trim_matches('\'');
            if !token.is_empty() && token.len() <= 512 && !token.chars().any(char::is_control) {
                return Ok(Some(token.into()));
            }
        }
    }
    Ok(None)
}

#[cfg(windows)]
fn bootstrap(token: &str, target: &GitHubWorkflowRequest) -> io::Result<()> {
    let path = format!(
        "/repos/{}/{}/contents/.github/workflows/{}",
        target.owner, target.repository, target.workflow
    );
    match request("GET", &path, Some(token), None) {
        Ok(_) => Ok(()),
        Err(error) if error.raw_os_error() == Some(404) => {
            // Bootstrap only the reviewed workflow shipped in the installed
            // payload. Setup cannot provide source, paths, or shell fragments.
            let workflow_path = crate::linux_node::payload_dir()
                .join("actions")
                .join(WORKFLOW);
            let content = std::fs::read(&workflow_path).map_err(|_| {
                io::Error::new(io::ErrorKind::NotFound, "ACTIONS_WORKFLOW_PAYLOAD_MISSING")
            })?;
            let encoded = base64_encode(&content);
            let body = format!(
                r#"{{"message":"Install GnX workflow","content":"{encoded}","branch":"{}"}}"#,
                target.reference
            );
            request("PUT", &path, Some(token), Some(&body)).map(|_| ())
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn upload_tailscale_secret(
    token: &str,
    target: &GitHubWorkflowRequest,
    tailscale_auth_key: &str,
) -> io::Result<()> {
    #[derive(Deserialize)]
    struct PublicKey {
        key: String,
        key_id: String,
    }
    let key_path = format!(
        "/repos/{}/{}/actions/secrets/public-key",
        target.owner, target.repository
    );
    let bytes = request("GET", &key_path, Some(token), None)?;
    let public: PublicKey = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::other("GITHUB_PUBLIC_KEY_INVALID"))?;
    let public_bytes =
        base64_decode(&public.key).ok_or_else(|| io::Error::other("GITHUB_PUBLIC_KEY_INVALID"))?;
    if public_bytes.len() != 32 {
        return Err(io::Error::other("GITHUB_PUBLIC_KEY_INVALID"));
    }
    sodiumoxide::init().map_err(|_| io::Error::other("GITHUB_CRYPTO_INIT_FAILED"))?;
    let public_key = sodiumoxide::crypto::box_::PublicKey::from_slice(&public_bytes)
        .ok_or_else(|| io::Error::other("GITHUB_PUBLIC_KEY_INVALID"))?;
    let sealed = sodiumoxide::crypto::sealedbox::seal(tailscale_auth_key.as_bytes(), &public_key);
    let encrypted = base64_encode(&sealed);
    let body = serde_json::json!({
        "encrypted_value": encrypted,
        "key_id": public.key_id,
    });
    let secret_path = format!(
        "/repos/{}/{}/actions/secrets/{}",
        target.owner, target.repository, ACTIONS_SECRET_NAME
    );
    let body = serde_json::to_string(&body).map_err(io::Error::other)?;
    request("PUT", &secret_path, Some(token), Some(&body)).map(|_| ())
}

#[cfg(windows)]
fn dispatch(
    token: &str,
    target: &GitHubWorkflowRequest,
    command: &str,
) -> io::Result<GitHubStatus> {
    let path = format!(
        "/repos/{}/{}/actions/workflows/{}/dispatches",
        target.owner, target.repository, target.workflow
    );
    let body =
        serde_json::json!({"ref": target.reference, "inputs": {"lease_minutes": "60"}}).to_string();
    request("POST", &path, Some(token), Some(&body))?;
    let mut status = projection(target, GitHubConnectionState::Connected, Some("stored"));
    status.run_state = Some("queued".into());
    status.message = Some(format!("GitHub workflow {command} dispatched."));
    Ok(status)
}

#[cfg(windows)]
fn workflow_status(token: &str, target: &GitHubWorkflowRequest) -> io::Result<GitHubStatus> {
    let path = format!(
        "/repos/{}/{}/actions/workflows/{}/runs?per_page=1",
        target.owner, target.repository, target.workflow
    );
    let bytes = request("GET", &path, Some(token), None)?;
    let runs: Runs =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("GITHUB_STATUS_INVALID"))?;
    let mut status = projection(target, GitHubConnectionState::Connected, Some("stored"));
    if let Some(run) = runs.workflow_runs.into_iter().next() {
        status.run_id = Some(run.id);
        status.run_state = run.status.map(|value| sanitize_state(&value));
        status.conclusion = run.conclusion.map(|value| sanitize_state(&value));
    }
    Ok(status)
}

#[cfg(windows)]
fn cancel(token: &str, target: &GitHubWorkflowRequest, run_id: u64) -> io::Result<()> {
    let path = format!(
        "/repos/{}/{}/actions/runs/{run_id}/cancel",
        target.owner, target.repository
    );
    request("POST", &path, Some(token), None).map(|_| ())
}

fn sanitize_state(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(32)
        .collect()
}

fn base64_decode(value: &str) -> Option<Vec<u8>> {
    fn digit(value: u8) -> Option<u8> {
        match value {
            b'A'..=b'Z' => Some(value - b'A'),
            b'a'..=b'z' => Some(value - b'a' + 26),
            b'0'..=b'9' => Some(value - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = value.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let a = digit(chunk[0])? as u32;
        let b = digit(chunk[1])? as u32;
        let c = if chunk[2] == b'=' {
            0
        } else {
            digit(chunk[2])? as u32
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            digit(chunk[3])? as u32
        };
        out.push(((a << 2) | (b >> 4)) as u8);
        if chunk[2] != b'=' {
            out.push(((b << 4) | (c >> 2)) as u8);
        }
        if chunk[3] != b'=' {
            out.push(((c << 6) | d) as u8);
        }
    }
    Some(out)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as usize;
        let b = chunk.get(1).copied().unwrap_or(0) as usize;
        let c = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(TABLE[a >> 2] as char);
        out.push(TABLE[((a & 3) << 4) | (b >> 4)] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((b & 15) << 2) | (c >> 6)] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[c & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(windows)]
fn token_path(state_dir: &Path) -> PathBuf {
    state_dir.join(TOKEN_FILE)
}

#[cfg(windows)]
fn save_token(state_dir: &Path, token: &str) -> io::Result<()> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: token.len() as u32,
        pbData: token.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    if unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            &mut output,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as _) };
    std::fs::create_dir_all(state_dir)?;
    let path = token_path(state_dir);
    std::fs::write(&path, bytes)?;
    crate::dedicated_account::restrict_system_admin_directory(state_dir)?;
    Ok(())
}

#[cfg(windows)]
fn load_token(state_dir: &Path) -> io::Result<Option<String>> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let path = token_path(state_dir);
    let bytes = match std::fs::read(path) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    if unsafe {
        CryptUnprotectData(
            &mut input,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            &mut output,
        )
    } == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "GITHUB_TOKEN_UNPROTECT_FAILED",
        ));
    }
    let clear =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as _) };
    String::from_utf8(clear)
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "GITHUB_TOKEN_INVALID"))
}

#[cfg(windows)]
fn request(
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> io::Result<Vec<u8>> {
    // WinHTTP is a native OS HTTPS client; no listener, shell, or arbitrary
    // host is involved. Only api.github.com and github.com OAuth endpoints are allowed.
    use std::ptr::null_mut;
    use windows_sys::Win32::Networking::WinHttp::*;
    let wide = |value: &str| {
        value
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let agent = wide("GnXHostAgent/1");
    let host = wide(if path.starts_with("/login/") {
        AUTH_HOST
    } else {
        API_HOST
    });
    let object = wide(path);
    let verb = wide(method);
    let session = unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            null_mut(),
            null_mut(),
            0,
        )
    };
    if session.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let connection = unsafe { WinHttpConnect(session, host.as_ptr(), API_PORT, 0) };
        if connection.is_null() {
            return Err(io::Error::last_os_error());
        }
        let request = unsafe {
            WinHttpOpenRequest(
                connection,
                verb.as_ptr(),
                object.as_ptr(),
                null_mut(),
                null_mut(),
                null_mut(),
                WINHTTP_FLAG_SECURE,
            )
        };
        if request.is_null() {
            unsafe { WinHttpCloseHandle(connection) };
            return Err(io::Error::last_os_error());
        }
        let content_type = if path.starts_with("/login/") {
            "application/x-www-form-urlencoded"
        } else {
            "application/json"
        };
        let headers = wide(&format!("Accept: application/vnd.github+json\r\nUser-Agent: GnXHostAgent/1\r\nContent-Type: {content_type}\r\n"));
        // Add the bearer header only in the native WinHTTP call; it is never
        // formatted into an error or log string.
        let auth = token.map(|t| wide(&format!("Accept: application/vnd.github+json\r\nUser-Agent: GnXHostAgent/1\r\nContent-Type: {content_type}\r\nAuthorization: Bearer {t}\r\n"))).unwrap_or(headers);
        let bytes = body.map(str::as_bytes).unwrap_or_default();
        let ok = unsafe {
            WinHttpSendRequest(
                request,
                auth.as_ptr(),
                u32::MAX,
                bytes.as_ptr() as *mut _,
                bytes.len() as u32,
                bytes.len() as u32,
                0,
            )
        } != 0;
        if !ok {
            unsafe {
                WinHttpCloseHandle(request);
                WinHttpCloseHandle(connection)
            };
            return Err(io::Error::last_os_error());
        }
        if unsafe { WinHttpReceiveResponse(request, null_mut()) } == 0 {
            unsafe {
                WinHttpCloseHandle(request);
                WinHttpCloseHandle(connection)
            };
            return Err(io::Error::last_os_error());
        }
        let mut status = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let _ = unsafe {
            WinHttpQueryHeaders(
                request,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                null_mut(),
                &mut status as *mut _ as *mut _,
                &mut size,
                null_mut(),
            )
        };
        let mut out = Vec::new();
        loop {
            let mut available = 0u32;
            if unsafe { WinHttpQueryDataAvailable(request, &mut available) } == 0 || available == 0
            {
                break;
            }
            let mut chunk = vec![0u8; available as usize];
            let mut got = 0u32;
            if unsafe {
                WinHttpReadData(request, chunk.as_mut_ptr() as *mut _, available, &mut got)
            } == 0
            {
                break;
            }
            out.extend_from_slice(&chunk[..got as usize]);
        }
        unsafe {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connection)
        };
        if status >= 400 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(out)
    })();
    unsafe { WinHttpCloseHandle(session) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_never_contains_credentials() {
        let target = GitHubWorkflowRequest {
            owner: "o".into(),
            repository: "r".into(),
            workflow: WORKFLOW.into(),
            reference: REF.into(),
        };
        let status = projection(&target, GitHubConnectionState::Connected, Some("gh"));
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("token"));
        assert!(!json.contains("Authorization"));
    }
    #[test]
    fn bootstrap_encoding_is_deterministic() {
        assert_eq!(base64_encode(b"GnX"), "R25Y");
    }
}
