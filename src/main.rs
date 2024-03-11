use std::{io::{Error, Read, Write}, net::{TcpListener, TcpStream}};

fn handle_conn(stream: TcpStream) -> Result<(), Error> {
    let mut stream = stream;

    let mut buf = [0u8; 1024];
    let _bytes_read = stream.read(&mut buf)?;

    let _bytes_write = stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;

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
