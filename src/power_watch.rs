//! Background watcher that listens for Windows power and session events
//! (sleep → resume, display power-on, session unlock) and automatically
//! reapplies accent colors from the registry with staged delays so they
//! survive DWM / Winlogon / theme resets.
//!
//! Users never need to run this manually or keep a terminal open.
//! `syscolor` automatically configures both a background daemon and a
//! native Windows Task Scheduler wake trigger whenever colors are applied.

use crate::system_colors::SystemColorManager;

#[cfg(windows)]
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_DISPLAYCHANGE, WM_POWERBROADCAST, WNDCLASSW,
    },
};

// ── Raw Win32 FFI ───────────────────────────────────────────────────────────
#[cfg(windows)]
mod ffi {
    use windows::Win32::Foundation::HWND;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn CreateMutexW(
            lp_mutex_attributes: *const core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;

        pub fn OpenMutexW(
            dw_desired_access: u32,
            b_inherit_handle: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;

        pub fn CloseHandle(h_object: *mut core::ffi::c_void) -> i32;
        pub fn GetLastError() -> u32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn RegisterSuspendResumeNotification(
            h_recipient: HWND,
            flags: u32,
        ) -> *mut core::ffi::c_void;

        pub fn UnregisterSuspendResumeNotification(
            handle: *mut core::ffi::c_void,
        ) -> i32;
    }

    #[link(name = "wtsapi32")]
    unsafe extern "system" {
        pub fn WTSRegisterSessionNotification(
            hwnd: HWND,
            dw_flags: u32,
        ) -> i32;

        pub fn WTSUnRegisterSessionNotification(
            hwnd: HWND,
        ) -> i32;
    }

    pub const ERROR_ALREADY_EXISTS: u32 = 183;
    pub const SYNCHRONIZE: u32 = 0x0010_0000;
    pub const DEVICE_NOTIFY_WINDOW_HANDLE: u32 = 0x0000_0000;
    pub const NOTIFY_FOR_THIS_SESSION: u32 = 0;

    pub const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
    pub const WTS_SESSION_UNLOCK: usize = 0x8;
    pub const WTS_SESSION_LOGON: usize = 0x5;
}

/// Session-scoped named mutex to ensure only one watcher runs per user session.
const MUTEX_NAME: &str = "Local\\SyscolorPowerWatcher";

// Power broadcast event constants from Win32 SDK.
#[cfg(windows)]
const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
#[cfg(windows)]
const PBT_APMRESUMESUSPEND: u32 = 0x0007;
#[cfg(windows)]
const PBT_POWERSETTINGCHANGE: u32 = 0x8013;

// ── Public API ──────────────────────────────────────────────────────────────

/// Checks whether the background power-watcher daemon is currently running.
#[cfg(windows)]
pub fn is_watcher_running() -> bool {
    let name_wide: Vec<u16> = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = ffi::OpenMutexW(ffi::SYNCHRONIZE, 0, name_wide.as_ptr());
        if !handle.is_null() {
            ffi::CloseHandle(handle);
            true
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
pub fn is_watcher_running() -> bool {
    false
}

/// Ensures both the background power-watcher daemon and the native Task Scheduler
/// wake trigger are active.
#[cfg(windows)]
pub fn ensure_watcher_running() {
    use std::os::windows::process::CommandExt;

    // 1. Ensure Windows Startup (HKCU\...\Run) entry exists
    let _ = set_autostart(true);

    // 2. Ensure native Task Scheduler trigger (Event ID 1 from Power-Troubleshooter)
    ensure_task_scheduler_task();

    // 3. Fast-path: check if daemon is already alive
    if is_watcher_running() {
        tracing::debug!("power watcher daemon is already running");
        return;
    }

    // 4. Locate current executable
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%e, "cannot locate syscolor binary to spawn watcher");
            return;
        }
    };

    // CREATE_NO_WINDOW (0x0800_0000)
    // CREATE_BREAKAWAY_FROM_JOB (0x0100_0000)
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

    let res = std::process::Command::new(&exe)
        .arg("watch")
        .creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    if res.is_err() {
        let _ = std::process::Command::new(&exe)
            .arg("watch")
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

#[cfg(not(windows))]
pub fn ensure_watcher_running() {}

/// Registers a native Windows Task Scheduler task that triggers `syscolor apply`
/// directly when Windows kernel event `Microsoft-Windows-Power-Troubleshooter` Event 1 fires.
#[cfg(windows)]
pub fn ensure_task_scheduler_task() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    let tr_arg = format!("\"{}\" apply", exe.to_string_lossy());
    let _ = std::process::Command::new("schtasks")
        .args([
            "/create",
            "/tn",
            "SyscolorWakeRefresh",
            "/tr",
            &tr_arg,
            "/sc",
            "ONEVENT",
            "/ec",
            "System",
            "/mo",
            "*[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1]]",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Registers or unregisters `syscolor watch` in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
#[cfg(windows)]
pub fn set_autostart(enable: bool) -> crate::errors::Result<()> {
    use winreg::{enums::{HKEY_CURRENT_USER, KEY_SET_VALUE}, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu.create_subkey_with_flags(run_path, KEY_SET_VALUE)?;

    if enable {
        let exe = std::env::current_exe()?;
        let cmd = format!("\"{}\" watch", exe.to_string_lossy());
        key.set_value("SyscolorWatcher", &cmd)?;
    } else {
        let _ = key.delete_value("SyscolorWatcher");
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn set_autostart(_enable: bool) -> crate::errors::Result<()> {
    Ok(())
}

/// Checks whether `SyscolorWatcher` is registered in `HKCU\...\Run`.
#[cfg(windows)]
pub fn is_autostart_enabled() -> bool {
    use winreg::{enums::{HKEY_CURRENT_USER, KEY_READ}, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_READ) {
        let val: Result<String, _> = key.get_value("SyscolorWatcher");
        val.is_ok()
    } else {
        false
    }
}

#[cfg(not(windows))]
pub fn is_autostart_enabled() -> bool {
    false
}

// ── Watcher Daemon Implementation ──────────────────────────────────────────

/// Runs the blocking power watcher loop.
///
/// Creates a top-level invisible window to receive broadcast power events and
/// registers for session unlock notifications. Reapplies colors with staged delays
/// to overcome DWM / theme resets.
#[cfg(windows)]
pub fn run_power_watcher(verbose: bool) -> crate::errors::Result<()> {
    // ── 1. Acquire singleton mutex ──────────────────────────────────────────
    let name_wide: Vec<u16> = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let _mutex_handle = unsafe {
        let handle = ffi::CreateMutexW(std::ptr::null(), 1, name_wide.as_ptr());
        if handle.is_null() {
            return Err(crate::errors::Error::Win32(
                "CreateMutexW failed for power watcher singleton".into(),
            ));
        }
        if ffi::GetLastError() == ffi::ERROR_ALREADY_EXISTS {
            ffi::CloseHandle(handle);
            return Ok(());
        }
        handle
    };

    // ── 2. Create invisible top-level window ─────────────────────────────────
    // Using HWND::default() (no parent) creates a top-level window that receives
    // system-wide broadcast messages (unlike message-only windows).
    unsafe {
        let class_name: Vec<u16> = "SyscolorPowerWatch\0".encode_utf16().collect();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(power_wnd_proc),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return Err(crate::errors::Error::Win32(
                "RegisterClassW failed for power watcher".into(),
            ));
        }

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR::null(),
            WINDOW_STYLE::default(), // No WS_VISIBLE: completely hidden
            0,
            0,
            0,
            0,
            HWND::default(), // Top-level window receives system broadcasts!
            None,
            None,
            None,
        )
        .map_err(|e| crate::errors::Error::Win32(format!("CreateWindowExW failed: {e}")))?;

        if hwnd.is_invalid() {
            return Err(crate::errors::Error::Win32(
                "CreateWindowExW returned invalid HWND".into(),
            ));
        }

        // ── 3. Register for suspend/resume notifications ─────────────────────
        let power_notify = ffi::RegisterSuspendResumeNotification(
            hwnd,
            ffi::DEVICE_NOTIFY_WINDOW_HANDLE,
        );

        // ── 4. Register for session unlock / logon notifications ─────────────
        let _ = ffi::WTSRegisterSessionNotification(hwnd, ffi::NOTIFY_FOR_THIS_SESSION);

        if verbose {
            tracing::info!("syscolor power watcher active (PID: {})", std::process::id());
        }

        // ── 5. Message pump (waits for events, 0% CPU) ──────────────────────
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = ffi::WTSUnRegisterSessionNotification(hwnd);
        if !power_notify.is_null() {
            ffi::UnregisterSuspendResumeNotification(power_notify);
        }
    }

    Ok(())
}

/// Spawns a background thread to reapply colors across multiple stages (50ms to 3s).
/// This overcomes Windows DWM and Theme Engine resetting colors during wake/unlock.
#[cfg(windows)]
fn schedule_staged_reapply(reason: &'static str) {
    tracing::info!(reason, "triggering staged color reapply");
    std::thread::spawn(move || {
        // Multi-stage retry sequence:
        // - 50ms: immediate catch
        // - 250ms: after display power-on
        // - 700ms: during Winlogon / lock screen transition
        // - 1500ms: after user unlock / DWM desktop switch
        // - 3000ms: final safety latch
        for delay in [50, 250, 700, 1500, 3000] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            let manager = SystemColorManager;
            if let Err(e) = manager.apply_raw() {
                tracing::warn!(%e, reason, delay, "failed to reapply colors");
            }
        }
        tracing::debug!(reason, "completed staged color reapply");
    });
}

/// Window procedure that catches power, display, and session unlock events.
#[cfg(windows)]
unsafe extern "system" fn power_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_POWERBROADCAST => {
            let event = wparam.0 as u32;
            if event == PBT_APMRESUMEAUTOMATIC
                || event == PBT_APMRESUMESUSPEND
                || event == PBT_POWERSETTINGCHANGE
            {
                schedule_staged_reapply("power resume");
            }
        }
        ffi::WM_WTSSESSION_CHANGE => {
            let event = wparam.0;
            if event == ffi::WTS_SESSION_UNLOCK || event == ffi::WTS_SESSION_LOGON {
                schedule_staged_reapply("session unlock");
            }
        }
        WM_DISPLAYCHANGE => {
            schedule_staged_reapply("display change");
        }
        _ => {}
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn run_power_watcher(_verbose: bool) -> crate::errors::Result<()> {
    Err(crate::errors::Error::UnsupportedPlatform(
        "power watching requires Windows",
    ))
}
