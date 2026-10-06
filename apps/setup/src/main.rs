//! Minimal console onboarding. Only the Windows service performs privileged work.
mod agent_client;

use gnx_control_protocol::{NodeState, Operation, Progress, Request, Response, PROTOCOL_VERSION};
use std::io::{self, IsTerminal, Write};
use uuid::Uuid;
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Password,
    MeshKey,
    Retry,
    Upgrade,
    Stop,
}

fn main() {
    let (result, pause) = match parse_args(std::env::args().skip(1)) {
        Ok((command, pause)) => (run(command), pause),
        Err(error) => (Err(error), false),
    };
    if let Err(error) = &result {
        eprintln!("Setup failed: {error}");
    }
    if pause && io::stdin().is_terminal() && io::stdout().is_terminal() {
        print!("Press Enter to close...");
        let _ = io::stdout().flush();
        let mut ignored = Zeroizing::new(String::new());
        let _ = io::stdin().read_line(&mut ignored);
    }
    if result.is_err() {
        std::process::exit(1);
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<(&'static str, bool)> {
    let mut words = Vec::new();
    let mut pause = false;
    for arg in args {
        if arg == "--pause" && !pause {
            pause = true;
        } else {
            words.push(arg);
        }
    }
    let command = match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["setup"] => "setup",
        ["help" | "--help" | "-h"] => "help",
        ["version" | "--version" | "-v"] => "version",
        ["status"] => "status",
        ["connect"] | ["actions", "connect"] => "github-connect",
        ["start"] | ["actions", "start"] => "github-start",
        ["stop"] | ["actions", "stop"] => "github-stop",
        ["actions", "status"] | ["github", "status"] => "github-status",
        ["provision"] | ["password"] | ["set", "password"] => "password",
        ["join"] => "join",
        ["retry"] => "retry",
        ["upgrade", "--confirm"] => "upgrade",
        ["runtime", "stop", "--confirm"] | ["shutdown", "--confirm"] => "runtime-stop",
        ["runtime", "start"] => "runtime-start",
        // Keep the legacy aliases, but name the shared machine correctly.
        ["workload", "private-services" | "rustfs", "start"] => "rustfs-start",
        ["workload", "private-services" | "rustfs", "stop", "--confirm"] => "rustfs-stop",
        ["diagnose", "compute"] => "diagnose-compute",
        ["gnx://workloads/private-services/start" | "gnx://workloads/rustfs/start"] => "rustfs-start-uri",
        ["gnx://workloads/private-services/stop" | "gnx://workloads/rustfs/stop"] => "rustfs-stop-uri",
        ["cancel"] => "cancel",
        _ => {
            return Err(
                "invalid command line; use gnx help (never pass credentials as arguments)".into(),
            )
        }
    };
    Ok((command, pause))
}

fn run(command: &str) -> Result<()> {
    match command {
        "help" => {
            println!("GnX Node Setup {}", env!("CARGO_PKG_VERSION"));
            println!(
                "Usage: gnx [setup|status|connect|start|stop|actions status|set password|join|retry|upgrade --confirm|runtime stop --confirm|runtime start|workload private-services start|workload private-services stop --confirm|cancel|help|version] [--pause]"
            );
            println!("Run in an Administrator terminal. No command: guided initial setup.");
            println!(
                "Proxmox password, confirmation, then Tailscale auth key: hidden prompts only."
            );
            println!(
                "set password: securely set or rotate the Proxmox root password with hidden prompts."
            );
            println!(
                "status reads the node checkpoint and desired/applied runtime state without registry calls or mutations; actions status/connect/start/stop use the native GitHub Actions integration."
            );
            println!("Existing ready nodes are preserved. Daily access: https://app.gnx");
        }
        "version" => println!("gnx {}", env!("CARGO_PKG_VERSION")),
        "status" => {
            print_progress(&get_progress()?);
        }
        "github-connect" => {
            finish(connect_github()?)?;
        }
        "github-start" => {
            finish(send(&request(Operation::Start))?)?;
        }
        "github-stop" => {
            finish(send(&request(Operation::Stop))?)?;
        }
        "github-status" => {
            finish(send(&request(Operation::Status))?)?;
        }
        "upgrade" => {
            println!("Applying installed runtime update; compute and VMs/LXC may restart...");
            finish(send(&request(Operation::Retry))?)?;
        }
        "retry" => {
            let current = get_progress()?;
            if current.error_code.as_deref() == Some("RUNTIME_UPDATE_PENDING") {
                print_progress(&current);
                return Err("runtime update requires gnx upgrade --confirm during a maintenance window".into());
            }
            finish(send(&request(Operation::Retry))?)?;
        }
        "cancel" => {
            finish(send(&request(Operation::Cancel))?)?;
        }
        "runtime-stop" => {
            finish(send(&request(Operation::StopRuntime))?)?;
        }
        "runtime-start" => {
            finish(send(&request(Operation::StartRuntime))?)?;
        }
        "rustfs-start" => {
            finish(send(&workload_request(Operation::StartWorkload))?)?;
        }
        "rustfs-stop" => {
            finish(send(&workload_request(Operation::StopWorkload))?)?;
        }
        "diagnose-compute" => {
            finish(send(&request(Operation::DiagnoseCompute))?)?;
        }
        "rustfs-start-uri" | "rustfs-stop-uri" => {
            let starting = command == "rustfs-start-uri";
            let verb = if starting { "start" } else { "stop" };
            println!("This controls the shared LXC: storage, Pocket ID and NetBird. Stopping it disables new PWA logins; recovery uses gnx workload private-services start.");
            print!("Confirm {verb} private services? [y/N]: ");
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !matches!(
                answer.trim().to_ascii_lowercase().as_str(),
                "y" | "yes" | "s" | "si" | "sí"
            ) {
                return Err("operation cancelled; no change was sent".into());
            }
            let operation = if starting {
                Operation::StartWorkload
            } else {
                Operation::StopWorkload
            };
            finish(send(&workload_request(operation))?)?;
            println!("You may return to app.gnx; its confirmed state will refresh automatically.");
        }
        "password" => {
            provision()?;
        }
        "join" => {
            join_mesh()?;
        }
        _ => interactive()?,
    }
    Ok(())
}

fn request(operation: Operation) -> Request {
    Request {
        version: PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        operation,
        tailscale_auth_key: None,
        proxmox_password: None,
        secret_file: None,
        github: None,
        workload: None,
    }
}

fn workload_request(operation: Operation) -> Request {
    let mut request = request(operation);
    // Stable ownership/IPC key of the existing shared guest; never adopt a new guest.
    request.workload = Some("rustfs".into());
    request
}

fn has_enrollment(progress: &Progress) -> bool {
    progress.node_private_ip.is_some() || progress.tailscale_ip.is_some()
}

fn next_action(progress: &Progress) -> Action {
    if progress.requires_restart || progress.state == NodeState::RebootRequired {
        return Action::Stop;
    }
    if progress.error_code.as_deref() == Some("RUNTIME_UPDATE_PENDING") {
        return Action::Upgrade;
    }
    if progress.state == NodeState::NodeReady {
        return Action::Stop;
    }
    // A verifier failure is not evidence that existing credentials are missing.
    if has_enrollment(progress) {
        return Action::Retry;
    }
    // Provisioning credentials are one-shot. A fresh node blocked in a
    // Windows/account/CA phase must ask again instead of first issuing a
    // secretless Retry that can only report PROXMOX_CREDENTIAL_MISSING.
    if progress.state == NodeState::Blocked
        && matches!(progress.phase.as_str(), "windows" | "account" | "ca")
    {
        return Action::Password;
    }
    match progress.error_code.as_deref() {
        Some("MESH_CREDENTIAL_MISSING") => return Action::MeshKey,
        Some("PROXMOX_CREDENTIAL_MISSING") => return Action::Password,
        Some(_) => return Action::Retry,
        None => {}
    }
    match progress.state {
        NodeState::New => Action::Password,
        NodeState::RuntimeReady | NodeState::MeshPending => Action::MeshKey,
        NodeState::Blocked | NodeState::Failed | NodeState::RecoveryRequired => Action::Retry,
        _ => Action::Stop,
    }
}

fn interactive() -> Result<()> {
    let current = get_progress()?;
    print_progress(&current);
    match next_action(&current) {
        Action::Password => {
            let progress = provision()?;
            if next_action(&progress) == Action::MeshKey {
                join_mesh()?;
            }
        }
        Action::MeshKey => {
            join_mesh()?;
        }
        Action::Retry => {
            finish(send(&request(Operation::Retry))?)?;
        }
        Action::Upgrade => {
            require_terminal()?;
            print!("Apply installed update? Compute and VMs/LXC may restart. [y/N]: ");
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes" | "s" | "si" | "sí") {
                finish(send(&request(Operation::Retry))?)?;
            } else {
                println!("Update remains pending; runtime was not changed.");
            }
        }
        Action::Stop => {
            check_error(&current)?;
        }
    }
    Ok(())
}

fn provision() -> Result<Progress> {
    let current = get_progress()?;
    let updating_existing = has_enrollment(&current) || current.state == NodeState::NodeReady;
    if current.requires_restart
        || !matches!(
            current.state,
            NodeState::New
                | NodeState::Blocked
                | NodeState::Failed
                | NodeState::RecoveryRequired
                | NodeState::NodeReady
        )
    {
        return Err(
            "checkpoint does not allow setting the Proxmox password; use gnx status first".into(),
        );
    }
    require_terminal()?;
    if updating_existing {
        println!("Password rotation recreates compute; VMs/LXC may restart. Use a maintenance window.");
    }
    let password = prompt_secret("Proxmox password: ")?;
    if password.is_empty() || password.len() > 1024 || password.chars().any(char::is_control) {
        return Err("invalid Proxmox password; nothing was sent".into());
    }
    let confirmation = prompt_secret("Confirm Proxmox password: ")?;
    if password != confirmation {
        return Err("Proxmox passwords do not match; nothing was sent".into());
    }
    println!("Preparing Windows and the Linux runtime; this can take several minutes...");
    let progress = finish(send_secrets(
        &request(Operation::Provision),
        None,
        Some(&password),
    )?)?;
    if updating_existing {
        println!("Proxmox password updated; reconciling the enrolled runtime...");
        finish(send(&request(Operation::Retry))?)
    } else {
        Ok(progress)
    }
}

fn connect_github() -> Result<Response> {
    require_terminal()?;
    let key = prompt_secret("Tailscale auth key (tskey-auth-*): ")?;
    if !key.starts_with("tskey-auth-")
        || key.len() <= "tskey-auth-".len()
        || key.len() > 512
        || key.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err("invalid Tailscale key; nothing was sent".into());
    }
    println!("Connecting GitHub Actions and securely updating the reusable secret...");
    send_secrets(&request(Operation::Connect), Some(&key), None)
}

fn join_mesh() -> Result<Progress> {
    let current = get_progress()?;
    if current.state == NodeState::NodeReady {
        print_progress(&current);
        check_error(&current)?;
        return Ok(current);
    }
    if current.requires_restart
        || !matches!(
            current.state,
            NodeState::RuntimeReady | NodeState::MeshPending | NodeState::Blocked
        )
    {
        return Err(
            "checkpoint does not allow mesh enrollment; run initial setup or status first".into(),
        );
    }
    // Explicit join may renew an interrupted enrollment, never implicitly reset a password.
    if has_enrollment(&current) {
        return Err("node already has an enrollment; use retry, not a new key".into());
    }
    require_terminal()?;
    let key = prompt_secret("Tailscale auth key (tskey-auth-*): ")?;
    if !key.starts_with("tskey-auth-")
        || key.len() <= "tskey-auth-".len()
        || key.len() > 512
        || key.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err("invalid Tailscale key; nothing was sent".into());
    }
    println!("Joining the private network and verifying the node...");
    finish(send_secrets(
        &request(Operation::JoinMesh),
        Some(&key),
        None,
    )?)
}

fn get_progress() -> Result<Progress> {
    // A read-only snapshot may describe a failed checkpoint; the caller decides recovery.
    Ok(send(&request(Operation::GetProgress))?.progress)
}

fn check_error(progress: &Progress) -> Result<()> {
    if let Some(code) = &progress.error_code {
        return Err(format!("host operation failed ({code}); use status for details").into());
    }
    Ok(())
}

fn finish(response: Response) -> Result<Progress> {
    print_progress(&response.progress);
    check_error(&response.progress)?;
    Ok(response.progress)
}

#[cfg(windows)]
fn send(req: &Request) -> Result<Response> {
    agent_client::request(req).map_err(|e| explain_pipe(e).into())
}
#[cfg(not(windows))]
fn send(_req: &Request) -> Result<Response> {
    Err("Setup requires Windows and the installed GnX host service".into())
}
#[cfg(windows)]
fn send_secrets(req: &Request, key: Option<&str>, password: Option<&str>) -> Result<Response> {
    agent_client::request_with_secrets(req, key, password).map_err(|e| explain_pipe(e).into())
}
#[cfg(not(windows))]
fn send_secrets(_req: &Request, _key: Option<&str>, _password: Option<&str>) -> Result<Response> {
    Err("Setup requires Windows; no secret was staged".into())
}

fn print_progress(progress: &Progress) {
    println!(
        "[{}] {:?}: {}",
        progress.phase, progress.state, progress.message
    );
    if let Some(code) = &progress.error_code {
        println!("Error code: {code}");
    }
    if progress.requires_restart || progress.state == NodeState::RebootRequired {
        println!("Restart required; restart Windows, then run gnx again.");
    }
    if let Some(ip) = progress
        .node_private_ip
        .as_deref()
        .or(progress.tailscale_ip.as_deref())
    {
        println!("Node private IP: {ip}");
    }
    if let Some(ip) = progress
        .pihole_private_ip
        .as_deref()
        .or(progress.pihole_ip.as_deref())
    {
        println!("Pi-hole private IP: {ip}");
    }
    for action in &progress.required_actions {
        println!(
            "Action {:?}: {}{}",
            action.kind,
            action.target,
            action
                .value
                .as_deref()
                .map(|v| format!(" ({v})"))
                .unwrap_or_default()
        );
    }
    if let Some(github) = &progress.github {
        println!("GitHub Actions: {:?}", github.connection);
        if let (Some(uri), Some(code)) = (&github.verification_uri, &github.user_code) {
            println!("Authorize at {uri} with code {code}, then run gnx connect again.");
        }
        if let Some(run) = github.run_id {
            println!("Workflow run {run}: {}{}", github.run_state.as_deref().unwrap_or("unknown"), github.conclusion.as_deref().map(|value| format!(" ({value})")).unwrap_or_default());
        }
    }
    if progress.state == NodeState::NodeReady {
        println!("Node already configured; credentials preserved.");
    }
    println!("Daily access: https://app.gnx | Compute: https://compute.gnx");
}

fn require_terminal() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "onboarding requires an Administrator console; redirected secret input is not accepted"
                .into(),
        );
    }
    Ok(())
}

#[cfg(windows)]
fn prompt_secret(prompt: &str) -> Result<Zeroizing<String>> {
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, ReadConsoleW, SetConsoleMode, ENABLE_ECHO_INPUT,
        STD_INPUT_HANDLE,
    };
    struct ModeGuard {
        handle: HANDLE,
        original: u32,
    }
    impl Drop for ModeGuard {
        fn drop(&mut self) {
            unsafe {
                SetConsoleMode(self.handle, self.original);
            }
        }
    }
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if input.is_null() || input == INVALID_HANDLE_VALUE {
        return Err("no interactive console available".into());
    }
    let mut original = 0u32;
    if unsafe { GetConsoleMode(input, &mut original) } == 0 {
        return Err("standard input is not a console".into());
    }
    if unsafe { SetConsoleMode(input, original & !ENABLE_ECHO_INPUT) } == 0 {
        return Err("could not disable console echo; secret was not requested".into());
    }
    let _mode = ModeGuard {
        handle: input,
        original,
    };
    print!("{prompt}");
    io::stdout().flush()?;
    let mut buffer = Zeroizing::new([0u16; 4096]);
    let mut read = 0u32;
    let ok = unsafe {
        ReadConsoleW(
            input,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            &mut read,
            std::ptr::null_mut(),
        )
    };
    println!();
    if ok == 0 || read == 0 {
        return Err("secret entry cancelled; nothing was sent".into());
    }
    if read as usize >= buffer.len() {
        return Err("secret is too long; nothing was sent".into());
    }
    while read > 0 && matches!(buffer[(read - 1) as usize], 10 | 13) {
        read -= 1;
    }
    Ok(Zeroizing::new(String::from_utf16(
        &buffer[..read as usize],
    )?))
}
#[cfg(not(windows))]
fn prompt_secret(_prompt: &str) -> Result<Zeroizing<String>> {
    Err("hidden prompts require Windows".into())
}

#[cfg(windows)]
fn explain_pipe(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::TimedOut => {
            "GnX service is unavailable; start or repair GnXHostAgent".into()
        }
        io::ErrorKind::PermissionDenied => {
            "access denied: run the installed Setup from an Administrator terminal".into()
        }
        _ => format!("unable to contact the GnX host agent: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn progress(state: NodeState, error: Option<&str>) -> Progress {
        Progress {
            request_id: None,
            operation_id: Uuid::nil(),
            sequence: 0,
            phase: "test".into(),
            state,
            message: "safe".into(),
            error_code: error.map(str::to_owned),
            retryable: true,
            requires_restart: false,
            percent: None,
            node_private_ip: None,
            pihole_private_ip: None,
            tailscale_ip: None,
            pihole_ip: None,
            required_actions: vec![],
            private_ca_certificate: None,
            github: None,
        }
    }
    #[test]
    fn fresh_password_then_mesh() {
        assert_eq!(
            next_action(&progress(NodeState::New, None)),
            Action::Password
        );
        assert_eq!(
            next_action(&progress(NodeState::MeshPending, None)),
            Action::MeshKey
        );
        assert_eq!(
            next_action(&progress(NodeState::RuntimeReady, None)),
            Action::MeshKey
        );
    }
    #[test]
    fn pending_upgrade_never_rekeys_or_silently_restarts() {
        let p = progress(NodeState::Blocked, Some("RUNTIME_UPDATE_PENDING"));
        assert_eq!(next_action(&p), Action::Upgrade);
        assert_eq!(parse_args(["upgrade".into(), "--confirm".into()]).unwrap(), ("upgrade", false));
        assert!(parse_args(["upgrade".into()]).is_err());
    }
    #[test]
    fn ready_preserves_even_with_stale_missing_error() {
        assert_eq!(
            next_action(&progress(
                NodeState::NodeReady,
                Some("PROXMOX_CREDENTIAL_MISSING")
            )),
            Action::Stop
        );
    }
    #[test]
    fn fresh_windows_failure_prompts_for_the_one_shot_password_again() {
        let mut p = progress(NodeState::Blocked, Some("ACCESS_DENIED"));
        p.phase = "windows".into();
        assert_eq!(next_action(&p), Action::Password);
    }

    #[test]
    fn blocked_runtime_failure_does_not_rekey() {
        assert_eq!(
            next_action(&progress(NodeState::Blocked, Some("RUNTIME_FAILED"))),
            Action::Retry
        );
        let mut p = progress(NodeState::Blocked, Some("MESH_CREDENTIAL_MISSING"));
        p.node_private_ip = Some("100.64.0.1".into());
        assert_eq!(next_action(&p), Action::Retry);
    }
    #[test]
    fn restart_and_in_progress_checkpoints_do_not_provision() {
        for state in [
            NodeState::RebootRequired,
            NodeState::WindowsReady,
            NodeState::WslReady,
        ] {
            assert_eq!(next_action(&progress(state, None)), Action::Stop);
        }
        let mut p = progress(NodeState::New, None);
        p.requires_restart = true;
        assert_eq!(next_action(&p), Action::Stop);
    }
    #[test]
    fn host_failure_has_nonzero_result() {
        assert!(check_error(&progress(NodeState::Blocked, Some("RUNTIME_FAILED"))).is_err());
    }
    #[test]
    fn parses_commands_and_pause_without_secret_flags() {
        assert_eq!(parse_args(Vec::new()).unwrap(), ("setup", false));
        assert_eq!(parse_args(["--pause".into()]).unwrap(), ("setup", true));
        assert_eq!(
            parse_args(["status".into(), "--pause".into()]).unwrap(),
            ("status", true)
        );
        assert_eq!(
            parse_args(["set".into(), "password".into()]).unwrap(),
            ("password", false)
        );
        assert_eq!(
            parse_args(["password".into()]).unwrap(),
            ("password", false)
        );
        for name in ["private-services", "rustfs"] {
            assert_eq!(parse_args(["workload", name, "start"].map(str::to_owned)).unwrap(), ("rustfs-start", false));
        }
        assert_eq!(parse_args(["gnx://workloads/private-services/stop".into()]).unwrap(), ("rustfs-stop-uri", false));
        assert!(parse_args(["workload", "arbitrary", "start"].map(str::to_owned)).is_err());
        for args in [
            vec!["--password", "do-not-print"],
            vec!["join", "tskey-auth-do-not-print"],
            vec!["set", "password", "do-not-print"],
            vec!["--pause", "--pause"],
        ] {
            let error = parse_args(args.into_iter().map(str::to_owned))
                .unwrap_err()
                .to_string();
            assert!(!error.contains("do-not-print"));
        }
    }
}
