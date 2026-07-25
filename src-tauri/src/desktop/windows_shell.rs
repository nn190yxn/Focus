use windows_sys::{
    core::BOOL,
    Win32::{
        Foundation::{GetLastError, SetLastError, ERROR_SUCCESS, HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, FindWindowExW, FindWindowW, IsWindow, SendMessageTimeoutW, SetParent,
            SMTO_NORMAL,
        },
    },
};

use crate::desktop::shell_attachment::{ShellAttachError, ShellFallbackReason, ShellHostAdapter};

const SPAWN_WORKER_MESSAGE: u32 = 0x052C;

#[derive(Default)]
pub struct WindowsShellAdapter;

impl ShellHostAdapter for WindowsShellAdapter {
    type Host = usize;

    fn discover_host(&self) -> Result<Self::Host, ShellAttachError> {
        let progman_class = wide_null("Progman");
        let progman = unsafe { FindWindowW(progman_class.as_ptr(), std::ptr::null()) };
        if progman.is_null() {
            return Err(last_host_error("Progman window was not found"));
        }
        unsafe {
            SendMessageTimeoutW(
                progman,
                SPAWN_WORKER_MESSAGE,
                0 as WPARAM,
                0 as LPARAM,
                SMTO_NORMAL,
                1_000,
                std::ptr::null_mut(),
            );
        }

        let mut host = None;
        unsafe { SetLastError(ERROR_SUCCESS) };
        let enumerated = unsafe {
            EnumWindows(
                Some(find_worker_window),
                (&mut host as *mut Option<HWND>) as LPARAM,
            )
        };
        if enumerated == 0 && host.is_none() && unsafe { GetLastError() } != ERROR_SUCCESS {
            return Err(last_host_error("desktop window enumeration failed"));
        }
        host.map(|handle| handle as usize)
            .ok_or_else(|| host_error("desktop WorkerW host was not found"))
    }

    fn is_host_valid(&self, host: Self::Host) -> bool {
        unsafe { IsWindow(host as HWND) != 0 }
    }

    fn attach(&self, window: usize, host: Self::Host) -> Result<(), ShellAttachError> {
        set_parent(window as HWND, host as HWND).map_err(|message| {
            ShellAttachError::new(ShellFallbackReason::AttachmentFailed, message)
        })
    }

    fn detach(&self, window: usize) -> Result<(), ShellAttachError> {
        set_parent(window as HWND, std::ptr::null_mut()).map_err(|message| {
            ShellAttachError::new(ShellFallbackReason::DetachmentFailed, message)
        })
    }
}

unsafe extern "system" fn find_worker_window(window: HWND, context: LPARAM) -> BOOL {
    let shell_view_class = wide_null("SHELLDLL_DefView");
    let has_shell_view = unsafe {
        !FindWindowExW(
            window,
            std::ptr::null_mut(),
            shell_view_class.as_ptr(),
            std::ptr::null(),
        )
        .is_null()
    };
    if has_shell_view {
        let worker_class = wide_null("WorkerW");
        let worker = unsafe {
            FindWindowExW(
                std::ptr::null_mut(),
                window,
                worker_class.as_ptr(),
                std::ptr::null(),
            )
        };
        if !worker.is_null() {
            let host = context as *mut Option<HWND>;
            if !host.is_null() {
                unsafe { *host = Some(worker) };
                return 0;
            }
        }
    }
    1
}

fn set_parent(window: HWND, parent: HWND) -> Result<(), String> {
    unsafe { SetLastError(ERROR_SUCCESS) };
    let previous_parent = unsafe { SetParent(window, parent) };
    let error = unsafe { GetLastError() };
    if previous_parent.is_null() && error != ERROR_SUCCESS {
        Err(std::io::Error::from_raw_os_error(error as i32).to_string())
    } else {
        Ok(())
    }
}

fn host_error(message: impl Into<String>) -> ShellAttachError {
    ShellAttachError::new(ShellFallbackReason::HostNotFound, message)
}

fn last_host_error(context: &str) -> ShellAttachError {
    let error = unsafe { GetLastError() };
    host_error(format!(
        "{context}: {}",
        std::io::Error::from_raw_os_error(error as i32)
    ))
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
