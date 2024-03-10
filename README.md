# Asynchronous Proxy Server in Rust

This module implements a simple asynchronous proxy server in Rust. The server listens for incoming TCP connections,
proxies the requests to one of the specified upstream servers, and forwards the responses back to the client.

## Modules

- `request`: Module for handling client requests.
- `http_health_checks`: Module for performing HTTP-based health checks on upstream servers.
- `test_active_health_check`: Module for testing active health check functionality.
- `test_request`: Module for testing request handling functionality.

## Dependencies

- `clap`: Command line argument parsing.
- `log`: Logging macros.
- `rand`: Random number generation for load balancing among upstream servers.
- `tokio`: Asynchronous runtime.

## Usage

To run the proxy server, use the following command:

 ```sh
 cargo run -- --upstream <upstream-server-1> --upstream <upstream-server-2> ... --bind <bind-address> --interval <health-check-interval> --path <health-check-path>
 ```

## Options

- `--upstream`: Upstream server(s) to proxy to.
- `--bind`: The address to bind the proxy server to.
- `--interval`: Interval between each health check in seconds. Default is 5 seconds.
- `--path`: The path to use for active health checks. Default value is "/".

## Structures

- `CmdOptions`: Represents the command-line options for configuring the proxy server.
- `ProxyState`: Represents the state of the proxy server, including active health check settings and upstream server addresses.

## Functions

- `connect_to_upstream_server`: Attempts to connect to an upstream server.
- `handle_connection`: Asynchronously handles incoming client connections, proxies requests, and forwards responses.

## Main Function

The `main` function initializes the proxy server by parsing command line arguments, creating a listener for incoming connections,
and starting asynchronous tasks for active health checks and connection handling.