use std::io::IsTerminal;

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Account {
    Control,
    Compute,
}

fn failure(operation: &'static str) -> Error {
    Error::External { operation, code: 1 }
}

pub fn show(account: Account) -> Result<String> {
    if !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return Err(failure("CREDENTIAL_TERMINAL_REQUIRED"));
    }
    #[cfg(windows)]
    windows::show(account)?;
    #[cfg(not(windows))]
    {
        let _ = account;
        return Err(Error::HostUnsupported);
    }
    #[cfg(windows)]
    Ok("credentials-hidden".into())
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        io::Write,
        path::Path,
        process::{Command, Stdio},
    };
    use windows_sys::Win32::{Foundation::HANDLE, System::Console::*};
    use zeroize::{Zeroize, Zeroizing};

    // No Debug implementation: neither errors nor test output may format this value.
    struct Credential {
        user: String,
        password: String,
    }

    impl Drop for Credential {
        fn drop(&mut self) {
            self.password.zeroize();
        }
    }

    fn reader(path: &Path) -> String {
        // Only a path enters argv. DPAPI plaintext travels in a private child pipe.
        let path = path.to_string_lossy().replace('\'', "''");
        format!(
            r#"$ErrorActionPreference='Stop'; try {{
            [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
            $c=Import-Clixml -LiteralPath '{path}'
            if ($c -isnot [PSCredential]) {{ exit 1 }}
            [Console]::Write($c.UserName + [char]0 + $c.GetNetworkCredential().Password)
        }} catch {{ exit 1 }}"#
        )
    }

    fn powershell() -> Result<Command> {
        let shell = std::env::var_os("SystemRoot").ok_or_else(|| failure("CREDENTIAL_READ"))?;
        let root = Path::new(&shell).join("System32/WindowsPowerShell/v1.0");
        let mut command = Command::new(root.join("powershell.exe"));
        command
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
            // Do not load incompatible PowerShell 7 or user-supplied modules.
            .env("PSModulePath", root.join("Modules"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        Ok(command)
    }

    fn load(path: &Path) -> Result<Credential> {
        if !path.is_file() {
            return Err(failure("CREDENTIAL_MISSING"));
        }
        let output = powershell()?
            .arg(reader(path))
            .output()
            .map_err(|_| failure("CREDENTIAL_READ"))?;
        let data = Zeroizing::new(output.stdout);
        if !output.status.success() {
            return Err(failure("CREDENTIAL_READ"));
        }
        // Avoid PowerShell output pipelines/transcription of the plaintext value.
        let (user, password) = std::str::from_utf8(&data)
            .ok()
            .and_then(|value| value.split_once('\0'))
            .ok_or_else(|| failure("CREDENTIAL_READ"))?;
        let credential = Credential {
            user: user.into(),
            password: password.into(),
        };
        // This cut only handles the generated ASCII accounts; reject terminal escapes.
        if [&credential.user, &credential.password]
            .iter()
            .any(|value| {
                value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|b| b.is_ascii_graphic())
            })
        {
            return Err(failure("CREDENTIAL_FORMAT"));
        }
        Ok(credential)
    }

    struct Screen {
        handle: HANDLE,
        mode: u32,
    }
    impl Screen {
        fn open() -> Result<Self> {
            let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
            let mut mode = 0;
            if unsafe { GetConsoleMode(handle, &mut mode) } == 0
                || unsafe { SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) } == 0
            {
                return Err(failure("CREDENTIAL_CONSOLE"));
            }
            let screen = Self { handle, mode };
            emit(b"\x1b[?1049h\x1b[2J\x1b[H")?;
            Ok(screen)
        }
    }
    impl Drop for Screen {
        fn drop(&mut self) {
            let _ = emit(b"\x1b[2J\x1b[H\x1b[?1049l");
            unsafe {
                SetConsoleMode(self.handle, self.mode);
            }
        }
    }
    fn emit(bytes: &[u8]) -> Result<()> {
        let mut out = std::io::stdout().lock();
        out.write_all(bytes)
            .and_then(|_| out.flush())
            .map_err(|_| failure("CREDENTIAL_CONSOLE"))
    }
    fn enter() -> Result<()> {
        let mut input = Zeroizing::new(String::new());
        if std::io::stdin()
            .read_line(&mut input)
            .map_err(|_| failure("CREDENTIAL_INPUT"))?
            == 0
        {
            return Err(failure("CREDENTIAL_INPUT"));
        }
        Ok(())
    }

    pub(super) fn show(account: Account) -> Result<()> {
        let (name, url, hint) = match account {
            Account::Control => ("control", "https://mesh.gnx", "Use the account email."),
            Account::Compute => (
                "compute",
                "https://proxmox.mesh.gnx",
                "Web login: root; realm: Linux PAM.",
            ),
        };
        emit(b"GNX: stop screen recording/sharing before revealing a password.\nPress Enter to reveal; Ctrl+C cancels.\n")?;
        enter()?;
        let screen = Screen::open()?;
        let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| failure("CREDENTIAL_READ"))?;
        let credential = load(
            &Path::new(&local)
                .join("GNX")
                .join(name)
                .join("owner.credential.xml"),
        )?;
        let display = Zeroizing::new(format!(
            "GNX {name}\n{url}\nAccount: {}\n{hint}\nPassword: {}\n\nPress Enter to hide and return.\n",
            credential.user, credential.password
        ));
        emit(display.as_bytes())?;
        enter()?;
        // Clear the alternate buffer before restoring the shell's scrollback.
        emit(b"\x1b[2J\x1b[H")?;
        drop(screen);
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn decrypts_only_a_generated_nonsecret_dpapi_fixture() {
            let folder = std::env::temp_dir().join(format!(
                "gnx-credential-fixture-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&folder).unwrap();
            let path = folder.join("owner.credential.xml");
            let quoted = path.to_string_lossy().replace('\'', "''");
            let script = format!(
                "$ErrorActionPreference='Stop'; $s=ConvertTo-SecureString 'GNX-NONSECRET-PROBE' -AsPlainText -Force; [PSCredential]::new('operator@email.gnx',$s) | Export-Clixml -LiteralPath '{quoted}'"
            );
            let output = powershell()
                .unwrap()
                .arg(script)
                .stderr(Stdio::piped())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "nonsecret fixture setup: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result = load(&path);
            std::fs::remove_file(&path).unwrap();
            std::fs::remove_dir(&folder).unwrap();
            let credential = match result {
                Ok(value) => value,
                Err(_) => panic!("DPAPI fixture read failed"),
            };
            assert!(credential.user == "operator@email.gnx");
            assert!(credential.password == "GNX-NONSECRET-PROBE");
        }

        #[test]
        fn dpapi_reader_quotes_paths_and_never_uses_profiles_or_files_for_plaintext() {
            let script = reader(Path::new("C:/operator's/GNX/owner.credential.xml"));
            assert!(script.contains("operator''s"));
            assert!(script.contains("Import-Clixml -LiteralPath"));
            assert!(!script.contains("Set-Content"));
            assert!(!script.contains("Set-Clipboard"));
        }

        #[test]
        fn missing_credentials_return_only_a_fixed_label() {
            let result = load(Path::new("Z:/gnx-nonexistent-fixture/owner.credential.xml"));
            assert!(matches!(
                result,
                Err(Error::External {
                    operation: "CREDENTIAL_MISSING",
                    ..
                })
            ));
        }
    }
}
