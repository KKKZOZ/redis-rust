use std::io::Write;
use std::net::TcpStream;

pub enum ResponseType {
    SimpleString,
    BulkString,
    SimpleError,
}

pub fn response(stream: &mut TcpStream, response_type: ResponseType, content: Option<&str>) {
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
