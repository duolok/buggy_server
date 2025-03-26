use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let server = "127.0.0.1:8080";
    let total_length = get_total_length(server)?;
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

        let chunk = download_chunk(server, start, end)?;
        let chunk_len = chunk.len();
        println!("Downloaded chunk: {} bytes (requested {}-{})\n", chunk_len, start, end);

        if chunk_len == 0 {
            println!("No more data received, exiting loop.");
            break;
        }

        downloaded_data.extend_from_slice(&chunk);
        start += chunk_len;
    }

    let hash = calculate_hash(&downloaded_data);
    println!("SHA-256 hash of downloaded data: {}", hash);

    Ok(())
}

fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&data);
    format!("{:x}", hasher.finalize())
}

fn get_total_length(server: &str) -> Result<usize> {
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        server
    );
    let response = send_request(server, &request)?;
    let (headers, _) = split_response(&response)?;
    let headers_map = parse_headers(headers)?;

    let content_length_str =headers_map
        .get("content-length")
        .ok_or("Content-Length header not found")?
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse content-length: {}", e))?;

    Ok(content_length_str)
}

fn send_request(server: &str, request: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect(server)?;
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}

fn split_response(response: &[u8]) -> Result<(&str, Vec<u8>)> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("Failed to find end of headers")?
        + 4;

    let headers = str::from_utf8(&response[..header_end])?;
    let body = response[header_end..].to_vec();
    Ok((headers, body))
}

fn parse_headers(response: &str) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();

    for line in response.lines() {
        if line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    Ok(headers)
}

fn download_chunk(server: &str, start: usize, end: usize) -> Result<Vec<u8>> {
    let range_header = format!("Range: bytes={}-{}\r\n", start, end);
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\n{}\r\nConnection: close\r\n\r\n",
        server, range_header
    );

    let response = send_request(server, &request)?;
    let (headers, body) = split_response(&response)?;

    let headers_map = parse_headers(headers)?;
    let status_code = parse_status_code(headers)?;

    if !status_code != 200 && status_code != 206 {
        return Err(format!("Unexpected status code: {}", status_code).into());
    }

    let expected_length = headers_map
        .get("content-length")
        .ok_or("Content-Length header not found")?
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse content-length: {}", e))?;

    Ok(body.into_iter().take(expected_length).collect())
}

fn parse_status_code(response: &str) -> Result<u16> {
    let status_line = response.lines().next().ok_or("Empty response.")?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();

    if parts.len() != 4 {
        return Err("Invalid status line.".into());
    }

    let code = parts[1].parse::<u16>()?;
    Ok(code)
}
