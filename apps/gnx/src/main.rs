#[cfg(windows)]
pub use gnx::client;

mod commands;
mod error;
mod output;

fn main() {
    let action = match commands::parse_args() {
        Ok(action) => action,
        Err(()) => {
            eprintln!("{}", commands::usage());
            std::process::exit(64);
        }
    };
    if let Err(error) = commands::run(action) {
        eprintln!("gnx: {error}");
        std::process::exit(1);
    }
}
