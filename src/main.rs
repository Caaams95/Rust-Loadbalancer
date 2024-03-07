mod request;

// use std::env::Args;
use clap::{arg, Parser};
use log::{error}; // Import the `error` and `info` macros from the `log` crate
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use rand::seq::SliceRandom;
use crate::request::{request_controller};


/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdOptions {
    /// Name of the person to greet
    #[arg(short, long, long_help="Upstream server(s) to proxy to")]
    upstream: Vec<String>,

    #[arg(short, long, long_help="Bind to this address", default_value="0.0.0.0:8080")]
    bind: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}


#[derive(Debug)]
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

fn connect_to_upstream_server(mut upstream_address_list: Vec<String>) -> Result<TcpStream, std::io::Error> {
    
    let mut rng = rand::thread_rng();
    let upstream_address = upstream_address_list.choose(&mut rng).unwrap();
    
    println!("upstream_address: {:?}", upstream_address);
    
    match TcpStream::connect(upstream_address) {
        Ok(stream) =>Ok(stream),
        Err(e) => {
            // check if the upstream_address_list is empty
            if upstream_address_list.is_empty() {
                Err(e)
            }else { 
                // remove the line  upstream_address in upstream_address_list
                let index = upstream_address_list.iter().position(|x| x == upstream_address).unwrap();
                let _ = upstream_address_list.remove(index);
                
                // connect to the next upstream server
                connect_to_upstream_server(upstream_address_list)
            }
            
        }
    }
}

fn handle_connection(mut client_stream: TcpStream, state: &ProxyState) {    
    
    let mut upstream_address_list = state.upstream_addresses.clone();
    
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


    // Connect to the selected upstream server
    // let mut upstream_stream = match TcpStream::connect(upstream_address) {
    //     Ok(stream) => stream,
    //     Err(_) => {
    //         // If unable to connect to the upstream server, inform the client with a 502 Bad Gateway error
    //         let response = "HTTP/1.1 502 Bad Gateway\r\n\r\n";
    //         client_stream.write(response.as_bytes()).unwrap();
    //         return;
    //     }
    // };
    
    
    

    // Begin looping to read requests from the client
    loop {

        // Read the request from the client and forward it to the upstream server using the request_controller function
        match request_controller(&mut client_stream, client_ip, &mut upstream_stream){
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

fn main() {
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
        active_health_check_interval: 0, // Initialize with appropriate values
        active_health_check_path: String::new(), // Initialize with appropriate values
        rate_limit_window_size: 0, // Initialize with appropriate values
        max_requests_per_window: 0, // Initialize with appropriate values
        upstream_addresses: args.upstream, // Example addresses, replace with your logic
    };
    
    println!("state: {:?}", state);
    

    for stream in listener.incoming() {
        println!("New connection: {:?}", stream);
        if let Ok(stream) = stream {
            // Handle the connection!
            handle_connection(stream, &state);
        }
    }
     

}
