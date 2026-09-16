use quetzalcoatl_gnx::{CLI_EXE, PRODUCT, SETUP_EXE};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] {
        println!("{PRODUCT}\n{SETUP_EXE} preflight\n{SETUP_EXE} install <rootfs.tar> <sha256> <client-SID>\n\nInstall requires an elevated console and an Ubuntu 24.04 rootfs verified against a trusted SHA256.\nThe companion {CLI_EXE} must be beside this EXE. This is an experimental installer.\nNo changes are made by preflight. Reboot-required exit code: 3010."); return;
    }
    #[cfg(windows)] let result = quetzalcoatl_gnx::windows::setup(&args);
    #[cfg(not(windows))] let result: Result<(), String> = Err("Windows host required.".into());
    if let Err(e) = result { eprintln!("{e}"); std::process::exit(1); }
}
