use std::io::{Read, Write};
use std::net::TcpStream;
use http::Request;

/// Enum representing possible errors during request handling.

#[derive(Debug)]
pub enum Error {
    /// Client sent an invalid HTTP request.
    MalformedRequest,
    /// Client closed the connection
    ClientClosedConnection,
    /// The request is partial, and we could stop parsing it. The path
    /// is not found in the router
    PartialRequest,
    /// Encountered an I/O error when reading/writing a TcpStream
    ConnectionError,
}

/// Serializes a request to bytes and writes those bytes to the provided stream.
///
/// This function serializes the given HTTP request to bytes and writes them to the provided TcpStream.
/// It includes the request line, headers, and body.
///
/// # Arguments
///
/// * `request` - The HTTP request to be serialized and sent.
/// * `stream` - The TcpStream to which the serialized request will be written.
///
/// # Returns
///
/// * `Ok(())` - If the serialization and writing process is successful.
/// * `Err(std::io::Error)` - If there is an error during the serialization or writing process.
fn write_to_stream(request: &Request<Vec<u8>>,stream: &mut TcpStream) -> Result<(), std::io::Error> {
    stream.write(&format_request_line(request).into_bytes())?;
    stream.write(&['\r' as u8, '\n' as u8])?; // \r\n
    for (header_name, header_value) in request.headers() {
        stream.write(&format!("{}: ", header_name).as_bytes())?;
        stream.write(header_value.as_bytes())?;
        stream.write(&['\r' as u8, '\n' as u8])?; // \r\n
    }
    stream.write(&['\r' as u8, '\n' as u8])?;
    if request.body().len() > 0 {
        stream.write(request.body())?;
    }
    Ok(())
}


/// Formats the request line of an HTTP request.
///
/// This function takes an HTTP request and returns a formatted string containing the request line,
/// including the method, URI, and version.
///
/// # Arguments
///
/// * `request` - The HTTP request for which the request line will be formatted.
///
/// # Returns
///
/// * `String` - The formatted request line.
pub fn format_request_line(request: &Request<Vec<u8>>) -> String {
    format!("{} {} {:?}", request.method(), request.uri(), request.version())
}


/// Controls the flow of incoming requests and handles the communication with the upstream server.
///
/// This function reads an HTTP request from the client, processes it, and sends the parsed request to the upstream server.
///
/// # Arguments
///
/// * `client_stream` - A mutable reference to the TcpStream connected to the client.
/// * `client_ip` - The IP address of the client.
/// * `upstream_stream` - A mutable reference to the TcpStream connected to the upstream server.
///
/// # Returns
///
/// * `Ok(())` - If the handling process is successful.
/// * `Err(Error)` - If there is an error during the handling process.
/// 
/// 
pub fn request_controller(client_stream: &mut TcpStream, client_ip: &str, upstream_stream: &mut TcpStream) -> Result<(), Error>{

    let req= match read_client_request(client_stream){
        Ok(req) => req,
        Err(Error::ClientClosedConnection) => {
            log::info!("Client closed the connection");
        //     return err 
            return Err(Error::ClientClosedConnection);
        },
        Err(e) => {
            log::error!("Error reading client request: {:?}", e);
            return Err(e)
        }
    };

    let parsed_request = match client_request_builder(client_ip, &req){
        Ok(parsed_request) => parsed_request,
        Err(e) => {
            log::error!("Error building client request: {:?}", e);
            return Err(e)
        }
    };

    // transform request into bytes and write to upstream stream
    if let Err(error) = write_to_stream(&parsed_request, upstream_stream){
        log::error!("Failed to send request to upstream server: {}", error);
        return Err(Error::ConnectionError);
    };
    log::debug!("Request sent to upstream server");
    
    Ok(())
}


/// Reads the client's HTTP request from the provided TcpStream.
///
/// This function attempts to read the client's HTTP request from the provided TcpStream.
/// If successful, it returns the parsed HTTP request. If the client closes the connection or
/// there is an error during the read operation, an appropriate error is returned.
///
/// # Arguments
///
/// * `client_stream` - A mutable reference to the TcpStream connected to the client.
///
/// # Returns
///
/// * `Result<Request<Vec<u8>>, Error>` - The result containing the parsed HTTP request or an error.
fn read_client_request(client_stream: &mut TcpStream) -> Result<Request<Vec<u8>>, Error>{
    let mut buffer = [0; 1024];
    let bytes_read = match client_stream.read(&mut buffer) {
        Ok(bytes) => bytes,
        Err(_) => {
            // Error handling in case the client sends a malformed request
            let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
            client_stream.write(response.as_bytes()).unwrap();
            return Err(Error::MalformedRequest);
        }
    };

    // If no bytes are read, the client closed the connection
    if bytes_read == 0 {
        log::info!("Client closed the connection");
        // return Err(Error::ClientClosedConnection).expect("Client closed the connection. EXPECTED");
    //     return and expect are not compatible
    //     do something if the program panics
        return Err(Error::ClientClosedConnection);
    } 

    // read the request from the client
    let mut headers = [httparse::EMPTY_HEADER; 16];

    let mut req = httparse::Request::new(&mut headers as &mut [httparse::Header]);

    let res = req.parse(&buffer).unwrap();

    // if the request is partial, we could stop parsing
    if res.is_partial() {
        match req.path {
            Some(ref path) => {
                // check router for path.
                // /404 doesn't exist? we could stop parsing
                println!("Path: {:?}", path);
                log::info!("Path: {:?}", path);                
            },
            None => {
                // we could stop parsing
                return Err(Error::PartialRequest);
            }
        }
    }

    // build parsed request with method, uri and version
    let mut parsed_request = http::Request::builder()
        .method(req.method.unwrap())
        .uri(req.path.unwrap())
        .version(http::Version::HTTP_11);

    // add headers to parsed request
    for header in req.headers {
        parsed_request = parsed_request.header(header.name, header.value);
    }

    // build parsed request with body and unwrap it
    let parsed_request = parsed_request.body(Vec::<u8>::new()).unwrap();

    return Ok(parsed_request)
}




/// Builds a modified client request by adding the client's IP and returns the new request.
///
/// # Arguments
///
/// * `client_ip` - A string representing the client's IP address.
/// * `req` - A reference to the original client request.
///
/// # Returns
///
/// * `Ok(Request<Vec<u8>>)` - If the modified client request is successfully created.
/// * `Err(Error)` - If an error occurs during the building process.


fn client_request_builder (client_ip: &str, req: &Request<Vec<u8>>) -> Result<Request<Vec<u8>>, Error>{

    // build parsed request with method, uri and version
    let mut parsed_request = Request::builder()
        .method(req.method())
        .uri(req.uri())
        .version(http::Version::HTTP_11);

    // add headers to parsed request
    for header in req.headers() {
        parsed_request = parsed_request.header(header.0, header.1);
    }


    parsed_request = parsed_request.header("X-Forwarded-For", client_ip);

    // build parsed request with body and unwrap it
    let parsed_request = parsed_request.body(Vec::<u8>::new()).unwrap();

    println!("\nParsed Request: {:?}", parsed_request);
    log::info!("\nParsed Request: {:?}", parsed_request);

    // return parsed request
    Ok(parsed_request)
}