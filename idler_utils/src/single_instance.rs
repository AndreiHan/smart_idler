use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{env, fs, io};

use sysinfo::{Pid, RefreshKind, System};

use crate::process_ops;

fn get_file_name(app: &process_ops::AppProcess) -> String {
    app.to_string().replace(".exe", ".lock")
}

fn check_existing_file(file_path: &PathBuf) {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(err) => {
            error!("Failed opening file {:?} with err: {}", file_path, err);
            return;
        }
    };
    let mut line = String::new();
    match io::BufReader::new(file).read_line(&mut line) {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to parse {:?}, with err: {}", file_path, err);
            return;
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
            return;
        }
    };
    match s.process(line) {
        None => {
            info!("Found invalid PID in lock file, ignoring it");
            let _ = create_lock_file(file_path);
        }
        Some(process) => {
            error!(
                "Lock PID is still alive with name: {}, exiting",
                process.name()
            );
            exit(1);
        }
    }
}

fn create_lock_file(file_path: &PathBuf) -> Result<(), ()> {
    let mut file = File::create(file_path).unwrap();

    let pid = format!("{}", std::process::id());
    match file.write_all(pid.as_ref()) {
        Ok(_) => {
            info!("Created lock file: {:?} with content: {}", file_path, &pid);
            Ok(())
        }
        Err(e) => {
            error!("Failed to create file: {:?} with err: {}", file_path, e);
            Err(())
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

    pub fn check(&self) {
        let current_lock = SingleInstance::get_path(&self.app).unwrap();
        if current_lock.is_file() {
            check_existing_file(&current_lock);
        } else {
            let _ = create_lock_file(&current_lock);
        }
    }

    pub fn exit(&self) {
        let current_lock = SingleInstance::get_path(&self.app).unwrap();
        if !current_lock.is_file() {
            info!("Could not find lock");
            return;
        }

        match fs::remove_file(current_lock) {
            Ok(_) => {
                info!("Removed Lock file")
            }
            Err(e) => {
                error!("Failed to remove lock file with err: {}", e)
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
