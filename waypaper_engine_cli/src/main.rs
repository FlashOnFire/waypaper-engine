use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use linux_ipc::IpcChannel;
use tracing::{debug, error, info};
use waypaper_engine_shared::ipc::IPCRequest;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    commands: Commands,
    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Output in JSON format",
        global = true
    )]
    json_output: bool,
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
enum Commands {
    /// Change the wallpaper on the given screen
    Set {
        /// The screen identifier (e.g. "DP-1", "HDMI-0")
        screen: String,
        /// The wallpaper ID to set
        id: u64,
    },
    /// List all available outputs
    Outputs,
    /// Kill the daemon
    #[clap(name = "kill-daemon", aliases = &["killdaemon", "kill"])]
    KillDaemon,
}

fn main() {
    let args = Args::parse();

    if !args.json_output {
        tracing_subscriber::fmt()
            .without_time()
            .with_target(false)
            .with_max_level(args.verbosity)
            .init()
    }

    let mut channel = match IpcChannel::connect("/tmp/waypaper-engine.sock") {
        Ok(channel) => channel,
        Err(err) => {
            if args.json_output {
                println!(
                    r#"{{"success": "false", "error_kind": "no_daemon", "message": "Failed to connect to the daemon: {}"}}"#,
                    err
                );
            } else {
                error!("Failed to connect to the daemon, is it running?");
                error!("{err}");
            }
            return;
        }
    };

    match &args.commands {
        Commands::Outputs => {
            info!("Listing outputs...");
        }
        Commands::Set { screen, id } => {
            info!("Setting screen {screen} with id {id}");
            if !args.json_output {
                debug!("Sending request to set wallpaper...");
            }
            match channel.send::<_, ()>(IPCRequest::SetWallpaper {
                screen: screen.clone(),
                id: *id,
            }) {
                Ok(_) => {
                    if args.json_output {
                        print_json_success();
                    } else {
                        info!("Wallpaper set successfully.");
                    }
                }
                Err(err) => {
                    if args.json_output {
                        print_json_error("set_wallpaper_failed", &err.to_string());
                    } else {
                        error!("Failed to set wallpaper");
                        error!("Error: {}", err);
                    }
                }
            }
        }
        Commands::KillDaemon => {
            if !args.json_output {
                debug!("Killing the daemon...");
            }

            match channel.send::<_, ()>(IPCRequest::KillDaemon) {
                Ok(_) => {
                    if args.json_output {
                        println!(r#"{{"success": true}}"#);
                    } else {
                        info!("Stop request sent to the daemon successfully.");
                    }
                }
                Err(err) => {
                    if args.json_output {
                        print_json_error("kill_daemon_failed", &err.to_string());
                    } else {
                        error!("Failed to send stop request to the daemon");
                        error!("Error: {}", err);
                    }
                }
            }
        }
    }
}

fn print_json_success() {
    println!(r#"{{"success": true}}"#);
}

fn print_json_error(error_kind: &str, message: &str) {
    println!(r#"{{"success": false, "error": "{}", "message": "{}"}}"#, error_kind, message);
}