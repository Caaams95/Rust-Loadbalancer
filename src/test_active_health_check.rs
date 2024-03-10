use std::io::{Read, Write};
use std::net::TcpStream;





/// Performs a basic HTTP health check on an upstream server.
///
/// This function sends a simple GET request to the upstream server to check if it's healthy.
/// The health check is successful if the response contains "200 OK."
///
/// # Arguments
///
/// * `upstream_ip` - A String containing the IP address and port of the upstream server.
/// * `path` - A String representing the path for the GET request.
///
/// # Returns
///
/// * `Result<String, std::io::Error>` - If the health check is successful, returns Ok with the upstream address.
///                                    If the health check fails, returns an Err with an I/O error containing the upstream address.


pub fn basic_http_health_check(upstream_ip : String, path : String ) -> Result< String, std::io::Error> {
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
            Ok(upstream_address.to_string())
        },
        Err(_) => {
            //     return a simple error containing the upstream_address
            Err(std::io::Error::new(std::io::ErrorKind::Other, upstream_address.to_string()))
        }
    }
}


/// Send a simple GET request to the upstream server to check if it's healthy.
///
/// This function takes a mutable reference to a TcpStream and returns a Result containing a unit type or an error.
/// The health check is successful if the response contains "200 OK."
///
/// # Arguments
///
/// * `stream` - A mutable reference to a TcpStream connected to the upstream server.
/// * `path` - A String representing the path for the GET request.
///
/// # Returns
///
/// * `Result<(), std::io::Error>` - If the health check is successful, returns Ok.
///                                If the health check fails, returns an Err with an I/O error.
fn simple_get_request(stream: &mut TcpStream, path : String) -> Result<(), std::io::Error> {


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

#[test]
fn test_active_health_check() {

    let status = basic_http_health_check("171.67.215.200:80".to_string(), "/".to_string())
    .map_or(-1, |_| 1);

    assert_eq!(status, 1);
}


#[test]
fn test_inactive_health_check() {

    let status = basic_http_health_check("1.1.1.1".to_string(), "/".to_string())
    .map_or(-1, |_| 1);
    
    assert_eq!(status, -1);
}
