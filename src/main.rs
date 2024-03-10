//! # Asynchronous Proxy Server in Rust
//!
//! This module implements a simple asynchronous proxy server in Rust. The server listens for incoming TCP connections,
//! proxies the requests to one of the specified upstream servers, and forwards the responses back to the client.
//!
//! ## Modules
//!
//! - `request`: Module for handling client requests.
//! - `http_health_checks`: Module for performing HTTP-based health checks on upstream servers.
//! - `test_active_health_check`: Module for testing active health check functionality.
//! - `test_request`: Module for testing request handling functionality.
//!
//! ## Dependencies
//!
//! - `clap`: Command line argument parsing.
//! - `log`: Logging macros.
//! - `rand`: Random number generation for load balancing among upstream servers.
//! - `tokio`: Asynchronous runtime.
//!
//! ## Usage
//!
//! To run the proxy server, use the following command:
//!
//! ```sh
//! cargo run -- --upstream <upstream-server-1> --upstream <upstream-server-2> ... --bind <bind-address> --interval <health-check-interval> --path <health-check-path>
//! ```
//!
//! ## Options
//!
//! - `--upstream`: Upstream server(s) to proxy to.
//! - `--bind`: The address to bind the proxy server to.
//! - `--interval`: Interval between each health check in seconds. Default is 5 seconds.
//! - `--path`: The path to use for active health checks. Default value is "/".
//!
//! ## Structures
//!
//! - `CmdOptions`: Represents the command-line options for configuring the proxy server.
//! - `ProxyState`: Represents the state of the proxy server, including active health check settings and upstream server addresses.
//!
//! ## Functions
//!
//! - `connect_to_upstream_server`: Attempts to connect to an upstream server.
//! - `handle_connection`: Asynchronously handles incoming client connections, proxies requests, and forwards responses.
//!
//! ## Main Function
//!
//! The `main` function initializes the proxy server by parsing command line arguments, creating a listener for incoming connections,
//! and starting asynchronous tasks for active health checks and connection handling.

mod request;
mod http_health_checks;

mod test_active_health_check;
mod test_request;


// use std::env::Args;
use clap::{arg, Parser};
use log::{error};
// Import the `error` and `info` macros from the `log` crate
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

use rand::seq::SliceRandom;
use crate::request::{request_controller};
use std::sync::{Arc};
use tokio::sync::{Mutex};
use tokio::time::{sleep, Duration};
use crate::http_health_checks::basic_http_health_check;



/// Command line options for the proxy server.
///
/// This struct represents the command-line options that can be used to configure the proxy server.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdOptions {
    /// Upstream server(s) to proxy to.
    ///
    /// This option specifies the addresses of the upstream servers that the proxy server will forward client requests to.
    #[arg(short, long, long_help = "Upstream server(s) to proxy to")]
    upstream: Vec<String>,

    /// The address to bind the proxy server to.
    ///
    /// This option specifies the network address to which the proxy server will bind and listen for incoming connections.
    #[arg(short, long, long_help = "Bind to this address", default_value = "0.0.0.0:8080")]
    bind: String,

    /// Interval between each health check in seconds. Default is 5 seconds.
    ///
    /// This option specifies the time interval (in seconds) between each health check performed by the proxy server
    /// to determine the availability of upstream servers
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// The path to use for active health checks.
    ///
    /// This option specifies the endpoint path used by the proxy server for active health checks on the upstream servers.
    /// The proxy server sends health check requests to this path to determine the availability of the upstream servers.
    /// Default value is "/".
    #[arg(short, long, default_value = "/")]
    path: String,
}

/// Represents the state of the proxy server.
#[derive(Debug)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive.
    ///
    /// This value determines the interval (in seconds) at which the proxy server performs active health checks
    /// on the upstream servers to determine their availability.
    #[allow(dead_code)]
    active_health_check_interval: u64,

    /// The path used for active health checks.
    ///
    /// This is the endpoint path to which the proxy server sends health check requests to the upstream servers
    /// to determine their availability.
    #[allow(dead_code)]
    active_health_check_path: String,

    /// Addresses of servers that the proxy server is proxying to.
    ///
    /// This vector contains the addresses of all the upstream servers that the proxy server forwards client requests to.
    upstream_addresses: Vec<String>,

    /// List of all the active upstream servers.
    ///
    /// This list is used to store the addresses of the upstream servers that are currently deemed as active,
    /// based on the results of the active health checks performed by the proxy server.
    active_upstream_addresses: Vec<String>,

}


/// Attempts to connect to an upstream server randomly selected from the provided list.
///
/// This function takes a list of upstream server addresses and randomly selects one to establish a TCP connection.
/// If the connection attempt fails, it recursively retries with the remaining addresses until a successful connection is made
/// or the list is exhausted. This helps in load balancing and handling failures gracefully.
///
/// # Arguments
///
/// - `upstream_address_list`: A mutable vector containing the addresses of upstream servers.
///
/// # Returns
///
/// - `Result<TcpStream, std::io::Error>`: A `Result` representing either a successfully established TCP stream or an error if all connection attempts fail.
///
/// # Example
///
/// ```rust
/// use std::net::TcpStream;
///
/// let upstream_addresses = vec!["127.0.0.1:8081", "127.0.0.1:8082", "127.0.0.1:8083"];
/// let result = connect_to_upstream_server(upstream_addresses);
/// match result {
///     Ok(stream) => {
///         // Successfully connected to an upstream server
///         // Use the 'stream' to communicate with the server
///     }
///     Err(error) => {
///         eprintln!("Failed to connect to upstream server: {}", error);
///     }
/// }
/// ```
fn connect_to_upstream_server(mut upstream_address_list: Vec<String>) -> Result<TcpStream, std::io::Error> {
    let mut rng = rand::thread_rng();
    let upstream_address = upstream_address_list.choose(&mut rng).unwrap();

    println!("upstream_address: {:?}", upstream_address);

    match TcpStream::connect(upstream_address) {
        Ok(stream) => Ok(stream),
        Err(e) => {
            // check if the upstream_address_list is empty
            if upstream_address_list.is_empty() {
                Err(e)
            } else {
                // remove the line  upstream_address in upstream_address_list
                let index = upstream_address_list.iter().position(|x| x == upstream_address).unwrap();
                let _ = upstream_address_list.remove(index);

                // connect to the next upstream server
                connect_to_upstream_server(upstream_address_list)
            }
        }
    }
}

/// Handles an incoming client connection asynchronously.
///
/// This async function is responsible for handling an incoming TCP client connection. It begins by attempting to establish a connection
/// to one of the active upstream servers randomly selected based on health and load balancing considerations. If the connection to the
/// upstream server is successful, it enters into a loop where it reads client requests, forwards them to the upstream server using the
/// `request_controller` function, and sends back the received responses to the client.
///
/// If the connection to the upstream server fails or encounters errors during request handling, appropriate HTTP error responses are sent
/// to the client to inform them of the issues.
///
/// # Arguments
///
/// - `client_stream`: A mutable reference to the TCP stream representing the client connection.
/// - `shared_state`: An `Arc<Mutex<ProxyState>>` representing the shared state of the proxy server, including active upstream server addresses.
///

async fn handle_connection(mut client_stream: TcpStream, shared_state: Arc<Mutex<ProxyState>>) {
    // Lock the shared state to access active upstream server addresses
    let state = shared_state.lock().await;
    let upstream_address_list = state.active_upstream_addresses.clone();
    
    // Print active upstream server addresses for debugging purposes
    println!("active_upstream_addresses: {:?}", state.active_upstream_addresses);

    // it checked and do some health check
    let mut upstream_stream = match connect_to_upstream_server(upstream_address_list.clone()) {
        Ok(stream) => stream,
        Err(_) => {

            // If unable to connect to the upstream server, inform the client with a 502 Bad Gateway error
            let response = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
            client_stream.write(response.as_bytes()).unwrap();
            return;
        }
    };

    // Get the client's IP address to include in request processing - two var to prevent the borrow error in &str
    let binding = client_stream.peer_addr().unwrap().to_string();
    let client_ip = binding.as_str();

    // Begin looping to read requests from the client
    loop {

        // Read the request from the client and forward it to the upstream server using the request_controller function
        match request_controller(&mut client_stream, client_ip, &mut upstream_stream) {
            Ok(_) => (),
            Err(request::Error::ClientClosedConnection) => {
                eprintln!("Client closed the connection");
                return;
            }
            Err(request::Error::ConnectionError) => {
                eprintln!("Error reading request from client");
                return;
            }
            Err(_) => {
                // If there is an error in reading the request, inform the client with a 400 Bad Request error and return
                let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
                client_stream.write(response.as_bytes()).unwrap();
                return;
            }
        };

        // Try to read the response from the upstream server into a string buffer (upstream_response) and handle any errors
        // If there is an error in receiving the response, inform the client with a 502 Bad Gateway error and return
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
        // Try to write the response to the client and handle any errors
        match client_stream.write_all(upstream_response.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to write to stream: {}", e);
                return;
            }
        }

        // Try to flush the stream
        match client_stream.flush() {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to flush stream: {}", e);
                return;
            }
        }
    }
}




/// Main entry point for the proxy server.
///
/// This function parses command line arguments, initializes the proxy state, and starts two asynchronous tasks:
/// one for active health checks and another for handling incoming connections.
#[tokio::main]
async fn main() {
    // Parse the command line arguments passed to this program
    let args = CmdOptions::parse();

    if args.upstream.len() < 1 {
        error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Creates a server socket so that it can begin listening for connections:
    let listener = match TcpListener::bind(&args.bind) {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {:?}: {}", args.bind, err);
            std::process::exit(1);
        }
    };

    println!("Listening for requests on {:?}", listener);

    // Initialize the proxy state
    let state = ProxyState {
        active_health_check_interval: args.interval, // Initialize with appropriate values
        active_health_check_path: args.path, // Initialize with appropriate values
        upstream_addresses: args.upstream, // Example addresses, replace with your logic
        active_upstream_addresses: Vec::new(), // Initialize with appropriate values
    };

    println!("{:?}", state);

    let shared_state = Arc::new(Mutex::new(state));

    let thread_state_health_check = Arc::clone(&shared_state);
    let thread_state_connection = Arc::clone(&shared_state);

    // Start a new thread to perform active health checks and update the active upstream servers
    tokio::spawn(async move {
        loop {
            // Perform active health checks and update the active upstream servers
            let mut state = thread_state_health_check.lock().await;
            let interval = state.active_health_check_interval.clone();

            // clear the active upstream servers
            state.active_upstream_addresses.clear();

            println!("Performing active health checks and updating the active upstream servers");
            for ip in state.upstream_addresses.clone() {
                // create match condition to check if the server is up or down and update the active upstream servers
                match basic_http_health_check(ip.clone(), state.active_health_check_path.clone()) {
                    Ok(_) => {
                        state.active_upstream_addresses.push(ip.clone());
                    }
                    Err(_) => {
                    }
                }
            }

            println!("{:?}", state.active_upstream_addresses);

            // drop(state);


            // Sleep for the specified interval
            sleep(Duration::from_secs(interval)).await;
        }
    });


    tokio::spawn(async move {
        loop {
            // Handle incoming connections
            let shared_state = thread_state_connection.clone();

            for stream in listener.incoming() {
                println!("New connection: {:?}", stream);
                if let Ok(stream) = stream {
                    // Handle the connection!
                    handle_connection(stream, shared_state.clone()).await;
                }
            }
        }
    });

    loop {}
}
