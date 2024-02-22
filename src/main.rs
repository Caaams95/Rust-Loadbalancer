use clap::Parser;
use log::{error}; // Import the `error` and `info` macros from the `log` crate
use std::net::{TcpListener, TcpStream};

use std::io::{Read, Write};
use rand::seq::SliceRandom;


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



fn handle_connection(mut client_stream: TcpStream, state: &ProxyState) {
    // Select a random upstream server
    let mut rng = rand::thread_rng();
    let upstream_address = state.upstream_addresses.choose(&mut rng).unwrap();

    // Connect to the selected upstream server
    let mut upstream_stream = match TcpStream::connect(upstream_address) {
        Ok(stream) => stream,
        Err(_) => {
            // If unable to connect to the upstream server, inform the client with a 502 Bad Gateway error
            let response = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
            client_stream.write(response.as_bytes()).unwrap();
            return;
        }
    };

    // Begin looping to read requests from the client
    loop {
        let mut buffer = [0; 1024];
        let bytes_read = match client_stream.read(&mut buffer) {
            Ok(bytes) => bytes,
            Err(_) => {
                // Error handling in case the client sends a malformed request
                let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
                client_stream.write(response.as_bytes()).unwrap();
                return;
            }
        };

        // If no bytes are read, the client closed the connection
        if bytes_read == 0 {
            return;
        }

        // Add X-Forwarded-For header
        let mut request = String::from("X-Forwarded-For: ");
        request.push_str(&client_stream.peer_addr().unwrap().to_string());
        request.push_str("\r\n");
        request.push_str(&String::from_utf8_lossy(&buffer[..]));

        // Relay the request to the upstream server
        upstream_stream.write_all(request.as_bytes()).unwrap();

        // Try to read the response from the upstream server
        let mut upstream_response = String::new();
        match upstream_stream.read_to_string(&mut upstream_response) {
            Ok(_) => (),
            Err(_) => {
                // If there is an error in receiving the response, inform the client
                let response = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
                client_stream.write(response.as_bytes()).unwrap();
                return;
            }
        }

        // Forward the response to the client
        client_stream.write_all(upstream_response.as_bytes()).unwrap();
        client_stream.flush().unwrap();
    }
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

    // Initialize the proxy state
    let state = ProxyState {
        active_health_check_interval: 0, // Initialize with appropriate values
        active_health_check_path: String::new(), // Initialize with appropriate values
        rate_limit_window_size: 0, // Initialize with appropriate values
        max_requests_per_window: 0, // Initialize with appropriate values
        upstream_addresses: vec!["127.0.0.1:8080".to_string()], // Example addresses, replace with your logic
    };
    

    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // Handle the connection!
        handle_connection(stream, &state);
    }
     

}
