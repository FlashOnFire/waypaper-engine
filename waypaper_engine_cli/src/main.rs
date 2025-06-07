use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use linux_ipc::IpcChannel;
use std::io;
use tracing::{debug, error, info};
use waypaper_engine_shared::ipc::{IPCError, IPCRequest, IPCResponse};

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
            print_daemon_connection_error(&err.to_string(), args.json_output);
            return;
        }
    };

    match &args.commands {
        Commands::Outputs => {
            if !args.json_output {
                debug!("Sending request to the daemon...");
            }
            handle_ipc_response(
                channel.send::<_, IPCResponse>(IPCRequest::ListOutputs),
                args.json_output,
            );
        }
        Commands::Set { screen, id } => {
            info!("Setting wallpaper with ID {} on screen {}", id, screen);
            if !args.json_output {
                debug!("Sending request to the daemon...");
            }
            handle_ipc_response(
                channel.send::<_, IPCResponse>(IPCRequest::SetWallpaper {
                    screen: screen.clone(),
                    id: *id,
                }),
                args.json_output,
            );
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
                    print_daemon_connection_error(&err.to_string(), args.json_output);
                }
            }
        }
    }
}

fn handle_ipc_response(response: Result<Option<IPCResponse>, io::Error>, json_output: bool) {
    match response {
        Ok(Some(response)) => print_ipc_response(&response, json_output),
        Ok(None) => print_daemon_no_response_error(json_output),
        Err(err) => {
            print_daemon_connection_error(&err.to_string(), json_output);
        }
    }
}

fn print_ipc_response(response: &IPCResponse, json_output: bool) {
    match response {
        IPCResponse::Success => {
            if json_output {
                print_json_success();
            } else {
                info!("Success");
            }
        }
        IPCResponse::Outputs(outputs) => {
            if json_output {
                println!(r#"{{"success": true, "outputs": {:?}}}"#, outputs);
            } else {
                info!("Outputs: {:?}", outputs);
            }
        }
        IPCResponse::Error(error) => {
            print_ipc_error(error, json_output);
        }
    }
}

fn print_ipc_error(error: &IPCError, json_output: bool) {
    let (error_kind, message) = match error {
        IPCError::ScreenNotFound => ("screen_not_found", "The specified screen was not found."),
        IPCError::WallpaperNotFound => (
            "wallpaper_not_found",
            "The specified wallpaper was not found.",
        ),
        IPCError::UnsupportedWallpaperType => (
            "unsupported_wallpaper_type",
            "The wallpaper type is unsupported.",
        ),
        IPCError::InternalError => (
            "internal_error",
            "An internal error occurred while processing the request.",
        ),
    };

    if json_output {
        print_json_error(error_kind, message);
    } else {
        error!("Error: {}", message);
    }
}

fn print_daemon_no_response_error(json_output: bool) {
    if json_output {
        println!(
            r#"{{"success": false, "error": "no_response", "message": "No response from daemon."}}"#
        );
    } else {
        error!("No response from daemon.");
    }
}

fn print_daemon_connection_error(error: &str, json_output: bool) {
    if json_output {
        println!(
            r#"{{"success": false, "error_kind": "no_daemon", "message": "{}"}}"#,
            error
        );
    } else {
        error!("Failed to connect to the daemon, is it running?");
        error!("{}", error);
    }
}

fn print_json_success() {
    println!(r#"{{"success": true}}"#);
}

fn print_json_error(error_kind: &str, message: &str) {
    println!(
        r#"{{"success": false, "error": "{}", "message": "{}"}}"#,
        error_kind, message
    );
}
