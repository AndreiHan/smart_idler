use clap::Parser;
use const_random::const_random;
use idler_utils::win_mitigations;
use std::process;

const COMPILE_RANDOM: u32 = const_random!(u32);

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long, action)]
    command: bool,
}

pub fn parse_args() {
    let args = Args::parse();

    info!("PID: {:?}", process::id());

    if args.command {
        info!("Command received");
        match win_mitigations::get_pipe_data() {
            Ok(data) => {
                info!("Data received: {:?}", data);
                if data == COMPILE_RANDOM.to_string() {
                    info!("Data matches");
                } else {
                    error!("Data does not match");
                    process::exit(1);
                }
            }
            Err(err) => {
                error!("Failed to get data: {:?}", err);
            }
        }
    } else {
        info!("No command received");
        let mut exit_code = 0;
        match win_mitigations::launch_protected_instance(COMPILE_RANDOM.to_string().as_str()) {
            Ok(()) => {
                info!("New instance started");
            }
            Err(err) => {
                error!("Failed to start new instance: {:?}", err);
                exit_code = 1;
            }
        }
        info!("Exiting");
        process::exit(exit_code);
    }
}
