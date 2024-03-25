use anyhow::{anyhow, Context, Result};
use std::mem::size_of_val;
use std::ptr::{addr_of, addr_of_mut};
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS;

use windows::{
    core::{s, GUID},
    Win32::{
        Foundation::{GetLastError, BOOL, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleA,
            Power::{
                RegisterPowerSettingNotification, SetThreadExecutionState, ES_CONTINUOUS,
                ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, POWERBROADCAST_SETTING,
            },
            SystemInformation::GetTickCount64,
            Threading::GetCurrentProcess,
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetLastInputInfo, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE,
                KEYBDINPUT, KEYEVENTF_KEYUP, LASTINPUTINFO, MOUSEEVENTF_WHEEL, MOUSEINPUT,
                VK_ESCAPE,
            },
            WindowsAndMessaging::{
                CreateWindowExA, DefWindowProcA, DispatchMessageA, GetMessageA, LoadCursorW,
                RegisterClassA, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG,
                PBT_APMQUERYSUSPEND, REGISTER_NOTIFICATION_FLAGS, WINDOW_EX_STYLE,
                WM_POWERBROADCAST, WNDCLASSA, WS_OVERLAPPEDWINDOW,
            },
        },
    },
};

use crate::db_ops;
use crate::registry_ops;
use crate::win_mnts;

static MONITOR_GUID: &str = "6FE69556-704A-47A0-8F24-C28D936FDA47";
static DEFAULT_INTERVAL: u64 = 60;

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
}

fn send_key_input() -> Result<()> {
    let press_key: INPUT = INPUT {
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
    };

    let release_key: INPUT = INPUT {
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
    };
    let keys_list = [press_key, release_key];
    unsafe {
        for item in keys_list {
            let value = SendInput(&[item], size_of_val(&[item]).try_into()?);
            if value == 1 {
                info!("Sent KeyboardInput");
            } else {
                let err = GetLastError();
                error!("Failed to send KeyboardInput, last err {:?}", err);
                return Err(anyhow!("{:?}", err));
            }
        }
        Ok(())
    }
}

fn send_mouse_input(wheel_movement: i32) -> Result<()> {
    let mouse_input: MOUSEINPUT = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: wheel_movement.unsigned_abs(),
        dwFlags: MOUSEEVENTF_WHEEL,
        time: 0,
        dwExtraInfo: 0,
    };

    let input_struct: INPUT = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: mouse_input },
    };

    unsafe {
        let value = SendInput(&[input_struct], size_of_val(&[input_struct]).try_into()?);
        if value == 1 {
            info!("Sent MouseInput");
            Ok(())
        } else {
            let err = GetLastError();
            error!("Failed to send MouseInput, last err {:?}", err);
            Err(anyhow!("{:?}", err))
        }
    }
}

fn send_mixed_input() {
    let mut log_input: bool = false;
    let registry_data =
        registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::LogStatistics).last_data;
    if registry_data == registry_ops::RegistryState::Enabled.to_string() {
        log_input = true;
    }

    if log_input {
        let _ = send_to_db();
    } else {
        debug!("Did not log input => Logging to db is disabled");
    }

    let _ = send_mouse_input(1);
    let _ = send_key_input();
    let local_time = registry_ops::get_current_time();
    let _ = registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::LastRobotInput)
        .set_registry_data(&local_time);
}
/// Spawns a new window.
///
/// # Errors
///
/// This function will return an error if the window creation fails for any reason,
/// such as if the window class could not be registered, or if the window could not be created.
pub fn spawn_window() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleA(None)?;
        debug_assert!(instance.0 != 0);

        let window_class = s!("window");

        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class,
            s!("Smart Idler"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance,
            None,
        );
        let guid = GUID::from(MONITOR_GUID);
        match RegisterPowerSettingNotification(
            GetCurrentProcess(),
            addr_of!(guid),
            REGISTER_NOTIFICATION_FLAGS(0),
        ) {
            Ok(_) => {
                info!("Registered for power notifications");
            }
            Err(_) => {
                error!("Could not register for power notifications");
            }
        }

        let mut message = MSG::default();

        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }
        Ok(())
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
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
                        thread::spawn(|| send_mixed_input);
                        let _ = registry_ops::RegistrySetting::new(
                            &registry_ops::RegistryEntries::LastRobotInput,
                        )
                        .set_registry_data(&registry_ops::get_current_time());
                    }
                }
                LRESULT(0)
            }
            _ => {
                debug!("{} - {:?} - {:?}", message, wparam, lparam);
                DefWindowProcA(window, message, wparam, lparam)
            }
        }
    }
}

fn get_last_input() -> Option<u64> {
    let mut last_input = LASTINPUTINFO {
        cbSize: 0,
        dwTime: 0,
    };
    last_input.cbSize = if let Ok(val) = size_of_val(&last_input).try_into() {
        val
    } else {
        error!("Failed to get size of last input");
        return None;
    };

    unsafe {
        if GetLastInputInfo(addr_of_mut!(last_input)) != BOOL(1) {
            error!("Failed to get last input info");
            return None;
        }
        let total_ticks = GetTickCount64();
        Some(Duration::from_millis(total_ticks - u64::from(last_input.dwTime)).as_secs())
    }
}

fn send_to_db() -> Result<()> {
    let mut db = db_ops::RobotDatabase::new().context("Db is none")?;

    db.insert_to_db(&db_ops::RobotInput {
        input_time: registry_ops::get_current_time(),
        interval: registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::ForceInterval)
            .last_data
            .to_string(),
    })?;
    info!("db items: {}", db.number_of_items);
    Ok(())
}
/// The main idle loop.
///
/// # Errors
///
/// This function will return an error if there is a problem with the registry operations or
/// sending inputs to the system.
pub fn idle_loop() -> Result<()> {
    debug!("Start idle time thread");
    let mut registry_interval =
        registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::ForceInterval);
    let mut registry_maintenance =
        registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::StartMaintenance);

    let mut sleep_rotations = 0;
    let mut start_maintenance: bool;

    loop {
        let _ = registry_interval.update_local_data();
        let max_idle = match registry_interval.last_data.parse() {
            Ok(d) => d,
            Err(err) => {
                error!("Failed to get force interval data with err: {err} using default");
                DEFAULT_INTERVAL
            }
        };

        let idle_time = get_last_input().unwrap_or(0);

        let parted = max_idle * 94 / 100;
        if idle_time >= (max_idle * 94 / 100) {
            sleep_rotations = 0;
            send_mixed_input();
            if get_last_input() >= Some(idle_time) {
                send_mixed_input();
            }
            continue;
        }

        let _ = registry_maintenance.update_local_data();
        if registry_maintenance.last_data == registry_ops::RegistryState::Enabled.to_string() {
            win_mnts::Maintenance::change_state(&win_mnts::Commands::Start);
            start_maintenance = true;
        } else {
            start_maintenance = false;
        }

        thread::sleep(Duration::from_secs(parted));
        sleep_rotations += 1;
        if sleep_rotations >= 2 && start_maintenance {
            win_mnts::Maintenance::change_state(&win_mnts::Commands::Stop);
        }
    }
}

pub fn spawn_idle_threads() {
    thread::spawn(move || loop {
        let status = idle_loop();
        if status.is_err() {
            error!("Failed to run idle loop with err: {:?}", status);
        }
        thread::sleep(Duration::from_secs(60));
    });

    thread::spawn(move || {
        let _ = spawn_window();
    });
}
