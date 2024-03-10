use std::io::Write;
use std::net::TcpStream;
use http::Request;

#[test]
fn write_to_stream() -> Result<(), std::io::Error> {
    // request: &Request<Vec<u8>>, stream: &mut TcpStream
    
    let request = Request::builder()
        .method("GET")
        .uri("http://localhost:8080/")
        .header("User-Agent", "curl/7.68.0")
        .body(Vec::new())
        .unwrap();
    
    let mut stream = TcpStream::connect("171.67.215.200:80")?;
    
    
    stream.write(&crate::request::format_request_line(&request).into_bytes())?;
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

#[test]

