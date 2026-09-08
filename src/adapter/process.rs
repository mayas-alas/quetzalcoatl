use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub struct Output {
    pub code: i32,
    pub stdout: Zeroizing<Vec<u8>>,
}
pub fn run(
    program: &str,
    args: &[&str],
    input: Option<&[u8]>,
    timeout: Duration,
    limit: usize,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|_| "PROCESS_START_FAILED")?;
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut b = Zeroizing::new(Vec::new());
        let result = stdout.take(limit as u64 + 1).read_to_end(&mut b);
        result.map(|_| b)
    });
    let writer = input.map(|bytes| {
        let bytes = Zeroizing::new(bytes.to_vec());
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => break Err("PROCESS_TIMEOUT".to_string()),
        }
    };
    // Terminate the whole process group so descendants cannot retain pipe handles.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
    if let Some(w) = writer {
        let _ = w.join();
    }
    let stdout = reader
        .join()
        .map_err(|_| "PROCESS_READ_FAILED")?
        .map_err(|_| "PROCESS_READ_FAILED")?;
    if stdout.len() > limit {
        return Err("PROCESS_OUTPUT_LIMIT".into());
    }
    Ok(Output {
        code: status?.code().unwrap_or(1),
        stdout,
    })
}
pub fn checked(
    program: &str,
    args: &[&str],
    input: Option<&[u8]>,
    seconds: u64,
) -> Result<Zeroizing<Vec<u8>>, String> {
    let o = run(
        program,
        args,
        input,
        Duration::from_secs(seconds),
        1024 * 1024,
    )?;
    if o.code != 0 {
        return Err("PROCESS_FAILED".into());
    }
    Ok(o.stdout)
}
