use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::GnxError;

const DEFAULT_OUTPUT_LIMIT: usize = 256 * 1024;
const ALLOWED_ENVIRONMENT: &[&str] = &[
    "PATH",
    "SystemRoot",
    "WINDIR",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "ProgramData",
    "ProgramFiles",
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_RUNTIME_DIR",
    "LANG",
    "LC_ALL",
    "TERM",
];

#[derive(Debug, Clone)]
pub struct CommandSpec {
    program: PathBuf,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    environment: Vec<(OsString, OsString)>,
    stdin: Option<Vec<u8>>,
    timeout: Duration,
    output_limit: usize,
}

#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration: Duration,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

impl CommandSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            environment: Vec::new(),
            stdin: None,
            timeout: Duration::from_secs(300),
            output_limit: DEFAULT_OUTPUT_LIMIT,
        }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|arg| arg.as_ref().to_os_string()));
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.environment
            .push((name.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    pub fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(bytes.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn output_limit(mut self, bytes: usize) -> Self {
        self.output_limit = bytes;
        self
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn run(&self, operation: &'static str) -> Result<ProcessOutput, GnxError> {
        let started = Instant::now();
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();

        for name in ALLOWED_ENVIRONMENT {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for (name, value) in &self.environment {
            command.env(name, value);
        }
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }

        let mut child = command.spawn().map_err(|error| {
            GnxError::process(operation, &self.program, error.to_string(), false)
        })?;

        if let Some(bytes) = &self.stdin {
            let mut input = child.stdin.take().ok_or_else(|| {
                GnxError::process(operation, &self.program, "stdin no disponible", false)
            })?;
            input.write_all(bytes).map_err(|error| {
                GnxError::process(operation, &self.program, error.to_string(), true)
            })?;
        }

        let stdout = child
            .stdout
            .take()
            .expect("stdout fue configurado como pipe");
        let stderr = child
            .stderr
            .take()
            .expect("stderr fue configurado como pipe");
        let output_limit = self.output_limit;
        let stdout_reader = thread::spawn(move || read_capped(stdout, output_limit));
        let stderr_reader = thread::spawn(move || read_capped(stderr, output_limit));

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < self.timeout => {
                    thread::sleep(Duration::from_millis(100));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GnxError::process(
                        operation,
                        &self.program,
                        format!("timeout después de {} segundos", self.timeout.as_secs()),
                        true,
                    ));
                }
                Err(error) => {
                    return Err(GnxError::process(
                        operation,
                        &self.program,
                        error.to_string(),
                        true,
                    ));
                }
            }
        };

        let (stdout, stdout_truncated) = stdout_reader.join().map_err(|_| {
            GnxError::process(operation, &self.program, "falló lectura de stdout", false)
        })?;
        let (stderr, stderr_truncated) = stderr_reader.join().map_err(|_| {
            GnxError::process(operation, &self.program, "falló lectura de stderr", false)
        })?;

        Ok(ProcessOutput {
            exit_code: status.code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            truncated: stdout_truncated || stderr_truncated,
            duration: started.elapsed(),
        })
    }

    pub fn run_checked(&self, operation: &'static str) -> Result<ProcessOutput, GnxError> {
        let output = self.run(operation)?;
        if output.success() {
            Ok(output)
        } else {
            let detail = if output.stderr.trim().is_empty() {
                output.stdout.trim()
            } else {
                output.stderr.trim()
            };
            Err(GnxError::process(
                operation,
                &self.program,
                format!("exit {:?}: {detail}", output.exit_code),
                true,
            ))
        }
    }
}

fn read_capped(mut reader: impl Read, limit: usize) -> (Vec<u8>, bool) {
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let remaining = limit.saturating_sub(output.len());
                let kept = remaining.min(read);
                output.extend_from_slice(&buffer[..kept]);
                truncated |= kept < read;
            }
        }
    }
    (output, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_reader_drains_and_marks_truncation() {
        let input = vec![b'x'; 32];
        let (output, truncated) = read_capped(input.as_slice(), 8);
        assert_eq!(output.len(), 8);
        assert!(truncated);
    }
}
