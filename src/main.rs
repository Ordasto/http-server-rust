// use std::io::{BufRead, BufReader, Read, Write};
//use std::net::TcpListener;

use std::env::{self, args, Args};
use std::fs;

use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeek, AsyncWriteExt, BufReader,
    ReadBuf, SeekFrom,
};
// use tokio::io::*;
use tokio::net::{TcpListener, TcpStream};

// enum HttpMethod {
//     GET,
// }

// struct HttpHeader {
//     name: String,
//     value: String,
// }

// struct HttpRequest {
//     // Start line
//     method: HttpMethod,
//     target: String,
//     version: String, // Http Version has inconsistent formatting (1.0 and then 2 without the .0)
//     // Headers
//     headers: Vec<HttpHeader>, // Consider a hashmap
//     body: Option<String>,
// }

// impl HttpRequest {
//     fn new(request: Vec<String>) -> Result<HttpRequest, &'static str> {
//         let mut request_iter = request.iter();
//         let method_str = request_iter.next().ok_or("Method not found.");

//         return Err("Not Implemented");
//     }
// }

fn form_response(content: &str, content_type: &str) -> String {
    let content_len = content.as_bytes().len();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        content_type, content_len, content
    )
    .to_string()
}

#[tokio::main]
async fn main() {
    for arg in env::args() {
        println!("{}", arg);
    }
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        tokio::spawn(handle_connection(listener.accept().await.unwrap().0));
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    let mut request_data: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        let bytes_read = buf_reader.read_line(&mut line).await.unwrap();
        if bytes_read == 0 || line.trim().is_empty() || line.is_empty() {
            break;
        }
        request_data.push(line.trim().to_string());
    }

    let start_line: Vec<&str> = request_data[0].split(' ').collect();
    match start_line[1] {
        "/" => {
            stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
        }
        "/user-agent" => {
            //
            for header in request_data {
                if header.starts_with("User-Agent:") {
                    let content = header.split_at(12).1;

                    stream
                        .write_all(form_response(content, "text/plain").as_bytes())
                        .await
                        .unwrap();
                }
            }
        }
        line if line.starts_with("/files/") => {
            let method = start_line[0].split(' ').nth(0).unwrap();
            let path = start_line[1];
            let (_directory, file_name) = path.split_at(7);
            let directory = args().nth(2).unwrap();
            match method {
                "POST" => {
                    // Incredibly cursed
                    let content_length = request_data
                        .iter()
                        .find(|header| header.starts_with("Content-Length"))
                        .unwrap()
                        .split(' ')
                        .nth(1)
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();

                    let mut data: Vec<u8> = vec![65; content_length];
                    match buf_reader.read_exact(&mut data).await {
                        Ok(bytes_read) => println!("Succesfully read {} bytes.", bytes_read),
                        Err(e) => println!("Unable to read bytes : {}", e),
                    }
                    match tokio::fs::create_dir(directory.clone()).await {
                        Ok(_) => println!("Succesfully created directory."),
                        Err(e) => println!("Cannot create directory: {}", e),
                    }

                    match tokio::fs::write(format!("{}{}", directory, file_name), data).await {
                        Ok(_) => println!("Succesfully wrote file."),
                        Err(e) => println!("Cannot write file: {}", e),
                    }

                    stream
                        .write_all(b"HTTP/1.1 201 Created\r\n\r\n")
                        .await
                        .unwrap();
                }
                "GET" => match fs::read_to_string(format!("{}/{}", directory, file_name)) {
                    Ok(data) => {
                        stream
                            .write_all(
                                form_response(data.as_str(), "application/octet-stream").as_bytes(),
                            )
                            .await
                            .unwrap();
                    }
                    Err(_) => {
                        stream
                            .write_all(b"HTTP/1.1 404 NOT FOUND\r\n\r\n")
                            .await
                            .unwrap();
                    }
                },
                _ => {
                    stream
                        .write_all(b"HTTP/1.1 404 NOT FOUND\r\n\r\n")
                        .await
                        .unwrap();
                }
            }
        }
        line if line.starts_with("/echo/") => {
            let content = start_line[1].split_at(6).1;
            stream
                .write_all(form_response(content, "text/plain").as_bytes())
                .await
                .unwrap();
        }
        _ => {
            stream
                .write_all(b"HTTP/1.1 404 NOT FOUND\r\n\r\n")
                .await
                .unwrap();
        }
    }
}
