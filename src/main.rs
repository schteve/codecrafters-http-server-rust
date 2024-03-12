use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    str::from_utf8,
};

use http_server_starter_rust::http;

fn handle_conn(stream: TcpStream) -> anyhow::Result<()> {
    let mut stream = stream;

    let mut buf = [0u8; 1024];
    let bytes_read = stream.read(&mut buf)?;
    let buf_str = from_utf8(&buf[0..bytes_read])?;

    let (_, req) =
        http::Request::parser(buf_str).map_err(|err| err.map(|e| e.input.to_owned()))?;
    let status = if req.req_line.path == "/" {
        http::Status::Ok
    } else {
        http::Status::NotFound
    };

    let response = http::Response::with_status(status);
    let _bytes_write = stream.write(response.serialize().as_bytes())?;

    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                println!("Accepted new connection");
                match handle_conn(stream) {
                    Ok(_) => (),
                    Err(e) => println!("Error handling connection: {e}"),
                }
            }
            Err(e) => {
                println!("Error accepting new connection: {}", e);
            }
        }
    }
}
