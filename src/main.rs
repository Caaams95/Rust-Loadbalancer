mod request;
mod http_health_checks;

mod active_health_check_test ;



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


/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdOptions {
    /// Name of the person to greet
    #[arg(short, long, long_help = "Upstream server(s) to proxy to")]
    upstream: Vec<String>,

    #[arg(short, long, long_help = "Bind to this address", default_value = "0.0.0.0:8080")]
    bind: String,

    /// Interval between each health check in seconds Default is 5 second
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// The path to use for active health checks
    /// Default is /
    #[arg(short, long, default_value = "/")]
    path: String,
}


#[derive(Debug)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 2)
    #[allow(dead_code)]
    active_health_check_interval: u64,
    /// Where we should send requests when doing active health checks (Milestone 2)
    #[allow(dead_code)]
    active_health_check_path: String,

    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,

    /// List of all the active upstream servers (Milestone 2)
    /// This list will be used to store the active upstream servers
    active_upstream_addresses: Vec<String>,

}

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

async fn handle_connection(mut client_stream: TcpStream, shared_state: Arc<Mutex<ProxyState>>) {
    let state = shared_state.lock().await;
    let upstream_address_list = state.active_upstream_addresses.clone();

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

    // get the client's IP address - two var to prevent the borrow error in &str
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
