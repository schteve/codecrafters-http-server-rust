use std::str::from_utf8;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use http_server_starter_rust::http;

async fn handle_conn(stream: TcpStream) -> anyhow::Result<()> {
    let mut stream = stream;

    let mut buf = [0u8; 1024];
    let bytes_read = stream.read(&mut buf).await?;
    let buf_str = from_utf8(&buf[0..bytes_read])?;

    let (_, req) = http::Request::parser(buf_str).map_err(|err| err.map(|e| e.input.to_owned()))?;

    let response = if req.req_line.path == "/" {
        println!("  GET Root");
        http::Response::new().with_status(http::Status::Ok)
    } else if let Some(remain) = req.req_line.path.strip_prefix("/echo/") {
        println!("  GET echo - {remain}");
        http::Response::new()
            .with_status(http::Status::Ok)
            .with_body(remain.to_owned())
    } else if req.req_line.path == "/user-agent" {
        let user_agent = req
            .headers
            .get("user-agent")
            .map_or_else(String::new, |ua| ua.clone());
        println!("  GET user-agent - {user_agent}");
        http::Response::new()
            .with_status(http::Status::Ok)
            .with_body(user_agent)
    } else {
        println!("  GET unknown - 404");
        http::Response::new().with_status(http::Status::NotFound)
    };
    let _bytes_write = stream.write(response.to_string().as_bytes()).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("Accepted new connection");
                tokio::spawn(async move {
                    match handle_conn(stream).await {
                        Ok(_) => println!("Connection handled successfully"),
                        Err(e) => println!("Error handling connection: {e}"),
                    }
                });
            }
            Err(e) => println!("Failed to accept new connection: {e}"),
        }
    }
}
