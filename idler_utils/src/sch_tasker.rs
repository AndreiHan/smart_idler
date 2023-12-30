use std::env;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::process::Command;

const TASK_NAME: &str = "SmartIdler";

fn get_schtasks_path() -> Option<PathBuf> {
    let win_dir = match env::var("WINDIR") {
        Ok(dir) => {
            debug!("system32 dir: {:?}", dir);
            dir
        }
        Err(e) => {
            error!("Failed to get system32: {}", e);
            return None;
        }
    };

    let sys32_path = Path::new(win_dir.as_str())
        .join("system32")
        .join("schtasks.exe");
    match sys32_path.is_file() {
        true => {
            debug!("Found file schtasks.exe: {:?}", sys32_path.to_str());
            Some(sys32_path)
        }
        false => {
            error!("schtasks path: {:?} doesn't exist", sys32_path.as_path());
            None
        }
    }
}

pub fn delete_rule(schtasks: Option<&PathBuf>) {
    let schtasks_path: PathBuf = match schtasks {
        None => match get_schtasks_path() {
            Some(p) => p,
            None => {
                error!("Failed to get schtasks path");
                return;
            }
        },
        Some(path) => path.to_path_buf(),
    };
    let mut scheduler = Command::new(schtasks_path);
    let sch_args = vec!["/delete", "/tn", TASK_NAME, "/f"];

    let output = match scheduler.args(sch_args).output() {
        Ok(out) => {
            debug!("Delete command output: {:?}", out);
            out
        }
        Err(err) => {
            error!("Failed sending delete command: {}", err);
            return;
        }
    };
    match output.status.code() {
        Some(value) => match value {
            0 => {
                info!("Deleted Task with name: {} | {:?}", TASK_NAME, output);
            }
            _ => {
                info!(
                    "Failed Deleting task with name: {} | {:?}",
                    TASK_NAME, output
                );
            }
        },
        None => {
            error!("Failed parsing output");
        }
    }
}

pub fn enable_rule() {
    let schtasks_path = match get_schtasks_path() {
        None => {
            error!("Failed to get schtasks path");
            return;
        }
        Some(path) => {
            debug!("schtasks path: {:?}", path.to_str());
            path
        }
    };
    delete_rule(Some(&schtasks_path));
    let mut scheduler = Command::new(schtasks_path);

    let exe_path = match current_exe() {
        Ok(t) => {
            debug!("Exe path: {:?}", t.to_str());
            t
        }
        Err(e) => {
            error!("Failed to get exe path: {:?}", e.to_string());
            return;
        }
    };

    let exe_str_path = match exe_path.to_str() {
        Some(value) => value,
        None => {
            error!("Failed to convert {:?} to str", exe_path);
            return;
        }
    };

    let sch_args = vec![
        "/create",
        "/tn",
        TASK_NAME,
        "/sc",
        "ONSTART",
        "/tr",
        exe_str_path,
        "/f",
    ];

    let output = match scheduler.args(sch_args).output() {
        Ok(t) => {
            debug!("Output: {:?}", t);
            t
        }
        Err(e) => {
            error!("Failed output: {:?}", e);
            return;
        }
    };
    match output.status.code() {
        None => debug!("Closed with signal"),
        Some(err_no) => match err_no {
            0 => {
                info!("Set task with name: {}", TASK_NAME)
            }
            err => {
                error!("Failed to set task with err: {} | {:?}", err, output)
            }
        },
    }
}
