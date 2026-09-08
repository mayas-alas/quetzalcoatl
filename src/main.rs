use clap::{Parser, Subcommand, ValueEnum};
use gnx::{
    Result,
    config::Config,
    plan,
    report::{self, Failure},
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use serde_json::Value;
use serde_json::json;
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    version,
    about = "GNX: private access, HTTPS control and persistent compute"
)]
struct Cli {
    #[arg(long, global = true, default_value = "gnx.toml")]
    config: PathBuf,
    #[arg(long, global = true, default_value = "GNX")]
    distribution: String,
    #[arg(long, global = true, default_value = "/usr/local/bin/gnx")]
    linux_binary: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write a starter configuration; never overwrite an existing file.
    Init,
    /// Validate intent and preview changes. Does not start services.
    Plan {
        #[arg(long)]
        render: bool,
    },
    Doctor,
    Apply,
    Status,
    Compute {
        #[command(subcommand)]
        operation: Operation,
    },
    Access {
        #[command(subcommand)]
        operation: AccessOperation,
    },
    Control {
        #[command(subcommand)]
        operation: Operation,
    },
    #[command(hide = true)]
    Invoke,
    #[cfg(target_os = "linux")]
    #[command(hide = true)]
    Probe {
        #[arg(long)]
        ip: std::net::Ipv4Addr,
        #[arg(long)]
        ca: PathBuf,
    },
}

#[derive(Subcommand)]
enum Operation {
    Apply,
    Status,
}

#[derive(Subcommand)]
enum AccessOperation {
    Apply,
    Status,
    Enroll,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Action {
    Doctor,
    Apply,
    Status,
    Enroll,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    All,
    Compute,
    Access,
    Control,
}

#[cfg(target_os = "linux")]
impl Scope {
    fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Compute => "compute",
            Self::Access => "access",
            Self::Control => "control",
        }
    }
}

#[cfg(target_os = "linux")]
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Apply => "apply",
            Self::Status => "status",
            Self::Enroll => "enroll",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    config: Config,
    action: Action,
    scope: Scope,
    credential: Option<String>,
}

fn input(limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::io::stdin().take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Failure::new(
            "INPUT_LIMIT",
            "Input exceeds the bounded request size",
        ));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn execute(request: &Request) -> Result<Value> {
    request.config.validate()?;
    let scope = request.scope.name();
    match request.action {
        Action::Doctor => gnx::runtime::doctor(&request.config),
        Action::Apply => gnx::runtime::apply(&request.config, scope),
        Action::Status => gnx::runtime::status(&request.config, scope),
        Action::Enroll if matches!(request.scope, Scope::Access) => gnx::runtime::enroll(
            &request.config,
            request.credential.as_deref().unwrap_or_default().as_bytes(),
        ),
        _ => Err(Failure::new(
            "REQUEST",
            "Invalid action and capability combination",
        )),
    }
}

#[cfg(target_os = "windows")]
fn bridge(cli: &Cli, request: &Request) -> Result<u8> {
    use std::process::{Command, Stdio};
    if !cli.linux_binary.starts_with('/')
        || cli
            .linux_binary
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && !b"/_.-".contains(&b))
    {
        return Err(Failure::new(
            "LINUX_BINARY",
            "Use an absolute Linux binary path",
        ));
    }
    // Fixed argv and typed stdin: no shell, interpolation or secret command arguments.
    let mut child = Command::new("wsl.exe")
        .args([
            "--distribution",
            &cli.distribution,
            "--user",
            "root",
            "--exec",
            &cli.linux_binary,
            "invoke",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|_| {
            Failure::action(
                "RUNTIME_MISSING",
                "Install the dedicated GNX Linux runtime and GNX binary",
            )
        })?;
    let payload = serde_json::to_vec(request).expect("serializable request");
    if let Err(e) = child.stdin.take().expect("piped stdin").write_all(&payload) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.into());
    }
    Ok(child.wait()?.code().unwrap_or(1).clamp(0, 255) as u8)
}

fn run(cli: Cli) -> Result<u8> {
    if matches!(cli.command, Commands::Init) {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&cli.config)?;
        file.write_all(include_bytes!("../config/gnx.example.toml"))?;
        return Ok(report::output(
            "all",
            "init",
            Ok(json!({"config":cli.config})),
        ));
    }
    #[cfg(target_os = "linux")]
    if let Commands::Probe { ip, ref ca } = cli.command {
        return Ok(report::output(
            "control",
            "probe",
            gnx::runtime::probe(ip, ca),
        ));
    }
    if matches!(cli.command, Commands::Invoke) {
        let bytes = input(131072)?;
        let request: Request = serde_json::from_slice(&bytes)
            .map_err(|_| Failure::new("REQUEST", "Invalid typed request"))?;
        request.config.validate()?;
        #[cfg(target_os = "linux")]
        return Ok(report::output(
            request.scope.name(),
            request.action.name(),
            execute(&request),
        ));
        #[cfg(not(target_os = "linux"))]
        return Err(Failure::new(
            "PLATFORM",
            "invoke is reserved for the Linux runtime",
        ));
    }
    let config = Config::read(&cli.config)?;
    if let Commands::Plan { render } = cli.command {
        let artifacts = plan::render(&config, "all")?;
        let changes = plan::changes(&config, &artifacts)?;
        let mut details = json!({"revision":config.revision(),"changes":changes,"applied":false,
            "pending_identity_ip":config.network.identity_ip.is_none()});
        if render {
            details["artifacts"] = serde_json::to_value(artifacts).unwrap();
        }
        return Ok(report::output("all", "plan", Ok(details)));
    }
    let (action, scope) = match cli.command {
        Commands::Doctor => (Action::Doctor, Scope::All),
        Commands::Apply => (Action::Apply, Scope::All),
        Commands::Status => (Action::Status, Scope::All),
        Commands::Compute { operation: ref op } => (operation_ref(op), Scope::Compute),
        Commands::Control { operation: ref op } => (operation_ref(op), Scope::Control),
        Commands::Access { operation: ref op } => (
            match op {
                AccessOperation::Apply => Action::Apply,
                AccessOperation::Status => Action::Status,
                AccessOperation::Enroll => Action::Enroll,
            },
            Scope::Access,
        ),
        _ => unreachable!(),
    };
    let credential = if matches!(action, Action::Enroll) {
        Some(
            String::from_utf8(input(4096)?)
                .map_err(|_| Failure::new("ENROLL_INPUT", "Credential encoding must be UTF-8"))?,
        )
    } else {
        None
    };
    let request = Request {
        config,
        action,
        scope,
        credential,
    };
    #[cfg(target_os = "linux")]
    return Ok(report::output(
        scope.name(),
        action.name(),
        execute(&request),
    ));
    #[cfg(target_os = "windows")]
    return bridge(&cli, &request);
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    Err(Failure::new(
        "PLATFORM",
        "Live operations require Linux or Windows with the GNX runtime",
    ))
}

fn operation_ref(op: &Operation) -> Action {
    match op {
        Operation::Apply => Action::Apply,
        Operation::Status => Action::Status,
    }
}

fn main() -> ExitCode {
    ExitCode::from(match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => report::output("all", "dispatch", Err(error)),
    })
}
