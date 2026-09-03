use std::{io::Write, process::ExitCode};

fn main() -> ExitCode {
    match gnx::cli::run() {
        Ok(message) => {
            println!("READY {message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let context = match &error {
                gnx::Error::AccessReport { fields, .. } => format!("{fields}\n"),
                _ => String::new(),
            };
            let _ = std::io::stderr()
                .write_all(format!("{context}FAILED {}\n", error.label()).as_bytes());
            ExitCode::from(error.exit_code())
        }
    }
}
