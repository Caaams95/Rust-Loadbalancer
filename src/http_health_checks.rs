//! # HTTP Health Checks Module
//!
//! This module provides functions for performing HTTP health checks on upstream servers.
//!
//! ## Functions
//!
//! ### `basic_http_health_check`
//!
//! This function sends a simple GET request to the upstream server to check if it's healthy. It takes an upstream server IP and a path as parameters.
//!
//! - **Parameters:**
//!   - `upstream_ip`: A String containing the upstream server IP.
//!   - `path`: A String representing the path used for the health check.
//!
//! - **Returns:**
//!   - `Ok(())`: If the health check is successful (200 OK response).
//!   - `Err(std::io::Error)`: If the health check fails, containing details about the error and the upstream server IP.
//!
//! - **Example:**
//!   ```rust
//!   use crate::http_health_checks::basic_http_health_check;
//!
//!   match basic_http_health_check(String::from("127.0.0.1:8080"), String::from("/health")) {
//!       Ok(_) => println!("Health check successful!"),
//!       Err(e) => eprintln!("Health check failed: {}", e),
//!   }
//!   ```
//!
//! ### `simple_get_request`
//!
//! This private function sends a simple GET request to the upstream server to check if it's healthy. It is used internally by `basic_http_health_check`.
//!
//! - **Parameters:**
//!   - `stream`: A mutable reference to a TcpStream.
//!   - `path`: A String representing the path used for the health check.
//!
//! - **Returns:**
//!   - `Ok(())`: If the health check is successful (200 OK response).
//!   - `Err(std::io::Error)`: If the health check fails, containing details about the error.
//!
//! - **Example:**
//!   ```rust
//!   use crate::http_health_checks::simple_get_request;
//!   use std::net::TcpStream;
//!
//!   let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
//!   match simple_get_request(&mut stream, String::from("/health")) {
//!       Ok(_) => println!("Health check successful!"),
//!       Err(e) => eprintln!("Health check failed: {}", e),
//!   }
//!   ```

use std::io::{Read, Write};
use std::net::TcpStream;

/// Performs a basic HTTP health check on the upstream server.
///
/// This function sends a simple GET request to the specified upstream server IP and path to check if it's healthy.
/// The health check is considered successful if the response contains "200 OK."
///
/// # Arguments
///
/// * `upstream_ip` - A String containing the upstream server IP.
/// * `path` - A String representing the path used for the health check.
///
/// # Returns
///
/// * `Ok(())` - If the health check is successful (200 OK response).
/// * `Err(std::io::Error)` - If the health check fails, containing details about the error and the upstream server IP.
///
/// # Example
///
/// ```rust
/// use crate::http_health_checks::basic_http_health_check;
///
/// match basic_http_health_check(String::from("127.0.0.1:8080"), String::from("/health")) {
///     Ok(_) => println!("Health check successful!"),
///     Err(e) => eprintln!("Health check failed: {}", e),
/// }
/// ``` 
pub fn basic_http_health_check(upstream_ip : String, path : String) -> Result< (), std::io::Error> {
    let upstream_address = upstream_ip;

    // send a simple GET request to the upstream server to check if it's healthy
    let mut upstream_stream = match TcpStream::connect(&upstream_address) {
        Ok(stream) => stream,
        Err(_) => {
            //     return a simple error containing the upstream_address
            return Err(std::io::Error::new(std::io::ErrorKind::Other, upstream_address.to_string()));
        }
    };


    // send a simple GET request to the upstream server to check if it's healthy returning 200 OK
    return match simple_get_request(&mut upstream_stream, path) {
        Ok(_) => {
            //     return a simple Ok containing the upstream_address
            Ok(())
        },
        Err(_) => {
            //     return a simple error containing the upstream_address
            Err(std::io::Error::new(std::io::ErrorKind::Other, upstream_address.to_string()))
        }
    }
    
}


/// Sends a simple GET request to the upstream server to check if it's healthy.
///
/// This private function is used internally by `basic_http_health_check`.
///
/// # Arguments
///
/// * `stream` - A mutable reference to a TcpStream.
/// * `path` - A String representing the path used for the health check.
///
/// # Returns
///
/// * `Ok(())` - If the health check is successful (200 OK response).
/// * `Err(std::io::Error)` - If the health check fails, containing details about the error.
///
/// # Example
///
/// ```rust
/// use crate::http_health_checks::simple_get_request;
/// use std::net::TcpStream;
///
/// let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
/// match simple_get_request(&mut stream, String::from("/health")) {
///     Ok(_) => println!("Health check successful!"),
///     Err(e) => eprintln!("Health check failed: {}", e),
/// }
/// ```
fn simple_get_request(stream: &mut TcpStream, path : String) -> Result<(), std::io::Error> {


    // send request on path to the upstream server

    let request = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
    stream.write(request.as_bytes())?;

    // check the http code
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);

    // check if the response contains 200 OK
    if !response.contains("200 OK") {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Non-200 OK response"));
    }

    Ok(())
}