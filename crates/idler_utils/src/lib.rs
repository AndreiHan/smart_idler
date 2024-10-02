use std::{mem::size_of_val, thread, time::Duration};
use tracing::{debug, error, info};

use anyhow::{anyhow, Result};

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
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
                LoadCursorW, RegisterClassW, TranslateMessage, UnregisterClassW, CS_HREDRAW,
                CS_VREDRAW, HWND_MESSAGE, IDC_ARROW, MSG, PBT_APMQUERYSUSPEND,
                REGISTER_NOTIFICATION_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE, WM_POWERBROADCAST,
                WNDCLASSW,
            },
        },
    },
};

use registry_ops::get_current_time;

static MONITOR_GUID: &str = "6FE69556-704A-47A0-8F24-C28D936FDA47";

const MOUSE_INPUT: INPUT = INPUT {
    r#type: INPUT_MOUSE,
    Anonymous: INPUT_0 {
        mi: MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 1,
            dwFlags: MOUSEEVENTF_WHEEL,
            time: 0,
            dwExtraInfo: 0,
        },
    },
};

const KEYBOARD_INPUT: [INPUT; 2] = [
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_ESCAPE,
                wScan: 1,
                dwFlags: KEYBD_EVENT_FLAGS(0),
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    for item in KEYBOARD_INPUT {
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

fn send_mouse_input() -> Result<()> {
    if unsafe { SendInput(&[MOUSE_INPUT], size_of_val(&[MOUSE_INPUT]).try_into()?) } == 1 {
        info!("Sent MouseInput");
        Ok(())
    } else {
        let err = unsafe { GetLastError() };
        error!("Failed to send MouseInput, last err {:?}", err);
        Err(anyhow!("{:?}", err))
    }
}

fn send_mixed_input(input_type: InputType) {
    if input_type == InputType::Mouse {
        let _ = send_mouse_input();
    } else {
        let _ = send_key_input();
    }
    let _ = cell_data::REGISTRY_ROBOT_INPUT
        .lock()
        .unwrap()
        .set_registry_data(get_current_time());
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

    let window_handle: HWND;
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
            Ok(hnd) => {
                info!("Window created");
                window_handle = hnd;
            }
            Err(err) => {
                error!("Failed to create window with err: {:?}", err);
                return Err(err.into());
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
        unsafe {
            if !TranslateMessage(&message).as_bool() {
                continue;
            }
            DispatchMessageW(&message);
        }
    }
    unsafe {
        DestroyWindow(window_handle)?;
        UnregisterClassW(window_class, instance)?;
    }
    Ok(())
}

unsafe extern "system" fn wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == PBT_APMQUERYSUSPEND {
        debug!("PBT_APMQUERYSUSPEND");
        LRESULT(0)
    } else if message == WM_POWERBROADCAST {
        debug!("WM_POWERBROADCAST: {:?} - {:?}", wparam, lparam);
        if wparam == WPARAM(32787) {
            let st: &mut POWERBROADCAST_SETTING = &mut *(lparam.0 as *mut POWERBROADCAST_SETTING);
            let guid = GUID::from(MONITOR_GUID);
            if st.PowerSetting == guid && st.Data == [0] {
                send_mixed_input(InputType::Mouse);
                let _ = cell_data::REGISTRY_ROBOT_INPUT
                    .lock()
                    .unwrap()
                    .set_registry_data(get_current_time());
            }
        }
        LRESULT(0)
    } else {
        debug!(
            "msg-only message: {} - {:?} - {:?}",
            message, wparam, lparam
        );
        DefWindowProcW(window, message, wparam, lparam)
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

/// The main idle loop.
///
/// # Errors
///
/// This function will return an error if there is a problem with the registry operations or
/// sending inputs to the system.
#[allow(clippy::missing_panics_doc)]
pub fn idle_loop() -> Result<()> {
    debug!("Start idle time thread");

    let mut max_idle: u64 = 0;
    let mut same_data_runs: u32 = 6;

    loop {
        debug!("Same data runs: {same_data_runs}");
        if same_data_runs >= 6 {
            debug!("Same data runs exceeded, resetting max_idle");
            max_idle = match cell_data::REGISTRY_FORCE_INTERVAL
                .lock()
                .unwrap()
                .last_data
                .parse()
            {
                Ok(data) => data,
                Err(err) => {
                    error!("Failed to parse force interval data with err: {err}");
                    0
                }
            };
            same_data_runs = 0;
        }
        same_data_runs += 1;
        debug!("Max idle: {max_idle}");

        if max_idle < 60 {
            info!("Force interval is less than 60 seconds, setting to 60 seconds");
            let status = cell_data::REGISTRY_FORCE_INTERVAL
                .lock()
                .unwrap()
                .set_registry_data("60".to_string());
            if status.is_err() {
                error!("Failed to set force interval to 60 seconds");
            }
            max_idle = 60;
        }

        let idle_time = get_last_input().unwrap_or(0);
        if idle_time >= (max_idle * 94 / 100) {
            ExecState::user_present();
            send_mixed_input(InputType::Mouse);
            if get_last_input() >= Some(idle_time) {
                send_mixed_input(InputType::Keyboard);
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
        mitigations::hide_current_thread_from_debuggers();
        thread::sleep(Duration::from_secs(10));
        loop {
            let status = idle_loop();
            if status.is_err() {
                error!("Failed to run idle loop with err: {:?}", status);
            }
            thread::sleep(Duration::from_secs(60));
        }
    });

    thread::spawn(move || {
        mitigations::hide_current_thread_from_debuggers();
        let _ = spawn_window();
    });
}
