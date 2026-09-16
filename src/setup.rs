#![cfg_attr(windows, windows_subsystem = "windows")]
use quetzalcoatl_gnx::{CLI_EXE, PRODUCT, SETUP_EXE};
#[cfg(windows)]
mod setup_gui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if !args.is_empty() && args != ["--gui"] {
        unsafe { windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS); }
    }
    if args == ["--help"] {
        println!("{PRODUCT}\n{SETUP_EXE} [--gui]\n{SETUP_EXE} preflight\n{SETUP_EXE} install <client-SID>\n{SETUP_EXE} install <rootfs.tar> <sha256> <client-SID>\n{SETUP_EXE} --resume\n\nOnline install is resumable and requires elevation. No arguments opens the graphical assistant.\nOffline install requires a trusted SHA256 (legacy non-resumable path).\nPlace {CLI_EXE} next to this EXE. Experimental. Reboot-required exit code: 3010.");
        return;
    }
    #[cfg(windows)]
    let result = dispatch(&args);
    #[cfg(not(windows))]
    let result: Result<i32, String> = Err("Windows host required.".into());
    match result { Ok(code) => std::process::exit(code), Err(e) => { eprintln!("{e}"); std::process::exit(1); } }
}

#[cfg(windows)]
fn dispatch(args: &[String]) -> Result<i32, String> {
    use quetzalcoatl_gnx::windows::{self, installer};
    match args {
        [] => { setup_gui::show()?; Ok(0) }
        [one] if one == "--gui" => { setup_gui::show()?; Ok(0) }
        [one] if one == "--resume" => installer::start(None),
        [op, sid] if op == "--start" || op == "install" => installer::start(Some(sid)),
        [op, sid] if op == "--restart" => installer::restart(sid),
        [op, sid] if op == "--engine-install" => { windows::setup(&["install".into(), sid.clone()])?; Ok(0) }
        _ => { windows::setup(args)?; Ok(0) }
    }
}
