use clap::Parser;
use log::{error}; // Import the `error` and `info` macros from the `log` crate
use std::net::{TcpListener, TcpStream};


/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdOptions {
    /// Name of the person to greet
    #[arg(short, long)]
    upstream: String,


    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 2)
    #[allow(dead_code)]
    active_health_check_interval: u64,
    /// Where we should send requests when doing active health checks (Milestone 2)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// How big the rate limiting window should be, default is 1 minute (Milestone 3)
    #[allow(dead_code)]
    rate_limit_window_size: u64,
    /// Maximum number of requests an individual IP can make in a window (Milestone 3)
    #[allow(dead_code)]
    max_requests_per_window: u64,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
}


fn main() {
    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Creates a server socket so that it can begin listening for connections:
    let listener = match TcpListener::bind(&options.upstream) {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.upstream, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.upstream);

    /*
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // Handle the connection!
        handle_connection(stream, &state);
    }
     */

}
