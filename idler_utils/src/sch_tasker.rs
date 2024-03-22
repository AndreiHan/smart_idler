use std::env;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};

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
    if sys32_path.is_file() {
        debug!("Found file schtasks.exe: {:?}", sys32_path.to_str());
        Some(sys32_path)
    } else {
        error!("schtasks path: {:?} doesn't exist", sys32_path.as_path());
        None
    }
}

pub fn delete_rule(schtasks: Option<&PathBuf>) -> Result<()> {
    let schtasks_path: PathBuf = match schtasks {
        None => match get_schtasks_path() {
            Some(p) => p,
            None => {
                if let Some(p) = get_schtasks_path() {
                    p
                } else {
                    error!("Failed to get schtasks path");
                    return Err(anyhow!("Path issue"));
                }
            }
        },
        Some(path) => path.clone(),
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
            return Err(anyhow!("Failed to run command"));
        }
    };
    if let Some(value) = output.status.code() {
        match value {
            0 => {
                info!("Deleted Task with name: {} | {:?}", TASK_NAME, output);
                Ok(())
            }
            err => {
                info!(
                    "Failed Deleting task with name: {} | {:?}",
                    TASK_NAME, output
                );
                Err(anyhow!("Error: {}", err))
            }
        }
    } else {
        error!("Failed parsing output");
        Err(anyhow!("Formatting issue"))
    }
}

pub fn enable_rule() -> Result<()> {
    let schtasks_path = match get_schtasks_path() {
        None => {
            error!("Failed to get schtasks path");
            return Err(anyhow!("Path issue"));
        }
        Some(path) => {
            debug!("schtasks path: {:?}", path.to_str());
            path
        }
    };
    let mut scheduler = Command::new(schtasks_path);

    let exe_path = match current_exe() {
        Ok(t) => {
            debug!("Exe path: {:?}", t.to_str());
            t
        }
        Err(e) => {
            error!("Failed to get exe path: {:?}", e.to_string());
            return Err(e.into());
        }
    };

    let Some(exe_str_path) = exe_path.to_str() else {
        error!("Failed to convert {:?} to str", exe_path);
        return Err(anyhow!("conversion issue"));
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
        "/RL",
        "HIGHEST",
    ];

    let output = match scheduler.args(sch_args).output() {
        Ok(t) => {
            debug!("Output: {:?}", t);
            t
        }
        Err(e) => {
            error!("Failed output: {:?}", e);
            return Err(e.into());
        }
    };
    match output.status.code() {
        None => {
            debug!("Closed with signal");
            Ok(())
        }
        Some(err_no) => match err_no {
            0 => {
                info!("Set task with name: {}", TASK_NAME);
                Ok(())
            }
            err => {
                error!("Failed to set task with err: {} | {:?}", err, output);
                Err(anyhow!("Failed with err: {}", err))
            }
        },
    }
}
