use std::thread;
use std::{fmt::Debug, process::Command};

use anyhow::{anyhow, Result};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, Signal, System};

#[derive(Copy, Clone, Debug)]
pub enum AppProcess {
    SysTray,
}

impl ToString for AppProcess {
    fn to_string(&self) -> String {
        match self {
            AppProcess::SysTray => String::from("smart_tray.exe"),
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
        info!("Restarting app: {:?}", current_app);
        match get_pid(current_app) {
            Some(_) => {
                let _ = kill_app(&current_app, true);
                let _ = open_app(&current_app, false);
            }
            None => {
                let _ = open_app(&current_app, false);
            }
        }
    });
}

pub fn kill_app(app: &AppProcess, wait_to_finish: bool) -> Result<()> {
    let current_app = *app;
    let kill_handle = std::thread::spawn(move || {
        let s = get_system_exe();
        let pid = get_pid(current_app).unwrap_or(0);
        if pid == 0 {
            error!("No pid found for {:?}", &current_app);
            return Err(anyhow!("pid lookup failed"));
        }
        if let Some(process) = s.process(Pid::from(pid as usize)) {
            if process.kill_with(Signal::Kill).is_none() {
                error!("This signal isn't supported on this platform");
                return Err(anyhow!("Unsupported platform for kill signal"));
            }
            info!("Killed {:?}", &current_app);
        }
        Ok(())
    });
    if !wait_to_finish {
        return Ok(());
    }

    match kill_handle.join() {
        Ok(_) => {
            info!("Closed kill thread for {:?}", app);
            Ok(())
        }
        Err(err) => {
            error!("Failed to close thread with err: {:?}", err);
            Err(anyhow!("Failed join, err: {:?}", err))
        }
    }
}

pub fn open_app(app: &AppProcess, wait_to_finish: bool) -> Result<()> {
    let current_app = *app;
    let open_handle = thread::spawn(move || {
        info!("Opened app: {:?}", &current_app);
        let _ = Command::new(current_app.to_string()).output();
    });

    if !wait_to_finish {
        return Ok(());
    }

    match open_handle.join() {
        Ok(()) => {
            info!("Closed kill thread for {:?}", app);
            Ok(())
        }
        Err(err) => {
            error!("Failed to close thread with err: {:?}", err);
            Err(anyhow!("{:?}", err))
        }
    }
}

fn get_pid(app: AppProcess) -> Option<u32> {
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
