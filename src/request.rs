use std::io::{Read, Write};
use std::net::TcpStream;
use http::Request;

#[derive(Debug)]
pub enum Error {
    /// Client sent an invalid HTTP request.
    MalformedRequest,
    /// Client closed the connection
    ClientClosedConnection,
    /// The request is partial, and we could stop parsing it. The path
    /// is not found in the router
    PartialRequest,
}

/// This function serializes a request to bytes and writes those bytes to the provided stream.
///
/// You will need to modify this function in Milestone 2.
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

pub fn format_request_line(request: &Request<Vec<u8>>) -> String {
    format!("{} {} {:?}", request.method(), request.uri(), request.version())
}


//////////////////////////////////////////////////

pub fn request_controller(client_stream: &mut TcpStream, client_ip: &str, upstream_stream: &mut TcpStream) {

    let req= match read_client_request(client_stream){
        Ok(req) => req,
        Err(e) => {
            log::error!("Error reading client request: {:?}", e);
            return;
        }
    };

    let parsed_request = match client_request_builder(client_ip, &req){
        Ok(parsed_request) => parsed_request,
        Err(e) => {
            log::error!("Error building client request: {:?}", e);
            return;
        }
    };

    // transform request into bytes and write to upstream stream
    if let Err(error) = write_to_stream(&parsed_request, upstream_stream){
        log::error!("Failed to send request to upstream server: {}", error);
        return;
    };
    log::debug!("Request sent to upstream server");


}

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
        return Err(Error::ClientClosedConnection).expect("Client closed the connection. EXPECTED");
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