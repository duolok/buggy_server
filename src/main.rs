use std::io::{Read, Write};
use std::net::TcpStream;
use std::str;
use sha2::{Sha256, Digest};
use std::error::Error;

//type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() {
    let server = "127.0.0.1:8080";
    let total_length = get_total_length(server);
    println!("Total length of data: {} \n\n", total_length);

    // pre-allocate the vector with expected capacity
    let mut downloaded_data = Vec::with_capacity(total_length);
    let chunk_size = 64 * 1024;
    let mut start = 0;

    while start < total_length {
        let end = if start + chunk_size > total_length {
            total_length
        } else {
            start + chunk_size
        };

        let chunk = download_chunk(server, start, end);
        let chunk_len = chunk.len();
        println!("Downloaded chunk: {} bytes (requested {}-{})\n", chunk_len, start, end);

        if chunk_len == 0 {
            println!("No more data received, exiting loop.");
            break;
        }

        downloaded_data.extend_from_slice(&chunk);
        start += chunk_len;
    }

    if downloaded_data.len() != total_length {
        println!(
            "Warning: Downloaded data length ({}) does not match expected total length ({})",
            downloaded_data.len(),
            total_length
        );
    }

    let mut hasher = Sha256::new();
    hasher.update(&downloaded_data);
    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    println!("SHA-256 hash of downloaded data: {}", hash);
}

fn get_total_length(server: &str) -> Result<usize> {
    let request = format!("GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", server);

    let response = send_request(server, &request)?;

    // Find the end of headers (look for "\r\n\r\n")
    let headers_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("Failed to find end of headers.") + 4;

    let headers = &response[..headers_end];
    let headers_str = str::from_utf8(headers)
        .expect("Failed to convert headers to string.");
    parse_content_length(headers_str).unwrap_or(0)
}

fn send_request(server: &str, request: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect(server).expect("Failed to connect to the server.");
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}

fn parse_content_length(headers: &str) -> Result<usize> {
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

fn download_chunk(server: &str, start: usize, end: usize) -> Vec<u8> {
    let mut stream = TcpStream::connect(server).expect("Failed to connect to the server.");

    let range_header = format!("Range: bytes={}-{}\r\n", start, end);
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\n{}\r\nConnection: close\r\n\r\n",
        server, range_header
    );

    stream.write_all(request.as_bytes()).expect("Failed to send GET request");
    
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("Failed to read response");

    // find end of headers
    let headers_end = match response.windows(4).position(|window| window == b"\r\n\r\n") {
        Some(pos) => pos + 4,
        None => {
            eprintln!("End of headers not found");
            return Vec::new();
        }
    };


    let headers = &response[..headers_end];
    let headers_str = match str::from_utf8(headers) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Invalid UTF-8 in headers");
            return Vec::new();
        }
    };
    println!("{:?}", &headers_str);

    // parse status code
    let status_line = headers_str.lines().next().unwrap_or("");
    let status_code = status_line.split_whitespace().nth(1).unwrap_or("0");
    let status_code: u16 = status_code.parse().unwrap_or(0);

    if !(status_code == 200 || status_code == 206) {
        eprintln!("Unexpected status code: {}", status_code);
        return Vec::new();
    }

    // parse Content-Length from the response header
    let content_length = match parse_content_length(headers_str) {
        Ok(len) => len,
        Err(e) => {
            eprintln!("{}", e);
            return Vec::new();
        }
    };

    let body_start = headers_end;
    let mut body_end = body_start + content_length;


    println!("body start: {} , content_length: {}, response len: {}", &body_start, &content_length, &response.len());
    if body_start + content_length > response.len() {
        body_end = response.len() - body_start;
    }

    response[body_start..body_end].to_vec()
}
