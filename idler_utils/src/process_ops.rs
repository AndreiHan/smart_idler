use std::fmt;
use std::process::Command;
use std::thread;

use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, Signal, System};

#[derive(Copy, Clone, Debug)]
pub enum AppProcess {
    SysTray,
    Controller,
}

impl fmt::Display for AppProcess {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppProcess::Controller => write!(f, "controller.exe"),
            AppProcess::SysTray => write!(f, "smart_tray.exe"),
        }
    }
}

fn get_system_exe() -> System {
    System::new_with_specifics(
        RefreshKind::new()
            .with_processes(ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always)),
    )
}

pub fn restart_app(app: &AppProcess) {
    let current_app = *app;
    std::thread::spawn(move || {
        info!("Restarting app: {:?}", &current_app);
        let app_pid = get_pid(&current_app);
        if app_pid.is_none() {
            open_app(&current_app, false)
        } else {
            kill_app(&current_app, true);
            open_app(&current_app, false);
        }
    });
}

pub fn kill_app(app: &AppProcess, wait_to_finish: bool) {
    let current_app = *app;
    let kill_handle = std::thread::spawn(move || {
        let s = get_system_exe();
        if let Some(process) = s.process(Pid::from(get_pid(&current_app).unwrap() as usize)) {
            if process.kill_with(Signal::Kill).is_none() {
                error!("This signal isn't supported on this platform");
            }
            info!("Killed {:?}", &current_app);
        }
    });
    if !wait_to_finish {
        return;
    }

    match kill_handle.join() {
        Ok(_) => info!("Closed kill thread for {:?}", app),
        Err(err) => error!("Failed to close thread with err: {:?}", err),
    }
}

pub fn open_app(app: &AppProcess, wait_to_finish: bool) {
    let current_app = *app;
    let open_handle = thread::spawn(move || {
        info!("Opened app: {:?}", &current_app);
        Command::new(current_app.to_string()).output().unwrap();
    });

    if !wait_to_finish {
        return;
    }

    match open_handle.join() {
        Ok(_) => info!("Closed kill thread for {:?}", app),
        Err(err) => error!("Failed to close thread with err: {:?}", err),
    }
}

fn get_pid(app: &AppProcess) -> Option<u32> {
    let process_name = app.to_string();
    let sys = get_system_exe();
    if let Some(process) = sys.processes_by_exact_name(&process_name).next() {
        if *process.name() != process_name {
            return None;
        }
        match process.pid().to_string().parse::<u32>() {
            Ok(value) => {
                info!("Pid for {:?} is {}", app, value);
                return Some(value);
            }
            Err(err) => {
                error!("Failed to get pid with err: {err}");
                return None;
            }
        }
    }
    None
}
