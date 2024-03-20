use anyhow::{anyhow, Result};
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use sysinfo::{Pid, RefreshKind, System};

use crate::process_ops::{self, AppProcess};

#[inline]
fn get_file_name(app: &process_ops::AppProcess) -> String {
    app.to_string().replace(".exe", ".lock")
}

#[derive(PartialEq, Eq)]
pub enum CheckStatus {
    Failed,
    Passed,
}

fn check_existing_file(file_path: &PathBuf, app: &AppProcess) -> Result<CheckStatus> {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(err) => {
            error!("Failed opening file {:?} with err: {}", file_path, err);
            return Err(err.into());
        }
    };
    let mut line = String::new();
    match io::BufReader::new(file).read_line(&mut line) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to parse {:?}, with err: {}", file_path, err);
            return Err(err.into());
        }
    }

    let s = System::new_with_specifics(RefreshKind::with_processes(
        Default::default(),
        Default::default(),
    ));
    let line = match line.parse::<Pid>() {
        Ok(l) => l,
        Err(err) => {
            error!(
                "Failed to parse line from: {:?}, with err: {}",
                file_path, err
            );
            return Err(err.into());
        }
    };
    match s.process(line) {
        None => {
            debug!("Found dead PID: {} in lock file, ignoring it", line);
            let _ = create_lock_file(file_path);
            Ok(CheckStatus::Passed)
        }
        Some(process) => {
            let alive_proc = process.name();
            debug!(
                "Lock PID: {} is still alive with name: {}",
                alive_proc, line
            );
            if alive_proc == app.to_string() {
                error!("Same name as process: {}", alive_proc);
                return Ok(CheckStatus::Failed);
            }
            debug!("Name different from process: {} ignoring", app.to_string());
            match create_lock_file(file_path) {
                Err(e) => {
                    error!("Failed to create lock file with err: {}", e);
                    Err(anyhow!(e))
                }
                Ok(_) => Ok(CheckStatus::Passed),
            }
        }
    }
}

fn create_lock_file(file_path: &PathBuf) -> Result<()> {
    let mut file = File::create(file_path)?;

    let pid = format!("{}", std::process::id());
    match file.write_all(pid.as_ref()) {
        Ok(_) => {
            debug!("Created lock file: {:?} with content: {}", file_path, &pid);
            Ok(())
        }
        Err(e) => {
            error!("Failed to create file: {:?} with err: {}", file_path, e);
            Err(anyhow!(e))
        }
    }
}

#[derive(Copy, Clone)]
pub struct SingleInstance {
    app: process_ops::AppProcess,
}

impl SingleInstance {
    pub fn new(new_app: process_ops::AppProcess) -> SingleInstance {
        SingleInstance { app: new_app }
    }

    pub fn check(&self) -> Result<CheckStatus> {
        let current_lock = SingleInstance::get_path(&self.app).ok_or(anyhow!("No lock file"))?;
        if current_lock.is_file() {
            check_existing_file(&current_lock, &self.app)
        } else {
            match create_lock_file(&current_lock) {
                Ok(_) => Ok(CheckStatus::Passed),
                Err(e) => {
                    error!("Failed to create lock file with err: {}", e);
                    Err(e)
                }
            }
        }
    }

    pub fn exit(&self) -> Result<()> {
        let current_lock = SingleInstance::get_path(&self.app).ok_or(anyhow!("No lock file"))?;
        if !current_lock.is_file() {
            info!("Could not find lock");
            return Ok(());
        }

        match fs::remove_file(current_lock) {
            Ok(_) => {
                info!("Removed Lock file");
                Ok(())
            }
            Err(e) => {
                error!("Failed to remove lock file with err: {}", e);
                Err(anyhow!(e))
            }
        }
    }

    fn get_path(app: &process_ops::AppProcess) -> Option<PathBuf> {
        let temp_dir = match env::var("TEMP") {
            Ok(dir) => {
                debug!("TEMP dir: {:?}", dir);
                dir
            }
            Err(e) => {
                error!("Failed to get TEMP dir: {}", e);
                return None;
            }
        };
        Some(Path::new(&temp_dir).join(get_file_name(app)))
    }
}
