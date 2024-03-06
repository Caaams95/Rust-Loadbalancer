mod request;

// use std::env::Args;
use clap::{arg, Parser};
use log::{error}; // Import the `error` and `info` macros from the `log` crate
use std::net::{TcpListener, TcpStream};

use std::io::{Read, Write};
use http::Request;
use rand::seq::SliceRandom;
use crate::request::Error;


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

    // get the client's IP address
    let client_ip = client_stream.peer_addr().unwrap().to_string();


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
            log::info!("Client closed the connection");
            return;
        }

        // read the request from the client
        // let mut request = String::from_utf8_lossy(&buffer[..bytes_read]);

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        let res = req.parse(&buffer).unwrap();
        if res.is_partial() {
            match req.path {
                Some(ref path) => {
                    // check router for path.
                    // /404 doesn't exist? we could stop parsing
                },
                None => {
                    // must read more and parse again
                }
            }
        }

        // build parsed request with method, uri and version
        let mut parsed_request = Request::builder()
            .method(req.method.unwrap())
            .uri(req.path.unwrap())
            .version(http::Version::HTTP_11);

        // add headers to parsed request
        for header in req.headers {
            parsed_request = parsed_request.header(header.name, header.value);
        }

        parsed_request = parsed_request.header("X-Forwarded-For", client_ip.clone());

        // build parsed request with body and unwrap it
        let parsed_request = parsed_request.body(Vec::<u8>::new()).unwrap();

        println!("Parsed Request: {:?}", parsed_request);

        // Add X-Forwarded-For header

        // Add X-Forwarded-For header
        // request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_stream) {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            _ => return,
        };

        // // If no bytes are read, the client closed the connection
        // if bytes_read == 0 {
        //     log::info!("Client closed the connection");
        //     return;
        // }

        // // Add X-Forwarded-For header
        // let mut request = String::from("X-Forwarded-For: ");
        // request.push_str(&client_stream.peer_addr().unwrap().to_string());
        // request.push_str("\r\n");
        // request.push_str(&String::from_utf8_lossy(&buffer[..]));

        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        request::write_to_stream(&request, &mut upstream_stream).expect("TODO: panic message");
        // Forward the request to the server
        // if let Err(error) = request::write_to_stream(&request, &mut upstream_stream) {
        //     log::error!("Failed to send request to upstream {}: {}", upstream_address, error);
        //     let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
        //     send_response(&mut client_conn, &response);
        //     return;
        // }


        // println!("Request: {}", request);

        // // Relay the request to the upstream server
        // upstream_stream.write_all(request.as_bytes()).unwrap();
        //
        // println!("upstream_stream: {:?}", upstream_stream);

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
    let args = CmdOptions::parse();

    if args.upstream.len() < 1 {
        error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // let count = args.count;
    // let upstream = args.upstream;
    //
    // println!("Upstream: {}", upstream);
    // println!("Count: {}", count);


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
    

    for stream in listener.incoming() {
        println!("New connection: {:?}", stream);
        if let Ok(stream) = stream {
            // Handle the connection!
            handle_connection(stream, &state);
        }
    }
     

}
