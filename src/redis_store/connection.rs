use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
};

pub enum ResponseType<'a> {
    SimpleString(&'a str),
    BulkString(Option<String>),
    SimpleError(&'a str),
    RdbFile(Bytes),
}
use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use tracing::info;
use ResponseType::*;

use crate::command::{parse_to_cmd, Command};
pub struct Connection {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let writer = stream.try_clone().unwrap();
        Connection {
            reader: BufReader::new(stream),
            writer,
        }
    }

    pub fn write_request(&mut self, content: &str) {
        let tokens: Vec<&str> = content.split(" ").collect();
        let mut cmd = format!("*{}\r\n", tokens.len());
        for token in tokens {
            cmd.push_str(format!("${}\r\n{}\r\n", token.len(), token).as_str());
        }
        self.writer.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn write_response(&mut self, response_type: ResponseType) {
        match response_type {
            SimpleString(content) => {
                self.writer
                    .write_all(format!("+{}\r\n", content).as_bytes())
                    .unwrap();
            }
            BulkString(content) => {
                let content = match content {
                    Some(content) => format!("${}\r\n{}\r\n", content.len(), content),
                    None => format!("$-1\r\n"),
                };
                self.writer.write_all(content.as_bytes()).unwrap();
            }
            RdbFile(payload) => {
                let mut buffer = BytesMut::new();
                buffer.put_u8(b'$');
                buffer.put_slice(&payload.len().to_string().as_bytes());
                buffer.put_slice(b"\r\n");
                buffer.put_slice(&payload);

                self.writer.write_all(buffer.as_ref()).unwrap();
            }
            SimpleError(content) => {
                self.writer
                    .write_all(format!("-{}\r\n", content).as_bytes())
                    .unwrap();
            }
        }
    }

    pub fn read_request(&mut self) -> Result<Command> {
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();

        while buffer.is_empty() {
            self.reader.read_line(&mut buffer).unwrap();
        }

        let arr_len = buffer[1..2].parse::<usize>().unwrap();

        let mut cmd_arr = vec![];
        for _ in 0..arr_len {
            let mut buf = String::new();
            self.reader.read_line(&mut buf).unwrap();
            self.reader.read_line(&mut buf).unwrap();
            let tokens = buf.split("\r\n").collect::<Vec<&str>>();
            cmd_arr.push(tokens[1].to_owned());
        }

        info!("cmd_arr: {:?}", cmd_arr);
        parse_to_cmd(cmd_arr.iter().map(AsRef::as_ref).collect())
    }

    pub fn read_response(&mut self) -> Option<String> {
        let mut buffer = String::new();

        self.reader.read_line(&mut buffer).unwrap();
        if buffer == "\r\n" {
            self.reader.read_line(&mut buffer).unwrap();
        }

        info!("buffer: {:?}", buffer);

        let header = buffer.chars().nth(0).unwrap();

        let resp = match header {
            '+' | '-' => buffer[1..buffer.len() - 2].to_string(),
            '$' => {
                let len = buffer[1..buffer.len() - 2].parse::<usize>().unwrap();
                let mut payload = vec![0; len];
                self.reader.read_exact(&mut payload).unwrap();
                String::from_utf8_lossy(&payload).to_string()
            }
            _ => {
                info!("unsupported response type");
                return None;
            }
        };

        Some(resp)
    }
}
