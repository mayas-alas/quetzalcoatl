use crate::report::{Report, State};
use std::process::{Command, Stdio};

const RELEASE_ROOT: &str = "C:\\Program Files\\GNX\\release-transaction";
const RELEASE_STAGE: &str = r#"set -eu
digest="$1"
txn=/var/lib/gnx/release-transaction
rm -rf "$txn/candidate" "$txn/previous" "$txn/state" "$txn/apply.json" "$txn/status.json"
mkdir -p "$txn/candidate" "$txn/previous"
umask 077
mv /var/lib/gnx/release-upload/bundle.tar "$txn/bundle.tar"
rm -rf /var/lib/gnx/release-upload
tar -xf "$txn/bundle.tar" -C "$txn/candidate"
actual=$(sha256sum "$txn/candidate/usr/local/bin/gnx" | cut -d ' ' -f1)
[ "$actual" = "$digest" ] || exit 11
[ -x /usr/local/bin/gnx ] || exit 12
cp -a /usr/local/bin/gnx "$txn/previous/gnx"
cp -a /usr/local/share/gnx/runtime "$txn/previous/runtime"
rollback() {
 code=$?
 trap - EXIT HUP INT TERM
 if install -o root -g root -m 0755 "$txn/previous/gnx" /usr/local/bin/gnx.new \
  && mv -f /usr/local/bin/gnx.new /usr/local/bin/gnx \
  && rm -rf /usr/local/share/gnx/runtime \
  && cp -a "$txn/previous/runtime" /usr/local/share/gnx/runtime \
  && /usr/local/bin/gnx apply --config /etc/gnx/gnx.toml >/dev/null \
  && /usr/local/bin/gnx status --config /etc/gnx/gnx.toml >/dev/null; then
  printf '%s\n' ROLLED_BACK > "$txn/state"
  exit 20
 fi
 printf '%s\n' ROLLBACK_FAILED > "$txn/state"
 exit 21
}
trap rollback EXIT HUP INT TERM
install -o root -g root -m 0755 "$txn/candidate/usr/local/bin/gnx" /usr/local/bin/gnx.new
mv -f /usr/local/bin/gnx.new /usr/local/bin/gnx
rm -rf /usr/local/share/gnx/runtime.new
cp -a "$txn/candidate/usr/local/share/gnx/runtime" /usr/local/share/gnx/runtime.new
rm -rf /usr/local/share/gnx/runtime
mv /usr/local/share/gnx/runtime.new /usr/local/share/gnx/runtime
/usr/local/bin/gnx apply --config /etc/gnx/gnx.toml > "$txn/apply.json"
/usr/local/bin/gnx status --config /etc/gnx/gnx.toml > "$txn/status.json"
printf '%s\n' AWAITING_COMMIT > "$txn/state"
trap - EXIT HUP INT TERM
exit 0
"#;
const RELEASE_COMMIT: &str = r#"set -eu
txn=/var/lib/gnx/release-transaction
[ "$(cat "$txn/state")" = AWAITING_COMMIT ]
rm -rf "$txn"
"#;
const RELEASE_ROLLBACK: &str = r#"set -eu
txn=/var/lib/gnx/release-transaction
[ "$(cat "$txn/state")" = AWAITING_COMMIT ]
install -o root -g root -m 0755 "$txn/previous/gnx" /usr/local/bin/gnx.new
mv -f /usr/local/bin/gnx.new /usr/local/bin/gnx
rm -rf /usr/local/share/gnx/runtime
cp -a "$txn/previous/runtime" /usr/local/share/gnx/runtime
/usr/local/bin/gnx apply --config /etc/gnx/gnx.toml >/dev/null
/usr/local/bin/gnx status --config /etc/gnx/gnx.toml >/dev/null
rm -rf "$txn"
"#;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseTransaction {
    schema: u32,
    operation: String,
    release_serial: u64,
    linux_sha256: String,
}

fn run_wsl_script(script: &str, args: &[&str]) -> Result<i32, String> {
    let mut command = Command::new("C:\\Windows\\System32\\wsl.exe");
    command.args(["-d", "GNX", "-u", "root", "--exec", "/bin/sh", "-s", "--"]);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| "RELEASE_WSL_FAILED")?;
    use std::io::Write;
    child
        .stdin
        .take()
        .ok_or("RELEASE_WSL_FAILED")?
        .write_all(script.as_bytes())
        .map_err(|_| "RELEASE_WSL_FAILED")?;
    child
        .wait()
        .map_err(|_| "RELEASE_WSL_FAILED")?
        .code()
        .ok_or_else(|| "RELEASE_WSL_FAILED".into())
}

fn upload_release_bundle(bundle: std::fs::File) -> Result<i32, String> {
    Command::new("C:\\Windows\\System32\\wsl.exe")
        .args([
            "-d",
            "GNX",
            "-u",
            "root",
            "--exec",
            "/bin/sh",
            "-c",
            "set -eu; umask 077; rm -rf /var/lib/gnx/release-upload; mkdir -p /var/lib/gnx/release-upload; cat > /var/lib/gnx/release-upload/bundle.tar",
        ])
        .stdin(Stdio::from(bundle))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "RELEASE_WSL_FAILED")?
        .code()
        .ok_or_else(|| "RELEASE_WSL_FAILED".into())
}

fn release_result(
    root: &std::path::Path,
    name: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    crate::adapter::filesystem::atomic_write(&root.join(name), value.to_string().as_bytes(), 0o600)
}

fn handle_release_transaction(root: &std::path::Path) -> Result<(), String> {
    let release = std::path::Path::new(RELEASE_ROOT);
    let metadata_path = release.join("transaction.json");
    if !metadata_path.exists() {
        return Ok(());
    }
    let metadata: ReleaseTransaction = serde_json::from_slice(
        &std::fs::read(&metadata_path).map_err(|_| "RELEASE_TRANSACTION_INVALID")?,
    )
    .map_err(|_| "RELEASE_TRANSACTION_INVALID")?;
    if metadata.schema != 1
        || !matches!(metadata.operation.as_str(), "update" | "rollback")
        || metadata.release_serial == 0
        || metadata.linux_sha256.len() != 64
        || !metadata
            .linux_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("RELEASE_TRANSACTION_INVALID".into());
    }
    let commit_request = release.join("commit.request");
    let rollback_request = release.join("rollback.request");
    let request_matches = |path: &std::path::Path, expected: &str| {
        std::fs::read_to_string(path)
            .ok()
            .is_some_and(|value| value.trim_end_matches(&['\r', '\n'][..]) == expected)
    };
    if rollback_request.exists() && !request_matches(&rollback_request, "GNX-RELEASE-ROLLBACK-1") {
        return Err("RELEASE_TRANSACTION_INVALID".into());
    }
    if commit_request.exists() && !request_matches(&commit_request, "GNX-RELEASE-COMMIT-1") {
        return Err("RELEASE_TRANSACTION_INVALID".into());
    }
    if commit_request.exists() && !rollback_request.exists() {
        if root.join("release-commit-result.json").exists() {
            return Ok(());
        }
        let code = run_wsl_script(RELEASE_COMMIT, &[])?;
        let ready = code == 0;
        release_result(
            root,
            "release-commit-result.json",
            serde_json::json!({"schema":1,"operation":metadata.operation,"state":if ready {"READY"} else {"FAILED"},"code":if ready {"RELEASE_COMMITTED"} else {"RELEASE_COMMIT_FAILED"},"release_serial":metadata.release_serial}),
        )?;
        return if ready {
            Ok(())
        } else {
            Err("RELEASE_COMMIT_FAILED".into())
        };
    }
    if rollback_request.exists() {
        if root.join("release-rollback-result.json").exists() {
            return Ok(());
        }
        let code = run_wsl_script(RELEASE_ROLLBACK, &[])?;
        let ready = code == 0;
        release_result(
            root,
            "release-rollback-result.json",
            serde_json::json!({"schema":1,"operation":metadata.operation,"state":if ready {"READY"} else {"FAILED"},"code":if ready {"RELEASE_ROLLED_BACK"} else {"RELEASE_ROLLBACK_FAILED"},"release_serial":metadata.release_serial}),
        )?;
        return if ready {
            Ok(())
        } else {
            Err("RELEASE_ROLLBACK_FAILED".into())
        };
    }
    let result_path = root.join("release-result.json");
    if result_path.exists() {
        let result: serde_json::Value = serde_json::from_slice(
            &std::fs::read(result_path).map_err(|_| "RELEASE_RESULT_INVALID")?,
        )
        .map_err(|_| "RELEASE_RESULT_INVALID")?;
        return if result.get("state").and_then(|value| value.as_str()) == Some("READY") {
            Ok(())
        } else {
            Err("RELEASE_CANDIDATE_REJECTED".into())
        };
    }
    let bundle =
        std::fs::File::open(release.join("bundle.tar")).map_err(|_| "RELEASE_BUNDLE_MISSING")?;
    if upload_release_bundle(bundle)? != 0 {
        return Err("RELEASE_BUNDLE_STAGE_FAILED".into());
    }
    let code = run_wsl_script(RELEASE_STAGE, &[&metadata.linux_sha256])?;
    let (state, result_code, rollback_code) = match code {
        0 => ("READY", "RELEASE_CANDIDATE_READY", None),
        20 => ("FAILED", "RELEASE_CANDIDATE_REJECTED", Some("ROLLED_BACK")),
        21 => (
            "FAILED",
            "RELEASE_CANDIDATE_REJECTED",
            Some("ROLLBACK_FAILED"),
        ),
        _ => ("FAILED", "RELEASE_CANDIDATE_REJECTED", None),
    };
    release_result(
        root,
        "release-result.json",
        serde_json::json!({"schema":1,"operation":metadata.operation,"state":state,"code":result_code,"rollback_code":rollback_code,"release_serial":metadata.release_serial,"linux_sha256":metadata.linux_sha256}),
    )?;
    if code == 0 {
        Ok(())
    } else {
        Err(if code == 21 {
            "RELEASE_ROLLBACK_FAILED".into()
        } else {
            "RELEASE_CANDIDATE_REJECTED".into()
        })
    }
}
pub fn keep_alive() -> std::io::Result<std::process::Child> {
    // WSL stops idle distributions even when their systemd units are active.
    // Keep a service-owned session open; SCM supervises the Windows parent.
    Command::new("C:\\Windows\\System32\\wsl.exe")
        .args([
            "-d",
            "GNX",
            "-u",
            "root",
            "--exec",
            "/bin/sleep",
            "infinity",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

pub fn unregister_for_uninstall() -> Result<(), String> {
    let status = Command::new("C:\\Windows\\System32\\wsl.exe")
        .args(["--unregister", "GNX"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "WSL_UNREGISTER_FAILED")?;
    if status.success() {
        Ok(())
    } else {
        Err("WSL_UNREGISTER_FAILED".into())
    }
}
pub fn invoke(op: &str, intent: &str) -> Report {
    invoke_secret(op, intent, None)
}
pub fn invoke_secret(
    op: &str,
    intent: &str,
    secret: Option<&crate::domain::secret::Secret>,
) -> Report {
    let fail = || {
        Report::new(
            op,
            State::ActionRequired,
            "WSL_RUNTIME_UNAVAILABLE",
            Some("Verify the service-owned GNX runtime and installed Linux release."),
        )
    };
    let Ok(frame) = crate::wire::encode(op, intent, secret) else {
        return fail();
    };
    let (linux_timeout, host_timeout) = if op == "apply" {
        ("600", 620)
    } else {
        ("30", 40)
    };
    let result = super::super::process::run(
        "C:\\Windows\\System32\\wsl.exe",
        &[
            "--distribution",
            "GNX",
            "--user",
            "root",
            "--exec",
            "/usr/bin/timeout",
            linux_timeout,
            "/usr/local/bin/gnx",
            op,
            "--broker",
        ],
        Some(&frame),
        std::time::Duration::from_secs(host_timeout),
        1024 * 1024,
    );
    match result {
        Ok(o) => match serde_json::from_slice::<Report>(&o.stdout) {
            Ok(r) if r.schema == 1 && r.operation == op && r.exit() == o.code => r,
            _ => fail(),
        },
        _ => fail(),
    }
}
pub fn bootstrap() -> Result<(), String> {
    let root = std::path::Path::new(super::account::PRIVATE_ROOT);
    handle_release_transaction(root)?;
    if !root.join("bundle.tar").exists() {
        return Ok(());
    }
    let wsl = "C:\\Windows\\System32\\wsl.exe";
    if root.join("rootfs.tar").exists() && !root.join("wsl\\ext4.vhdx").exists() {
        let s = Command::new(wsl)
            .args([
                "--import",
                "GNX",
                "C:\\ProgramData\\GNX\\runtime\\wsl",
                "C:\\ProgramData\\GNX\\runtime\\rootfs.tar",
                "--version",
                "2",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "WSL_IMPORT_FAILED")?;
        if !s.success() {
            return Err("WSL_IMPORT_FAILED".into());
        }
    }
    let mut child = Command::new(wsl)
        .args([
            "-d", "GNX", "-u", "root", "--exec", "/bin/tar", "-xf", "-", "-C", "/",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "BUNDLE_INSTALL_FAILED")?;
    let mut f = std::fs::File::open(root.join("bundle.tar")).map_err(|_| "BUNDLE_MISSING")?;
    std::io::copy(&mut f, &mut child.stdin.take().unwrap()).map_err(|_| "BUNDLE_STREAM_FAILED")?;
    if !child.wait().map_err(|_| "BUNDLE_INSTALL_FAILED")?.success() {
        return Err("BUNDLE_INSTALL_FAILED".into());
    }
    let script="set -eu; umask 077; printf '[boot]\\nsystemd=true\\n[automount]\\nenabled=false\\n[interop]\\nenabled=false\\nappendWindowsPath=false\\n' > /etc/wsl.conf; test -f /etc/gnx/gnx.toml || cp /etc/gnx/gnx.example.toml /etc/gnx/gnx.toml; chmod 600 /etc/gnx/gnx.toml; chmod 755 /usr/local/bin/gnx";
    if !Command::new(wsl)
        .args(["-d", "GNX", "-u", "root", "--exec", "/bin/sh", "-c", script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "ISOLATION_FAILED")?
        .success()
    {
        return Err("ISOLATION_FAILED".into());
    }
    if !Command::new(wsl)
        .args(["--terminate", "GNX"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "WSL_RESTART_FAILED")?
        .success()
    {
        return Err("WSL_RESTART_FAILED".into());
    }
    std::fs::remove_file(root.join("bundle.tar")).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?;
    if root.join("rootfs.tar").exists() {
        std::fs::remove_file(root.join("rootfs.tar")).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?;
    }
    Ok(())
}
