use std::{mem::size_of_val, sync::Mutex, thread, time::Duration};

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;

use windows::{
    core::{w, GUID},
    Win32::{
        Foundation::{GetLastError, BOOL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Power::{
                RegisterPowerSettingNotification, SetThreadExecutionState, ES_CONTINUOUS,
                ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, ES_USER_PRESENT, POWERBROADCAST_SETTING,
            },
            SystemInformation::GetTickCount64,
            Threading::GetCurrentProcess,
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetLastInputInfo, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE,
                KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, LASTINPUTINFO, MOUSEEVENTF_WHEEL,
                MOUSEINPUT, VK_ESCAPE,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW,
                RegisterClassW, CS_HREDRAW, CS_VREDRAW, HWND_MESSAGE, IDC_ARROW, MSG,
                PBT_APMQUERYSUSPEND, REGISTER_NOTIFICATION_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_POWERBROADCAST, WNDCLASSW,
            },
        },
    },
};

use crate::db_ops;
use crate::registry_ops;
use crate::win_mitigations;

static MONITOR_GUID: &str = "6FE69556-704A-47A0-8F24-C28D936FDA47";

#[derive(Debug, PartialEq, Eq)]
enum InputType {
    Mouse,
    Keyboard,
}

#[non_exhaustive]
pub struct ExecState;

impl ExecState {
    #[inline]
    pub fn start() {
        unsafe {
            let state =
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            info!("{:?} - ENABLE", state);
        }
    }
    #[inline]
    pub fn stop() {
        unsafe {
            let state = SetThreadExecutionState(ES_CONTINUOUS);
            info!("{:?} - DISABLE", state);
        }
    }

    pub fn user_present() {
        unsafe {
            let state = SetThreadExecutionState(ES_USER_PRESENT);
            info!("{:?} - USER_PRESENT", state);
        }
    }
}

fn send_key_input() -> Result<()> {
    let keys_list = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_ESCAPE,
                    wScan: 1,
                    dwFlags: KEYBD_EVENT_FLAGS::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_ESCAPE,
                    wScan: 1,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    for item in keys_list {
        let value = unsafe { SendInput(&[item], size_of_val(&[item]).try_into()?) };
        if value == 1 {
            info!("Sent KeyboardInput");
        } else {
            let err = unsafe { GetLastError() };
            error!("Failed to send KeyboardInput, last err {:?}", err);
            return Err(anyhow!("{:?}", err));
        }
    }
    Ok(())
}

fn send_mouse_input(wheel_movement: i32) -> Result<()> {
    let input_struct: INPUT = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: wheel_movement.unsigned_abs(),
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    if unsafe { SendInput(&[input_struct], size_of_val(&[input_struct]).try_into()?) } == 1 {
        info!("Sent MouseInput");
        Ok(())
    } else {
        let err = unsafe { GetLastError() };
        error!("Failed to send MouseInput, last err {:?}", err);
        Err(anyhow!("{:?}", err))
    }
}

static REGISTRY_LOG_STATISTICS: Lazy<Mutex<registry_ops::RegistrySetting>> = Lazy::new(|| {
    Mutex::new(registry_ops::RegistrySetting::new(
        &registry_ops::RegistryEntries::LogStatistics,
    ))
});

static REGISTRY_ROBOT_INPUT: Lazy<Mutex<registry_ops::RegistrySetting>> = Lazy::new(|| {
    Mutex::new(registry_ops::RegistrySetting::new(
        &registry_ops::RegistryEntries::LastRobotInput,
    ))
});

fn send_mixed_input(input_type: &InputType) {
    let mut log_input = false;
    if REGISTRY_LOG_STATISTICS.lock().unwrap().is_enabled() {
        log_input = true;
    }

    if log_input {
        match send_to_db() {
            Ok(()) => {
                debug!("Logged input");
            }
            Err(err) => {
                error!("Failed to log input with err: {:?}", err);
            }
        }
    } else {
        debug!("Did not log input => Logging to db is disabled");
    }

    if *input_type == InputType::Mouse {
        let _ = send_mouse_input(1);
    } else {
        let _ = send_key_input();
    }
    let _ = REGISTRY_ROBOT_INPUT
        .lock()
        .unwrap()
        .set_registry_data(&registry_ops::get_current_time());
}
/// Spawns a new window.
///
/// # Errors
///
/// This function will return an error if the window creation fails for any reason,
/// such as if the window class could not be registered, or if the window could not be created.
#[allow(clippy::missing_safety_doc)]
pub fn spawn_window() -> Result<()> {
    let instance: HINSTANCE = unsafe { GetModuleHandleW(None) }?.into();

    let window_class = w!("window");

    let wc = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        hInstance: instance,
        lpszClassName: window_class,

        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&wc) };
    debug_assert!(atom != 0);

    unsafe {
        match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            window_class,
            w!("LsWindow"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            None,
        ) {
            Ok(_) => {
                info!("Window created");
            }
            Err(err) => {
                error!("Failed to create window with err: {:?}", err);
            }
        }
    };
    let guid = GUID::from(MONITOR_GUID);
    match unsafe {
        RegisterPowerSettingNotification(
            GetCurrentProcess(),
            std::ptr::from_ref(&guid),
            REGISTER_NOTIFICATION_FLAGS(0),
        )
    } {
        Ok(hp) => {
            info!("Registered for power notifications: {:?}", hp);
        }
        Err(err) => {
            error!("Could not register for power notifications, err: {:?}", err);
        }
    }

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0).into() } {
        unsafe { DispatchMessageW(&message) };
    }
    Ok(())
}

unsafe extern "system" fn wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        PBT_APMQUERYSUSPEND => {
            debug!("PBT_APMQUERYSUSPEND");
            LRESULT(0)
        }
        WM_POWERBROADCAST => {
            debug!("WM_POWERBROADCAST: {:?} - {:?}", wparam, lparam);
            if wparam == WPARAM(32787) {
                let st: &mut POWERBROADCAST_SETTING =
                    &mut *(lparam.0 as *mut POWERBROADCAST_SETTING);
                let guid = GUID::from(MONITOR_GUID);
                if st.PowerSetting == guid && st.Data == [0] {
                    send_mixed_input(&InputType::Mouse);
                    let _ = REGISTRY_ROBOT_INPUT
                        .lock()
                        .unwrap()
                        .set_registry_data(&registry_ops::get_current_time());
                }
            }
            LRESULT(0)
        }
        _ => {
            debug!(
                "msg-only message: {} - {:?} - {:?}",
                message, wparam, lparam
            );
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

fn get_last_input() -> Option<u64> {
    let mut last_input = LASTINPUTINFO::default();

    last_input.cbSize = if let Ok(val) = size_of_val(&last_input).try_into() {
        val
    } else {
        error!("Failed to get size of last input");
        return None;
    };
    let total_ticks;
    unsafe {
        if GetLastInputInfo(std::ptr::from_mut(&mut last_input)) != BOOL(1) {
            error!("Failed to get last input info, {:?}", GetLastError());
            return None;
        }
        total_ticks = GetTickCount64();
    }
    Some(Duration::from_millis(total_ticks - u64::from(last_input.dwTime)).as_secs())
}

static REGISTRY_FORCE_INTERVAL: Lazy<Mutex<registry_ops::RegistrySetting>> = Lazy::new(|| {
    Mutex::new(registry_ops::RegistrySetting::new(
        &registry_ops::RegistryEntries::ForceInterval,
    ))
});

fn send_to_db() -> Result<()> {
    let mut db = db_ops::RobotDatabase::new().context("Db is none")?;
    db.insert_to_db(&db_ops::RobotInput {
        interval: REGISTRY_FORCE_INTERVAL.lock().unwrap().last_data.clone(),
        ..Default::default()
    })?;
    info!("db items: {:?}", db.number_of_items.get());
    Ok(())
}
/// The main idle loop.
///
/// # Errors
///
/// This function will return an error if there is a problem with the registry operations or
/// sending inputs to the system.
#[allow(clippy::missing_panics_doc)]
pub fn idle_loop() -> Result<()> {
    debug!("Start idle time thread");
    loop {
        let _ = REGISTRY_FORCE_INTERVAL.lock().unwrap().update_local_data();
        let max_idle: u64 = match REGISTRY_FORCE_INTERVAL.lock().unwrap().last_data.parse() {
            Ok(data) => data,
            Err(err) => {
                error!("Failed to parse force interval data with err: {err}");
                thread::sleep(Duration::from_secs(60));
                continue;
            }
        };

        if max_idle < 60 {
            info!("Force interval is less than 60 seconds, skipping");
            thread::sleep(Duration::from_secs(60));
            continue;
        }

        let idle_time = get_last_input().unwrap_or(0);
        if idle_time >= (max_idle * 94 / 100) {
            ExecState::user_present();
            send_mixed_input(&InputType::Mouse);
            if get_last_input() >= Some(idle_time) {
                send_mixed_input(&InputType::Keyboard);
                thread::sleep(Duration::from_secs(10));
            }
            if get_last_input() >= Some(idle_time) {
                error!("Failed to reset idle time, skipping");
            }
            continue;
        }
        thread::sleep(Duration::from_secs(max_idle * 94 / 100));
    }
}

pub fn spawn_idle_threads() {
    thread::spawn(move || {
        win_mitigations::hide_current_thread_from_debuggers();
        loop {
            let status = idle_loop();
            if status.is_err() {
                error!("Failed to run idle loop with err: {:?}", status);
            }
            thread::sleep(Duration::from_secs(60));
        }
    });

    thread::spawn(move || {
        win_mitigations::hide_current_thread_from_debuggers();
        info!("Spawn window status: {:?}", spawn_window());
    });
}
