use std::{env, fs, path::PathBuf};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use http_server_starter_rust::{http, ser::Serialize};

fn get_file_directory() -> Option<PathBuf> {
    let arg_pairs = env::args().zip(env::args().skip(1));
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
    let buf_read = &buf[0..bytes_read];

    let (_, req) =
        http::Request::parser(buf_read).map_err(|err| err.map(|e| e.input.to_owned()))?;
    let response = if req.req_line.method == http::Method::Get {
        route_get(&req, file_dir)
    } else if req.req_line.method == http::Method::Post {
        route_post(&req, file_dir)
    } else {
        http::Response::new(http::Status::Internal)
    };
    let _bytes_write = stream.write(&response.to_bytes()).await?;

    Ok(())
}

fn route_get(req: &http::Request, file_dir: Option<&PathBuf>) -> http::Response {
    if req.req_line.path == "/" {
        route_get_root()
    } else if let Some(remain) = req.req_line.path.strip_prefix("/echo/") {
        route_get_echo(remain)
    } else if req.req_line.path == "/user-agent" {
        route_get_user_agent(req)
    } else if let Some(remain) = req.req_line.path.strip_prefix("/files/") {
        route_get_files(remain, file_dir)
    } else {
        println!("  GET unknown ({}) - 404", req.req_line.path);
        http::Response::new(http::Status::NotFound)
    }
}

fn route_get_root() -> http::Response {
    println!("  GET Root");
    http::Response::new(http::Status::Ok)
}

fn route_get_echo(path: &str) -> http::Response {
    println!("  GET echo - {path}");
    http::Response::new(http::Status::Ok).with_body(path.as_bytes(), "text/plain")
}

fn route_get_user_agent(req: &http::Request) -> http::Response {
    let user_agent = req
        .headers
        .get("user-agent")
        .map_or_else(String::new, |ua| ua.clone());
    println!("  GET user-agent - {user_agent}");
    http::Response::new(http::Status::Ok).with_body(user_agent.as_bytes(), "text/plain")
}

fn route_get_files(path: &str, file_dir: Option<&PathBuf>) -> http::Response {
    let Some(dir) = file_dir else {
        println!("  GET files - fail, no directory configured");
        return http::Response::new(http::Status::Internal);
    };

    println!("  GET files - {path}");
    let mut file_path = dir.clone();
    file_path.push(path);

    match fs::read_to_string(file_path) {
        Ok(file_data) => http::Response::new(http::Status::Ok)
            .with_body(file_data.as_bytes(), "application/octet-stream"),
        Err(e) => {
            println!("  GET files - fail, {e}");
            http::Response::new(http::Status::NotFound)
        }
    }
}

fn route_post(req: &http::Request, file_dir: Option<&PathBuf>) -> http::Response {
    if let Some(remain) = req.req_line.path.strip_prefix("/files/") {
        route_post_files(req, remain, file_dir)
    } else {
        println!("  POST unknown ({}) - 404", req.req_line.path);
        http::Response::new(http::Status::NotFound)
    }
}

fn route_post_files(req: &http::Request, path: &str, file_dir: Option<&PathBuf>) -> http::Response {
    let Some(dir) = file_dir else {
        println!("  POST files - fail, no directory configured");
        return http::Response::new(http::Status::Internal);
    };

    let Some(body) = &req.body else {
        println!("  POST files - fail, no body provided");
        return http::Response::new(http::Status::BadRequest);
    };

    let Some(content_len) = req.get_content_length() else {
        println!("  POST files - fail, no content-length");
        return http::Response::new(http::Status::BadRequest);
    };

    if content_len > body.len() {
        println!("  POST files - fail, invalid content-length");
        return http::Response::new(http::Status::BadRequest);
    }

    println!("  POST files - {path}");
    let mut file_path = dir.clone();
    file_path.push(path);

    match fs::write(file_path, &body[0..content_len]) {
        Ok(_) => http::Response::new(http::Status::Created),
        Err(e) => {
            println!("  POST files - fail, {e}");
            http::Response::new(http::Status::Internal)
        }
    }
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
