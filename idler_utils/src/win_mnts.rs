use std::{process::Command, thread};

const MNTS_EXE: &str = "MSchedExe.exe";

#[derive(Clone, Debug)]
pub enum Commands {
    Start,
    Stop,
}

impl ToString for Commands {
    fn to_string(&self) -> String {
        match self {
            Commands::Start => String::from("Start"),
            Commands::Stop => String::from("Stop"),
        }
    }
}

pub struct Maintenance {}

impl Maintenance {
    pub fn change_state(wanted_state: &Commands) {
        let current_args = vec![wanted_state.to_string()];
        info!(
            "Running maintenance command: {:?} with args: {:?}",
            wanted_state, current_args
        );
        thread::spawn(move || {
            let _ = Command::new(MNTS_EXE).args(current_args).spawn();
        });
    }
}
