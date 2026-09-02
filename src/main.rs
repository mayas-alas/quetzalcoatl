use std::process::ExitCode;

fn main() -> ExitCode {
    match gnx::cli::run() {
        Ok(message) => {
            println!("READY {message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAILED {}", error.label());
            ExitCode::from(error.exit_code())
        }
    }
}
