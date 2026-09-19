#[cfg(windows)]
fn main() {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a == ["--install"] {
        match gnx::adapter::windows::account::install() {
            Ok(sid) => println!(
                "{}",
                serde_json::json!({"schema":1,"state":"READY","account_sid":sid})
            ),
            Err(code) => {
                println!(
                    "{}",
                    serde_json::json!({"schema":1,"state":"FAILED","code":code})
                );
                std::process::exit(1)
            }
        }
        return;
    }
    if !a.is_empty() {
        std::process::exit(1)
    }
    if gnx::adapter::windows::service::run().is_err() {
        std::process::exit(1)
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("GNXRuntime is a Windows SCM entrypoint.");
    std::process::exit(2)
}
