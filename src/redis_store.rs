use std::{
    collections::HashMap, io::Read, net::TcpStream, sync::Mutex, time::Duration, time::SystemTime,
};

use crate::command::Command;

use super::data_item::DataItem;

mod response_writer;

use response_writer::*;

pub struct RedisStore {
    data: Mutex<HashMap<String, DataItem<String>>>,
}

impl RedisStore {
    pub fn new() -> Self {
        RedisStore {
            data: Mutex::new(HashMap::new()),
        }
    }

    pub fn set(&self, key: String, value: String, ttl: Option<SystemTime>) {
        self.data
            .lock()
            .unwrap()
            .insert(key, DataItem::new(value, ttl));
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().get(key)?.expired_or_return()
    }

    pub fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 512];
        while let Ok(n) = stream.read(&mut buffer) {
            if n == 0 {
                break;
            }
            let command = Command::new(String::from_utf8_lossy(&buffer[..n]).to_string());
            match command {
                Ok(cmd) => match cmd {
                    Command::PING => {
                        response(&mut stream, ResponseType::SimpleString, Some("PONG"));
                    }
                    Command::ECHO(content) => {
                        response(&mut stream, ResponseType::SimpleString, Some(&content));
                    }
                    Command::INFO(_section) => {
                        response(&mut stream, ResponseType::BulkString, Some("role:master"));
                    }
                    Command::SET(key, value, ttl) => {
                        let ttl = match ttl {
                            Some(ttl) => SystemTime::now().checked_add(Duration::from_millis(ttl)),
                            None => None,
                        };
                        self.set(key, value, ttl);
                        response(&mut stream, ResponseType::SimpleString, Some("OK"));
                    }
                    Command::GET(key) => {
                        if let Some(value) = self.get(&key) {
                            response(&mut stream, ResponseType::BulkString, Some(&value));
                        } else {
                            response(&mut stream, ResponseType::BulkString, None);
                        }
                    }
                },
                Err(_e) => {
                    response(
                        &mut stream,
                        ResponseType::SimpleError,
                        Some("ERR unknown command"),
                    );
                }
            }
        }
    }
}
