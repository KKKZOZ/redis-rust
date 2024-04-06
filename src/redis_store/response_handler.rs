use std::io::{Read, Write};
use std::net::TcpStream;

use tracing::info;

pub enum ResponseType {
    SimpleString,
    BulkString,
    SimpleError,
}

pub fn write_response(stream: &mut TcpStream, response_type: ResponseType, content: Option<&str>) {
    match response_type {
        ResponseType::SimpleString => {
            stream
                .write_all(format!("+{}\r\n", content.unwrap()).as_bytes())
                .unwrap();
        }
        ResponseType::BulkString => {
            let content = match content {
                Some(content) => format!("${}\r\n{}\r\n", content.len(), content),
                None => format!("$-1\r\n"),
            };
            stream.write_all(content.as_bytes()).unwrap();
        }
        ResponseType::SimpleError => {
            stream
                .write_all(format!("-{}\r\n", content.unwrap()).as_bytes())
                .unwrap();
        }
    }
}

pub fn read_response(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = [0; 512];
    if let Ok(n) = stream.read(&mut buffer) {
        let response = String::from_utf8_lossy(&buffer[..n]).to_string();
        let header = response.chars().nth(0).unwrap();
        let len = response.len() - 2;
        let resp = match header {
            '+' => response[1..len].to_string(),
            '-' => response[1..len].to_string(),
            _ => response[1..len].to_string(),
        };
        info!("Response: {}", resp);
        Some(resp)
    } else {
        None
    }
}
