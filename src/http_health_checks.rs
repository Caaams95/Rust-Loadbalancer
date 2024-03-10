use std::io::{Read, Write};
use std::net::TcpStream;
use tokio::time::{sleep, Duration};

/// This function sends a simple GET request to the upstream server to check if it's healthy
/// It takes a String containing the upstream server IP and return a Result containing a String or an error
/// The health check is successful if the response contains 200 OK
/// If the health check is successful, the function returns a String containing the upstream server IP
/// If the health check fails, the function returns an error containing the upstream server IP
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

/// Send a simple GET request to the upstream server to check if it's healthy
/// It takes a mutable reference to a TcpStream and return a Result containing a unit type or an error
/// The health check is successful if the response contains 200 OK
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