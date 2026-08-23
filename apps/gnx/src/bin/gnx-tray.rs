#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod windows_tray {
    use std::ffi::OsString;
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::thread;
    use std::time::Duration;

    use gnx::client;
    use gnx_contracts::{PveUrl, StatusResponse};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, POINT,
        WPARAM,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CreateMutexW, DETACHED_PROCESS,
    };
    use windows_sys::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
        Shell_NotifyIconW, ShellExecuteW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
        DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, HMENU, KillTimer, LoadIconW,
        MF_DISABLED, MF_GRAYED, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
        RegisterWindowMessageW, SW_SHOWNORMAL, SetForegroundWindow, SetTimer, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP, WM_CLOSE, WM_CONTEXTMENU,
        WM_DESTROY, WM_ENDSESSION, WM_NULL, WM_QUERYENDSESSION, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    };

    const WINDOW_CLASS: &str = "Quetzalcoatl.GnxTray.Window";
    const INSTANCE_MUTEX: &str = "Local\\Quetzalcoatl.GnxTray";
    const TRAY_ICON_ID: u32 = 1;
    const APP_ICON_RESOURCE_ID: usize = 1;
    const WM_TRAY_ICON: u32 = WM_APP + 1;
    const TIMER_ID: usize = 1;
    const TIMER_INTERVAL_MS: u32 = 3_000;
    const MENU_STATUS: usize = 1001;
    const MENU_VERSION: usize = 1002;
    const MENU_CONNECT: usize = 1003;

    static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
    static STATE: OnceLock<Mutex<TrayState>> = OnceLock::new();

    struct TrayState {
        status_label: String,
        pve_url: Option<String>,
    }

    impl TrayState {
        fn unavailable() -> Self {
            Self {
                status_label: "Estado: servicio no disponible".into(),
                pve_url: None,
            }
        }
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Mode {
        Run,
        Shutdown,
        LaunchDetached,
    }

    pub fn entry() -> i32 {
        match parse_mode(std::env::args_os().skip(1)) {
            Ok(Mode::Run) => {
                run();
                0
            }
            Ok(Mode::Shutdown) => {
                request_shutdown();
                0
            }
            Ok(Mode::LaunchDetached) => match launch_detached() {
                Ok(()) => 0,
                Err(_) => 1,
            },
            Err(()) => 64,
        }
    }

    fn parse_mode(args: impl Iterator<Item = OsString>) -> Result<Mode, ()> {
        let args: Vec<_> = args.collect();
        match args.as_slice() {
            [] => Ok(Mode::Run),
            [argument] if argument == "--shutdown" => Ok(Mode::Shutdown),
            [argument] if argument == "--launch-detached" => Ok(Mode::LaunchDetached),
            _ => Err(()),
        }
    }

    fn launch_detached() -> io::Result<()> {
        let system_root = std::env::var_os("SystemRoot")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "SystemRoot is not set"))?;
        let working_directory = PathBuf::from(system_root).join("System32");
        Command::new(std::env::current_exe()?)
            .current_dir(working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()?;
        Ok(())
    }

    fn request_shutdown() {
        let class_name = wide(WINDOW_CLASS);
        let window = unsafe { FindWindowW(class_name.as_ptr(), null()) };
        if window.is_null() {
            return;
        }
        unsafe {
            PostMessageW(window, WM_CLOSE, 0, 0);
        }
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(100));
            if unsafe { FindWindowW(class_name.as_ptr(), null()) }.is_null() {
                break;
            }
        }
    }

    fn run() {
        let _ = STATE.set(Mutex::new(TrayState::unavailable()));
        let mutex_name = wide(INSTANCE_MUTEX);
        let mutex = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
        if mutex.is_null() {
            return;
        }
        let _mutex = OwnedHandle(mutex);
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return;
        }

        let taskbar_created = wide("TaskbarCreated");
        TASKBAR_CREATED.store(
            unsafe { RegisterWindowMessageW(taskbar_created.as_ptr()) },
            Ordering::Relaxed,
        );

        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return;
        }
        let class_name = wide(WINDOW_CLASS);
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            hIcon: unsafe { LoadIconW(instance, APP_ICON_RESOURCE_ID as *const u16) },
            ..unsafe { zeroed() }
        };
        if unsafe { RegisterClassW(&window_class) } == 0 {
            return;
        }

        let window = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null(),
            )
        };
        if window.is_null() {
            return;
        }

        add_or_update_icon(window, NIM_ADD);
        refresh_status(window);
        unsafe {
            SetTimer(window, TIMER_ID, TIMER_INTERVAL_MS, None);
        }

        let mut message: MSG = unsafe { zeroed() };
        while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == TASKBAR_CREATED.load(Ordering::Relaxed) {
            add_or_update_icon(window, NIM_ADD);
            return 0;
        }

        match message {
            WM_TRAY_ICON => {
                let event = lparam as u32;
                if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                    show_menu(window);
                }
                0
            }
            WM_TIMER if wparam == TIMER_ID => {
                refresh_status(window);
                0
            }
            WM_QUERYENDSESSION => 1,
            WM_ENDSESSION => {
                if wparam != 0 {
                    unsafe {
                        DestroyWindow(window);
                    }
                }
                0
            }
            WM_CLOSE => {
                unsafe {
                    DestroyWindow(window);
                }
                0
            }
            WM_DESTROY => {
                unsafe {
                    KillTimer(window, TIMER_ID);
                }
                remove_icon(window);
                unsafe {
                    PostQuitMessage(0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    fn refresh_status(window: HWND) {
        let next = match client::status() {
            Ok(status) => state_from_status(&status),
            Err(_) => TrayState::unavailable(),
        };
        if let Some(state) = STATE.get()
            && let Ok(mut state) = state.lock()
        {
            *state = next;
        }
        add_or_update_icon(window, NIM_MODIFY);
    }

    fn state_from_status(status: &StatusResponse) -> TrayState {
        let pve_url = if status.overall == "ready"
            && status.components.proxmox == "ready"
            && status.components.tailscale_serve == "ready"
        {
            status
                .pve_url
                .as_deref()
                .and_then(|value| PveUrl::parse(value.to_owned()).ok())
                .map(|value| value.to_string())
        } else {
            None
        };

        let status_label = if pve_url.is_some() {
            "Estado: PVE HTTPS listo"
        } else if status.overall == "failed" {
            "Estado: error"
        } else if status.components.proxmox == "ready" {
            "Estado: preparando HTTPS"
        } else {
            "Estado: preparando PVE"
        };

        TrayState {
            status_label: status_label.into(),
            pve_url,
        }
    }

    fn show_menu(window: HWND) {
        refresh_status(window);
        let (status_label, pve_url) = STATE
            .get()
            .and_then(|state| state.lock().ok())
            .map(|state| (state.status_label.clone(), state.pve_url.clone()))
            .unwrap_or_else(|| {
                (
                    "Estado: servicio no disponible".into(),
                    Option::<String>::None,
                )
            });

        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            return;
        }

        append_menu(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            MENU_STATUS,
            &status_label,
        );
        append_menu(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            MENU_VERSION,
            &format!("Versión: {}", env!("CARGO_PKG_VERSION")),
        );
        append_menu(
            menu,
            if pve_url.is_some() {
                MF_STRING
            } else {
                MF_STRING | MF_DISABLED | MF_GRAYED
            },
            MENU_CONNECT,
            "Conectar",
        );

        let mut point = POINT { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut point);
            SetForegroundWindow(window);
        }
        let selected = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                point.x,
                point.y,
                0,
                window,
                null(),
            )
        };
        unsafe {
            DestroyMenu(menu);
            PostMessageW(window, WM_NULL, 0, 0);
        }

        if selected as usize == MENU_CONNECT
            && let Some(url) = pve_url
                .as_deref()
                .and_then(|value| PveUrl::parse(value.to_owned()).ok())
        {
            open_url(&url);
        }
    }

    fn append_menu(menu: HMENU, flags: u32, identifier: usize, label: &str) {
        let label = wide(label);
        unsafe {
            AppendMenuW(menu, flags, identifier, label.as_ptr());
        }
    }

    fn open_url(url: &str) {
        let operation = wide("open");
        let url = wide(url);
        unsafe {
            ShellExecuteW(
                null_mut(),
                operation.as_ptr(),
                url.as_ptr(),
                null(),
                null(),
                SW_SHOWNORMAL,
            );
        }
    }

    fn add_or_update_icon(window: HWND, operation: u32) {
        let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = window;
        data.uID = TRAY_ICON_ID;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WM_TRAY_ICON;
        let instance = unsafe { GetModuleHandleW(null()) };
        data.hIcon = unsafe { LoadIconW(instance, APP_ICON_RESOURCE_ID as *const u16) };

        let tooltip = STATE
            .get()
            .and_then(|state| state.lock().ok())
            .map(|state| format!("Quetzalcoatl — {}", state.status_label))
            .unwrap_or_else(|| "Quetzalcoatl".into());
        copy_wide(&mut data.szTip, &tooltip);

        unsafe {
            Shell_NotifyIconW(operation, &data);
        }
    }

    fn remove_icon(window: HWND) {
        let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = window;
        data.uID = TRAY_ICON_ID;
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }

    fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
        for (index, unit) in value.encode_utf16().take(N.saturating_sub(1)).enumerate() {
            target[index] = unit;
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn parse(values: &[&str]) -> Result<Mode, ()> {
            parse_mode(values.iter().map(OsString::from))
        }

        #[test]
        fn accepts_only_closed_tray_operations() {
            assert_eq!(parse(&[]), Ok(Mode::Run));
            assert_eq!(parse(&["--shutdown"]), Ok(Mode::Shutdown));
            assert_eq!(parse(&["--launch-detached"]), Ok(Mode::LaunchDetached));
            assert!(parse(&["--shutdown", "extra"]).is_err());
            assert!(parse(&["--unknown"]).is_err());
        }
    }
}

#[cfg(windows)]
fn main() {
    std::process::exit(windows_tray::entry());
}
