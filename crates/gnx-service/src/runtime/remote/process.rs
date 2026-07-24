use super::super::*;

pub(in crate::runtime) fn machine_stdin<I, S>(podman: &Path, args: I, input: &[u8]) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = machine_stdin_output(podman, args, input)?;
    check_output(output, "podman machine probe")
}

// Podman machine SSH transports a remote command string, not a preserved argv vector.
// Send multiword shell programs through stdin to `sh -s`; never pass them as a `sh -c` argument.
pub(in crate::runtime) fn machine_stdin_output<I, S>(podman: &Path, args: I, input: &[u8]) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(podman);
    command
        .args(["machine", "ssh", "--username", "root", MACHINE_NAME])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| {
        GateError::command(format!("cannot start podman machine probe: {error}"))
    })?;
    child
        .stdin
        .take()
        .ok_or_else(|| GateError::command("podman machine probe stdin is unavailable"))?
        .write_all(input)
        .map_err(|error| GateError::command(format!("cannot write machine probe: {error}")))?;
    let output = child
        .wait_with_output()
        .map_err(|error| GateError::command(format!("cannot wait for machine probe: {error}")))?;
    Ok(output)
}


pub(in crate::runtime) fn system_binary(name: &str) -> Result<PathBuf, GateError> {
    let root = env::var_os("SystemRoot").ok_or_else(|| {
        GateError::command("SystemRoot is not available in the service environment")
    })?;
    let path = PathBuf::from(root).join("System32").join(name);
    existing_binary(path)
}

pub(in crate::runtime) fn podman_binary() -> Result<PathBuf, GateError> {
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

pub(in crate::runtime) fn existing_binary(path: PathBuf) -> Result<PathBuf, GateError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(GateError::command(format!(
            "required executable is absent: {}",
            path.display()
        )))
    }
}

pub(in crate::runtime) fn run_command<I, S>(program: &Path, args: I) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            GateError::command(format!("cannot execute {}: {error}", program.display()))
        })?;
    check_output(output, &program.display().to_string())
}

pub(in crate::runtime) fn check_output(output: Output, operation: &str) -> Result<Output, GateError> {
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
pub(in crate::runtime) fn bounded_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).replace(['\r', '\n'], " ");
    text.chars()
        .take(1600)
        .collect::<String>()
        .trim()
        .to_owned()
}

