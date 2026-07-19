#[cfg(windows)]
mod pipe;

use std::env;

use gnx_protocol::StatusResponse;

fn usage() -> &'static str {
    "Usage: gnx status [--json]"
}

fn parse_args() -> Result<bool, ()> {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (Some(command), None, None) if command == "status" => Ok(false),
        (Some(command), Some(format), None) if command == "status" && format == "--json" => {
            Ok(true)
        }
        _ => Err(()),
    }
}

#[cfg(windows)]
fn run(json: bool) -> Result<(), String> {
    let status = pipe::status()?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&status).map_err(|e| format!("cannot encode status: {e}"))?
        );
    } else {
        print_human(&status);
    }
    Ok(())
}

#[cfg(not(windows))]
fn run(_json: bool) -> Result<(), String> {
    Err("gnx requires Windows".into())
}

fn print_human(status: &StatusResponse) {
    println!("overall: {}", status.overall);
    println!("stage: {}", status.stage);
    println!("role: {}", status.role.as_deref().unwrap_or("not_resolved"));
    println!("service: {}", status.components.service);
    println!("wsl: {}", status.components.wsl);
    println!("podman_machine: {}", status.components.podman_machine);
    println!("kvm: {}", status.components.kvm);
    if let Some(error) = &status.last_error {
        println!("last_error: {error}");
    }
}

fn main() {
    let json = match parse_args() {
        Ok(json) => json,
        Err(()) => {
            eprintln!("{}", usage());
            std::process::exit(64);
        }
    };
    if let Err(error) = run(json) {
        eprintln!("gnx: {error}");
        std::process::exit(1);
    }
}
