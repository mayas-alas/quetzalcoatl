use gnx::{
    config::Config,
    domain::secret::Secret,
    report::{Report, State},
};
use std::io::{IsTerminal, Read};
fn failure(op: &str, code: &str) -> Report {
    Report::new(
        op,
        State::Failed,
        code,
        Some("Use gnx doctor|plan|apply|status [--config FILE]."),
    )
}
fn execute(op: &str, input: &str, secret: Option<&Secret>) -> Report {
    let c = match Config::parse(input) {
        Ok(c) => c,
        Err(e) => return failure(op, &e),
    };
    #[cfg(windows)]
    {
        let _ = c;
        gnx::adapter::windows::broker::request_secret(op, input, secret).unwrap_or_else(|_| {
            Report::new(
                op,
                State::ActionRequired,
                "BROKER_UNAVAILABLE",
                Some("Install and start GNXRuntime."),
            )
        })
    }
    #[cfg(not(windows))]
    {
        let linux = gnx::adapter::linux::Linux::new(c.clone());
        let state = gnx::adapter::filesystem::Filesystem {
            root: linux.root.clone(),
        };
        if op == "apply" {
            let out = Report::new(op, State::ActionRequired, "RUNTIME_NOT_READY", None);
            let mut out = out;
            use gnx::port::state::StateStore;
            out.revision = match state.current() {
                Ok(v) => v,
                Err(e) => return failure(op, &e),
            };
            gnx::app::apply::run_secret(out, &c, &linux, &linux, &state, secret)
        } else {
            gnx::app::execute(op, &c, &linux, &linux, &state)
        }
    }
}
fn run() -> Report {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let op = a.first().map(String::as_str).unwrap_or("");
    if gnx::wire::opcode(op).is_none() {
        return failure(op, "INVALID_OPERATION");
    }
    if a.len() == 2 && a[1] == "--broker" {
        let mut b = zeroize::Zeroizing::new(Vec::new());
        if std::io::stdin()
            .take(gnx::wire::MAX_FRAME as u64 + 1)
            .read_to_end(&mut b)
            .is_err()
        {
            return failure(op, "INVALID_FRAME");
        }
        return match gnx::wire::decode(&b) {
            Ok(q) if q.operation == op => execute(op, &q.intent, q.secret.as_ref()),
            _ => failure(op, "INVALID_FRAME"),
        };
    }
    let input = if a.len() == 1 {
        std::fs::read_to_string(if cfg!(windows) {
            "gnx.toml"
        } else {
            "/etc/gnx/gnx.toml"
        })
    } else if a.len() == 3 && a[1] == "--config" {
        std::fs::read_to_string(&a[2])
    } else if a.len() == 2 && a[1] == "--stdin" {
        let mut s = String::new();
        std::io::stdin()
            .take(65537)
            .read_to_string(&mut s)
            .map(|_| s)
    } else {
        return failure(op, "INVALID_ARGUMENT");
    };
    let input = match input {
        Ok(s) if s.len() <= 65536 => s,
        _ => {
            return Report::new(
                op,
                State::ActionRequired,
                "CONFIG_REQUIRED",
                Some("Provide a readable intent file of at most 64 KiB."),
            )
        }
    };
    let mut result = execute(op, &input, None);
    // Only a Linux decision authorizes asking for a secret. Noninteractive runs never hang.
    for _ in 0..2 {
        if op != "apply" || !std::io::stdin().is_terminal() {
            break;
        }
        let Some(kind) = result.secret_kind else {
            break;
        };
        let prompt = match kind {
            gnx::domain::secret::SecretKind::ComputePassword => "Compute password (hidden): ",
            gnx::domain::secret::SecretKind::AccessEnrollment => "Access enrollment key (hidden): ",
        };
        let value = match rpassword::prompt_password(prompt) {
            Ok(s) => s.into_bytes(),
            Err(_) => break,
        };
        let secret = match Secret::new(kind, value) {
            Ok(s) => s,
            Err(e) => return failure(op, &e),
        };
        result = execute(op, &input, Some(&secret));
    }
    result
}
fn main() {
    let r = run();
    println!("{}", serde_json::to_string(&r).unwrap());
    std::process::exit(r.exit())
}
