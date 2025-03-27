use std::net::TcpStream;
use std::io::{Read, Write};
use std::error::Error;
use std::collections::HashMap;
use std::str;
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const CHUNK_SIZE: usize = 32 * 1024;
const SERVER: &str = "127.0.0.1:8080";

fn main() -> Result<()> {
    let total_length = get_total_length(SERVER)?;
    println!("Total length of data: {} \n", total_length);

    // pre-allocate the vector with expected capacity 
    // it works with Vector::new as well but used with capacity to ensure predetermined size
    let mut downloaded_data = Vec::with_capacity(total_length);
    let mut start = 0;

    while start < total_length {
        let mut end = start + CHUNK_SIZE;
        if end > total_length {
            end = total_length;
        }

        let chunk = download_chunk(SERVER, start, end)?;
        let chunk_len = chunk.len();
        println!("Downloaded chunk: {} bytes (requested {}-{})", chunk_len, start, end);

        downloaded_data.extend_from_slice(&chunk);
        start += chunk_len;
    }

    let hash = calculate_hash(&downloaded_data);
    println!("\nSHA-256 hash of downloaded data: {}", hash);

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


fn parse_status_code(response: &str) -> Result<u16> {
    let status_line = response.lines().next().ok_or("Empty response.")?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();

    if parts.len() != 4 {
        return Err("Invalid status line.".into());
    }

    let code = parts[1].parse::<u16>()?;
    Ok(code)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let header_str = "\
            Content-Length: 123\r\n\
            Content-Type: text/plain\r\n\
            \r\n";

        let headers = parse_headers(header_str).unwrap();
        assert_eq!(headers.get("content-length").unwrap(), "123");
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");
    }

    #[test]
    fn test_parse_headers_should_fail() {
        let header_str = "\
            Content-Length: 456\r\n\
            Content-Type: text/json\r\n\
            \r\n";

        let headers = parse_headers(header_str).unwrap();
        assert_ne!(headers.get("content-length").unwrap(), "123");
        assert_ne!(headers.get("content-type").unwrap(), "text/plain");
    }

    #[test]
    fn test_parse_status_code() {
        let response = "HTTP/1.1 206 Partial Content";
        let code = parse_status_code(response).unwrap();
        assert_eq!(code, 206);
    }

    #[test]
    fn test_parse_status_code_not_equal() {
        let response = "HTTP/1.1 206 Partial Content";
        let code = parse_status_code(response).unwrap();
        assert_ne!(code, 200);
    }


    #[test]
    fn test_split_response() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        let (headers, body) = split_response(response).expect("Should parse correctly");

        assert!(headers.contains("HTTP/1.1 200 OK"));
        assert!(headers.contains("Content-Length: 5"));
        assert_eq!(body, b"Hello");
    }

    #[test]
    fn test_split_response_no_separator_fail() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5";
        let result = split_response(response);

        assert!(result.is_err(), "Should fail if no \\r\\n\\r\\n separator is found");
    }

    #[test]
    fn test_split_response_empty_body() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let (headers, body) = split_response(response).expect("Should parse correctly");

        assert!(headers.contains("HTTP/1.1 200 OK"));
        assert!(headers.contains("Content-Length: 0"));
        assert!(body.is_empty(), "Body should be empty");
    }
}
