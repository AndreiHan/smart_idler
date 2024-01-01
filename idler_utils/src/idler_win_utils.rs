use std::mem::size_of_val;
use std::ptr::{addr_of, addr_of_mut};
use std::thread;
use std::time::Duration;

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{LibraryLoader::*, Power::*, SystemInformation::*, Threading::*},
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

use crate::db_ops;
use crate::registry_ops;
use crate::win_mnts;

static MONITOR_GUID: &str = "6FE69556-704A-47A0-8F24-C28D936FDA47";
static DEFAULT_INTERVAL: u64 = 60;

pub struct ExecState;

impl ExecState {
    pub fn start() {
        unsafe {
            let state =
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            info!("{:?} - ENABLE", state);
        }
    }
    pub fn stop() {
        unsafe {
            let state = SetThreadExecutionState(ES_CONTINUOUS);
            info!("{:?} - DISABLE", state);
        }
    }
}

fn send_key_input() {
    let press_key: INPUT = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_ESCAPE,
                wScan: 1,
                dwFlags: Default::default(),
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
            let value = SendInput(&[item], size_of_val(&[item]) as i32);
            if value == 1 {
                info!("Sent KeyboardInput");
            } else {
                error!(
                    "Failed to send KeyboardInput, last err {:?}",
                    GetLastError()
                );
            }
        }
    }
}

fn send_mouse_input(wheel_movement: i32) {
    let mouse_input: MOUSEINPUT = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: wheel_movement as u32,
        dwFlags: MOUSEEVENTF_WHEEL,
        time: 0,
        dwExtraInfo: 0,
    };

    let input_struct: INPUT = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: mouse_input },
    };

    unsafe {
        let value = SendInput(&[input_struct], size_of_val(&[input_struct]) as i32);
        if value == 1 {
            info!("Sent MouseInput");
        } else {
            error!("Failed to send MouseInput, last err {:?}", GetLastError());
        }
    }
}

fn send_mixed_input() {
    let mut log_input: bool = false;
    let registry_data =
        registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::LogStatistics).last_data;
    if registry_data == registry_ops::RegistryState::Enabled.to_string() {
        log_input = true;
    }

    if log_input {
        send_to_db();
    } else {
        debug!("Did not log input => Logging to db is disabled")
    }

    send_mouse_input(1);
    send_key_input();
    let local_time = registry_ops::get_current_time();
    registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::LastRobotInput)
        .set_registry_data(&local_time);
}

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
        match RegisterPowerSettingNotification(GetCurrentProcess(), addr_of!(guid), 0) {
            Ok(_) => {
                info!("Registered for power notifications")
            }
            Err(_) => {
                error!("Could not register for power notifications")
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
                        registry_ops::RegistrySetting::new(
                            registry_ops::RegistryEntries::LastRobotInput,
                        )
                        .set_registry_data(&registry_ops::get_current_time())
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

fn get_last_input() -> u64 {
    let mut last_input = LASTINPUTINFO {
        cbSize: 0,
        dwTime: 0,
    };
    last_input.cbSize = size_of_val(&last_input) as u32;

    unsafe {
        if GetLastInputInfo(addr_of_mut!(last_input)) != BOOL(1) {
            error!("Failed to get last input info");
            return 0;
        }
        let total_ticks = GetTickCount64();
        Duration::from_millis(total_ticks - last_input.dwTime as u64).as_secs()
    }
}

fn send_to_db() {
    let db = db_ops::RobotDatabase::new();
    if db.is_none() {
        return;
    }
    let mut db: db_ops::RobotDatabase = db.unwrap();
    db.insert_to_db(&db_ops::RobotInput {
        input_time: registry_ops::get_current_time(),
        interval: registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::ForceInterval)
            .last_data,
    });
    info!("db items: {}", db.number_of_items);
}

pub fn idle_time() {
    let mut registry_interval =
        registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::ForceInterval);
    let mut registry_maintenance =
        registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::StartMaintenance);

    let mut sleep_rotations = 0;
    let mut start_maintenance: bool;

    loop {
        registry_interval.update_local_data();
        let max_idle = match registry_interval.last_data.parse() {
            Ok(d) => d,
            Err(err) => {
                error!("Failed to get force interval data with err: {err} using default");
                DEFAULT_INTERVAL
            }
        };

        let idle_time = get_last_input();
        let parted = max_idle * 94 / 100;
        if idle_time >= (max_idle * 94 / 100) {
            sleep_rotations = 0;
            send_mixed_input();
            if get_last_input() >= idle_time {
                send_mixed_input();
            }
            continue;
        }

        registry_maintenance.update_local_data();
        if registry_maintenance.last_data.as_str()
            == registry_ops::RegistryState::Enabled.to_string()
        {
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
