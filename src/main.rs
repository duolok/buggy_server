use std::io::{Read, Write};
use std::net::TcpStream;
use std::str;

fn main() {
    let server = "127.0.0.1:8080";
    let total_length = get_total_length_and_hash(server);
    println!("Total length of data: {}", total_length);
}

fn get_total_length_and_hash(server: &str) -> usize {
    let mut stream = TcpStream::connect(server).expect("Failed to connect to the server.");
    let request = format!("GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", server);

    stream.write_all(request.as_bytes()).expect("Failed to send GET request.");

    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("Failed to read response.");

    // find the end of the headers 
    let headers_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("Failed to find end of headers.")
        + 4;

    let headers = &response[..headers_end];
    let headers_str = str::from_utf8(headers).expect("Failed to convert headers to string.");
    let content_length = parse_content_length(headers_str).unwrap_or(0);

    content_length
}

fn parse_content_length(headers: &str) -> Result<usize, &'static str> {
    for line in headers.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            let parts: Vec<&str> = line.splitn(2, ':').collect();

            if parts.len() != 2 {
                return Err("Invalid Content-Length header");
            }

            let value = parts[1].trim();
            return value.parse::<usize>().map_err(|_| "Failed to parse Content-Length");
        }
    }

    Err("content-length header not found")
}
