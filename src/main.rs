use std::process::ExitCode;

fn main() -> ExitCode {
    gnx::run(std::env::args_os())
}
