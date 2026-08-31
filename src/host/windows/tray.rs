use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, LoadIconW,
    MB_ICONINFORMATION, MB_OK, MSG, MessageBoxW, PostQuitMessage, RegisterClassW, SW_HIDE,
    ShowWindow, TranslateMessage, WM_APP, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSW,
};

use crate::error::GnxError;
use crate::report::StatusReport;

const ICON_ID: usize = 2;
const TRAY_ID: u32 = 1;
const WM_GNX_TRAY: u32 = WM_APP + 1;
const CLASS_NAME: &str = "QuetzalcoatlNextTrayWindow";

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn run(config_path: PathBuf) -> Result<(), GnxError> {
    crate::logs::event("info", "tray", "start", "Iniciando bandeja GNX");
    // SAFETY: this hidden internal mode owns its UI thread and global callback state.
    unsafe {
        let class_name = wide(CLASS_NAME);
        if !FindWindowW(class_name.as_ptr(), null()).is_null() {
            return Ok(());
        }
        CONFIG_PATH.set(config_path).map_err(|_| {
            GnxError::new(
                "TRAY_ALREADY_RUNNING",
                "tray",
                "initialize",
                "La bandeja ya se inicializó en este proceso.",
                "Conserve una sola instancia de la bandeja GNX.",
                false,
                20,
            )
        })?;
        let console = GetConsoleWindow();
        if !console.is_null() {
            ShowWindow(console, SW_HIDE);
            FreeConsole();
        }

        let instance = GetModuleHandleW(null());
        if instance.is_null() {
            return Err(last_error("tray_module"));
        }
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        if RegisterClassW(&window_class) == 0 {
            return Err(last_error("tray_class"));
        }
        let window = CreateWindowExW(
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
        );
        if window.is_null() {
            return Err(last_error("tray_window"));
        }

        let mut icon_data: NOTIFYICONDATAW = zeroed();
        icon_data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        icon_data.hWnd = window;
        icon_data.uID = TRAY_ID;
        icon_data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        icon_data.uCallbackMessage = WM_GNX_TRAY;
        icon_data.hIcon = LoadIconW(instance, ICON_ID as _);
        if icon_data.hIcon.is_null() {
            return Err(last_error("tray_icon"));
        }
        copy_wide(&mut icon_data.szTip, &tooltip());
        if Shell_NotifyIconW(NIM_ADD, &icon_data) == 0 {
            return Err(last_error("tray_add"));
        }
        crate::logs::event("info", "tray", "ready", "Icono agregado a la bandeja");

        let mut message: MSG = zeroed();
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result == -1 {
                Shell_NotifyIconW(NIM_DELETE, &icon_data);
                return Err(last_error("tray_message"));
            }
            if result == 0 {
                break;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        Shell_NotifyIconW(NIM_DELETE, &icon_data);
    }
    crate::logs::event("info", "tray", "stop", "Bandeja GNX cerrada");
    Ok(())
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_GNX_TRAY if lparam as u32 == WM_LBUTTONDBLCLK || lparam as u32 == WM_RBUTTONUP => {
            show_status(window);
            0
        }
        WM_DESTROY => {
            // SAFETY: called by the Win32 UI thread that owns the message loop.
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => {
            // SAFETY: unhandled messages are delegated to the system window procedure.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
    }
}

fn tooltip() -> String {
    let stage = crate::state::OperationalState::load(&crate::state::default_state_path())
        .ok()
        .flatten()
        .map(|state| state.stage.as_str())
        .unwrap_or("pending");
    format!("Quetzalcoatl Next — {stage}")
}

fn show_status(window: HWND) {
    let path = CONFIG_PATH.get();
    let body = match path.and_then(|path| StatusReport::collect(path).ok()) {
        Some(report) => format!(
            "Estado: {}\nPodman Machine: {}\nDocktail: {}\nProxmox: {}\nInfra: {}",
            report.stage, report.machine, report.docktail, report.proxmox, report.infra
        ),
        None => "GNX aún no tiene un estado legible. Ejecute gnx doctor.".to_string(),
    };
    let title = wide("Quetzalcoatl Next");
    let body = wide(&body);
    // SAFETY: strings are NUL-terminated and the owner window remains valid.
    unsafe {
        MessageBoxW(
            window,
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

fn copy_wide(destination: &mut [u16], value: &str) {
    for (target, source) in destination
        .iter_mut()
        .zip(value.encode_utf16().chain(std::iter::once(0)))
    {
        *target = source;
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn last_error(operation: &'static str) -> GnxError {
    GnxError::new(
        "TRAY_WINDOWS_FAILED",
        "tray",
        operation,
        std::io::Error::last_os_error().to_string(),
        "Cierre otras instancias de la bandeja o ejecute gnx repair.",
        true,
        20,
    )
}
