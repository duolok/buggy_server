# Buggy Server

This is a simple client application written in Rust that downloads binary data from the glitchy server. 
The server sends some randomized data on every GET request, but often it doesn't send the whole data, but just a part of the data.
The server supports the HTTP header "Range", which basically means that you can request a specific fragment of the data you want. It allows you 
to ask for any size of data.

Rust application downloads (sends GET requests to the server) the data and then prints the SHA256 hash of the data. That hash should match the hash server outputs at the beginning.
The application is written with Rust's standard library and sha2 crate.

## Running 

Start the server: 
```bash
python3 buggy_server.py
```

Run Client app:
```bash 
cargo run
```
