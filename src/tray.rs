#![cfg_attr(windows, windows_subsystem = "windows")]
#[cfg(windows)]
mod native {
    use std::{mem::size_of, path::PathBuf, ptr::{null, null_mut}};
    use windows_sys::Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::{Shell::*, WindowsAndMessaging::*}};
    use quetzalcoatl_gnx::{PRODUCT, SETUP_EXE};

    const WM_TRAY: u32 = WM_APP + 1;
    const ID_OPEN: usize = 1001;
    const ID_UNINSTALL: usize = 1002;
    const ID_EXIT: usize = 1003;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_RBUTTONUP: u32 = 0x0205;

    fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
    fn setup_path() -> PathBuf { PathBuf::from(r"C:\ProgramData\QuetzalcoatlGNX-Setup").join(SETUP_EXE) }
    unsafe fn launch(args: &str, elevate: bool) {
        let file = wide(setup_path().to_str().unwrap_or(SETUP_EXE));
        let parameters = wide(args);
        let verb = wide(if elevate { "runas" } else { "open" });
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.lpVerb = verb.as_ptr(); info.lpFile = file.as_ptr(); info.lpParameters = parameters.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        ShellExecuteExW(&mut info);
        if !info.hProcess.is_null() { CloseHandle(info.hProcess); }
    }
    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        if msg == WM_TRAY && (l as u32 == WM_LBUTTONUP || l as u32 == WM_RBUTTONUP) {
            let menu = CreatePopupMenu();
            let open = wide("Abrir asistente"); let uninstall = wide("Desinstalar GNX..."); let exit = wide("Salir");
            AppendMenuW(menu, MF_STRING, ID_OPEN, open.as_ptr());
            AppendMenuW(menu, MF_STRING, ID_UNINSTALL, uninstall.as_ptr());
            AppendMenuW(menu, MF_SEPARATOR, 0, null()); AppendMenuW(menu, MF_STRING, ID_EXIT, exit.as_ptr());
            let mut point: POINT = std::mem::zeroed(); GetCursorPos(&mut point); SetForegroundWindow(hwnd);
            TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, null()); DestroyMenu(menu); PostMessageW(hwnd, WM_NULL, 0, 0);
        } else if msg == WM_COMMAND {
            match (w as usize) & 0xffff { ID_OPEN => launch("--gui", false), ID_UNINSTALL => launch("--uninstall --confirm", true), ID_EXIT => { let mut data: NOTIFYICONDATAW = std::mem::zeroed(); data.cbSize = size_of::<NOTIFYICONDATAW>() as u32; data.hWnd = hwnd; data.uID = 1; Shell_NotifyIconW(NIM_DELETE, &mut data); DestroyWindow(hwnd); PostQuitMessage(0); }, _ => () }
        } else if msg == WM_DESTROY { PostQuitMessage(0); }
        DefWindowProcW(hwnd, msg, w, l)
    }
    pub fn run() -> Result<(), String> {
        unsafe {
            let instance = GetModuleHandleW(null()); let class = wide("QuetzalcoatlGNXTray");
            let wc = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: instance, lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
            if RegisterClassW(&wc) == 0 { return Err("No se pudo registrar la ventana del tray".into()); }
            let hwnd = CreateWindowExW(0, class.as_ptr(), wide(PRODUCT).as_ptr(), 0, 0, 0, 0, 0, HWND_MESSAGE, null_mut(), instance, null_mut());
            if hwnd.is_null() { return Err("No se pudo crear la ventana del tray".into()); }
            let mut data: NOTIFYICONDATAW = std::mem::zeroed(); data.cbSize = size_of::<NOTIFYICONDATAW>() as u32; data.hWnd = hwnd; data.uID = 1; data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP; data.uCallbackMessage = WM_TRAY; data.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
            let tip = wide(PRODUCT); let tip_len = tip.len().min(data.szTip.len()); data.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);
            if Shell_NotifyIconW(NIM_ADD, &mut data) == 0 { return Err("No se pudo crear el ícono de notificación".into()); }
            let mut msg: MSG = std::mem::zeroed(); while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
        }
        Ok(())
    }
}
fn main() { #[cfg(windows)] if let Err(e) = native::run() { eprintln!("{e}"); std::process::exit(1); } }
