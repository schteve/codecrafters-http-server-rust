use std::{path::PathBuf, str::from_utf8};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use http_server_starter_rust::http;

fn get_file_directory() -> Option<PathBuf> {
    let arg_pairs = std::env::args().zip(std::env::args().skip(1));
    for (a, b) in arg_pairs {
        if a == "--directory" {
            let mut dir = PathBuf::new();
            dir.push(b);
            return Some(dir);
        }
    }
    None
}

async fn handle_conn(stream: TcpStream, file_dir: Option<&PathBuf>) -> anyhow::Result<()> {
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
            .with_body(remain.to_owned(), "text/plain")
    } else if req.req_line.path == "/user-agent" {
        let user_agent = req
            .headers
            .get("user-agent")
            .map_or_else(String::new, |ua| ua.clone());
        println!("  GET user-agent - {user_agent}");
        http::Response::new()
            .with_status(http::Status::Ok)
            .with_body(user_agent, "text/plain")
    } else if let Some(remain) = req.req_line.path.strip_prefix("/files/") {
        if let Some(dir) = file_dir {
            println!("  GET files - {remain}");
            let mut file_path = dir.clone();
            file_path.push(remain);

            match std::fs::read_to_string(file_path) {
                Ok(file_data) => http::Response::new()
                    .with_status(http::Status::Ok)
                    .with_body(file_data, "application/octet-stream"),
                Err(_) => http::Response::new().with_status(http::Status::NotFound),
            }
        } else {
            println!("  GET files - fail, no directory configured");
            http::Response::new().with_status(http::Status::Internal)
        }
    } else {
        println!("  GET unknown - 404");
        http::Response::new().with_status(http::Status::NotFound)
    };
    let _bytes_write = stream.write(response.to_string().as_bytes()).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let file_dir = get_file_directory();

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("Accepted new connection");
                let f = file_dir.clone(); // Clone before move
                tokio::spawn(async move {
                    match handle_conn(stream, f.as_ref()).await {
                        Ok(_) => println!("Connection handled successfully"),
                        Err(e) => println!("Error handling connection: {e}"),
                    }
                });
            }
            Err(e) => println!("Failed to accept new connection: {e}"),
        }
    }
}
