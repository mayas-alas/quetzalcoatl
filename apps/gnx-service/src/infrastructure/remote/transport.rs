use std::env;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;

use zeroize::Zeroizing;

use super::limits::{
    MAX_REMOTE_INPUT_BYTES, MAX_REMOTE_OUTPUT_BYTES, REMOTE_COMMAND_TIMEOUT, REMOTE_POLL_INTERVAL,
};
use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::infrastructure::runtime_assets::{CREATE_NO_WINDOW, MACHINE_NAME};
use std::time::Instant;

pub(crate) fn machine_stdin<I, S>(podman: &Path, args: I, input: &[u8]) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = machine_stdin_output(podman, args, input)?;
    check_output(output, "podman machine operation")
}

// Podman Machine SSH ultimately transports a remote command string. Multi-line programs are
// therefore sent through stdin to `sh -s`; no caller may build a dynamic `sh -c` expression.
pub(crate) fn machine_stdin_output<I, S>(
    podman: &Path,
    args: I,
    input: &[u8],
) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    crate::infrastructure::service_shutdown::ensure_running()?;
    if input.len() > MAX_REMOTE_INPUT_BYTES {
        return Err(GateError::command(format!(
            "remote stdin exceeds the {} byte contract",
            MAX_REMOTE_INPUT_BYTES
        )));
    }

    let mut command = Command::new(podman);
    command
        .args(["machine", "ssh", "--username", "root", MACHINE_NAME])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);

    let mut child = command.spawn().map_err(|error| {
        GateError::command(format!("cannot start podman machine operation: {error}"))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| GateError::command("podman machine stdin is unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GateError::command("podman machine stdout is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GateError::command("podman machine stderr is unavailable"))?;

    let input = Zeroizing::new(input.to_vec());
    let stdin_worker = thread::spawn(move || -> Result<(), String> {
        stdin
            .write_all(&input)
            .map_err(|error| format!("cannot write podman machine stdin: {error}"))?;
        drop(stdin);
        Ok(())
    });
    let stdout_worker = thread::spawn(move || read_limited(stdout, "stdout"));
    let stderr_worker = thread::spawn(move || read_limited(stderr, "stderr"));

    let started = Instant::now();
    let status = loop {
        if crate::infrastructure::service_shutdown::requested() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdin_worker.join();
            let _ = stdout_worker.join();
            let _ = stderr_worker.join();
            return Err(GateError::new(
                "SERVICE_STOPPING",
                Component::None,
                "service shutdown canceled a Podman Machine operation",
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < REMOTE_COMMAND_TIMEOUT => {
                thread::sleep(REMOTE_POLL_INTERVAL);
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdin_worker.join();
                let _ = stdout_worker.join();
                let _ = stderr_worker.join();
                return Err(GateError::command(format!(
                    "podman machine operation exceeded {} seconds",
                    REMOTE_COMMAND_TIMEOUT.as_secs()
                )));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdin_worker.join();
                let _ = stdout_worker.join();
                let _ = stderr_worker.join();
                return Err(GateError::command(format!(
                    "cannot poll podman machine operation: {error}"
                )));
            }
        }
    };

    join_worker(stdin_worker, "stdin")?;
    let stdout = join_worker(stdout_worker, "stdout")?;
    let stderr = join_worker(stderr_worker, "stderr")?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_limited<R: Read>(reader: R, stream: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    reader
        .take((MAX_REMOTE_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| format!("cannot read podman machine {stream}: {error}"))?;
    if output.len() > MAX_REMOTE_OUTPUT_BYTES {
        return Err(format!(
            "podman machine {stream} exceeds the {} byte contract",
            MAX_REMOTE_OUTPUT_BYTES
        ));
    }
    Ok(output)
}

fn join_worker<T>(
    worker: thread::JoinHandle<Result<T, String>>,
    stream: &str,
) -> Result<T, GateError> {
    worker
        .join()
        .map_err(|_| GateError::command(format!("podman machine {stream} worker panicked")))?
        .map_err(GateError::command)
}

pub(crate) fn system_binary(name: &str) -> Result<PathBuf, GateError> {
    let root = env::var_os("SystemRoot").ok_or_else(|| {
        GateError::command("SystemRoot is not available in the service environment")
    })?;
    let path = PathBuf::from(root).join("System32").join(name);
    existing_binary(path)
}

pub(crate) fn podman_binary() -> Result<PathBuf, GateError> {
    let root = env::var_os("ProgramFiles").ok_or_else(|| {
        GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "ProgramFiles is not available in the service environment",
        )
    })?;
    existing_binary(PathBuf::from(root).join("Podman").join("podman.exe"))
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))
}

pub(crate) fn existing_binary(path: PathBuf) -> Result<PathBuf, GateError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(GateError::command(format!(
            "required executable is absent: {}",
            path.display()
        )))
    }
}

pub(crate) fn run_command<I, S>(program: &Path, args: I) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    crate::infrastructure::service_shutdown::ensure_running()?;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| {
            GateError::command(format!("cannot execute {}: {error}", program.display()))
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GateError::command("local command stdout is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GateError::command("local command stderr is unavailable"))?;
    let stdout_worker = thread::spawn(move || read_limited(stdout, "stdout"));
    let stderr_worker = thread::spawn(move || read_limited(stderr, "stderr"));
    let started = Instant::now();
    let status = loop {
        if crate::infrastructure::service_shutdown::requested() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_worker.join();
            let _ = stderr_worker.join();
            return Err(GateError::new(
                "SERVICE_STOPPING",
                Component::None,
                format!(
                    "service shutdown canceled local command {}",
                    program.display()
                ),
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < REMOTE_COMMAND_TIMEOUT => {
                thread::sleep(REMOTE_POLL_INTERVAL);
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_worker.join();
                let _ = stderr_worker.join();
                return Err(GateError::command(format!(
                    "{} exceeded {} seconds",
                    program.display(),
                    REMOTE_COMMAND_TIMEOUT.as_secs()
                )));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_worker.join();
                let _ = stderr_worker.join();
                return Err(GateError::command(format!(
                    "cannot poll {}: {error}",
                    program.display()
                )));
            }
        }
    };
    let stdout = join_worker(stdout_worker, "stdout")?;
    let stderr = join_worker(stderr_worker, "stderr")?;
    let output = Output {
        status,
        stdout,
        stderr,
    };
    check_output(output, &program.display().to_string())
}

pub(crate) fn check_output(output: Output, operation: &str) -> Result<Output, GateError> {
    if output.status.success() {
        return Ok(output);
    }
    let detail = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    Err(GateError::command(format!(
        "{operation} failed with exit {}: {}",
        output.status.code().unwrap_or(-1),
        bounded_text(detail)
    )))
}

pub(crate) fn bounded_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).replace(['\r', '\n'], " ");
    let text = text.trim();
    const MAX_CHARS: usize = 1600;
    const HEAD_CHARS: usize = 512;
    const SEPARATOR: &str = " ... ";
    let char_count = text.chars().count();
    if char_count <= MAX_CHARS {
        return text.to_owned();
    }

    let tail_chars = MAX_CHARS - HEAD_CHARS - SEPARATOR.chars().count();
    let head = text.chars().take(HEAD_CHARS).collect::<String>();
    let tail = text
        .chars()
        .skip(char_count - tail_chars)
        .collect::<String>();
    format!("{head}{SEPARATOR}{tail}")
}

#[cfg(test)]
mod tests {
    use super::bounded_text;

    #[test]
    fn bounded_text_preserves_the_error_tail() {
        let input = format!("{}FINAL_DIAGNOSTIC", "x".repeat(2000));
        let output = bounded_text(input.as_bytes());

        assert_eq!(output.chars().count(), 1600);
        assert!(output.starts_with(&"x".repeat(512)));
        assert!(output.contains(" ... "));
        assert!(output.ends_with("FINAL_DIAGNOSTIC"));
    }
}
