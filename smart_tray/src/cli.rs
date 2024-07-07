use clap::Parser;
use idler_utils::win_mitigations;
use std::process;
use const_random::const_random;

const COMPILE_RANDOM: u32 = const_random!(u32);

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long, default_value_t = String::from("none"))]
    command: String,
}

pub fn parse_args() {
    let args = Args::parse();

    info!("PID: {:?}", process::id());

    let command = args.command.as_str();
    if command == "none" {
        let com = format!("-c \"{COMPILE_RANDOM}\"");
        info!("Command: {:?}", com);
        match win_mitigations::launch_new_instance(Some(com.as_str())) {
            Ok(()) => {
                info!("New instance started");
            }
            Err(err) => {
                error!("Failed to start new instance: {:?}", err);
            }
        }
        info!("Exiting");
        process::exit(0);
    }

    if command == COMPILE_RANDOM.to_string().as_str() {
        info!("Correct compile random: {:?}", command);
        return;
    }
    error!("Invalid command: {:?}", args.command);
    process::exit(1);
}
