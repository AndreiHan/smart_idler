use std::{process::Command, thread};

const MNTS_EXE: &str = "MSchedExe.exe";

#[derive(Clone, Debug)]
pub enum Commands {
    Start,
    Stop,
}

pub struct Maintenance {}

impl Maintenance {
    pub fn change_state(wanted_state: &Commands) {
        info!("Running maintenance command: {:?}", wanted_state);
        run_command(wanted_state)
    }
}

fn get_args(state: &Commands) -> Vec<String> {
    match state {
        Commands::Start => {
            vec!["Start".to_string()]
        }
        Commands::Stop => {
            vec!["Stop".to_string()]
        }
    }
}

fn run_command(state: &Commands) {
    let current_state = state.clone();
    thread::spawn(move || {
        let args = get_args(&current_state);
        debug!("Current args: {:?}", args);
        let _ = Command::new(MNTS_EXE).args(args).spawn();
    });
}
