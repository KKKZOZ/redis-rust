use std::io::{Read, Write};
use std::net::TcpStream;

use bytes::{BufMut, Bytes, BytesMut};
use tracing::info;

use ResponseType::*;

pub enum ResponseType<'a> {
    SimpleString(&'a str),
    BulkString(Option<String>),
    SimpleError(&'a str),
    RdbFile(Bytes),
}

pub fn write_response(stream: &mut TcpStream, response_type: ResponseType) {
    match response_type {
        SimpleString(content) => {
            stream
                .write_all(format!("+{}\r\n", content).as_bytes())
                .unwrap();
        }
        BulkString(content) => {
            let content = match content {
                Some(content) => format!("${}\r\n{}\r\n", content.len(), content),
                None => format!("$-1\r\n"),
            };
            stream.write_all(content.as_bytes()).unwrap();
        }
        RdbFile(payload) => {
            let mut buffer = BytesMut::new();
            buffer.put_u8(b'$');
            buffer.put_slice(&payload.len().to_string().as_bytes());
            buffer.put_slice(b"\r\n");
            buffer.put_slice(&payload);

            stream.write_all(buffer.as_ref()).unwrap();
        }
        SimpleError(content) => {
            stream
                .write_all(format!("-{}\r\n", content).as_bytes())
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
