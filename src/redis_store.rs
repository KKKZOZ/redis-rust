use std::{
    collections::HashMap, io::Read, io::Write, net::TcpStream, sync::Mutex, time::Duration,
    time::SystemTime,
};

use crate::command::Command;

use super::data_item::DataItem;

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
                        stream.write_all(b"+PONG\r\n").unwrap();
                    }
                    Command::ECHO(content) => {
                        stream
                            .write_all(format!("+{}\r\n", content).as_bytes())
                            .unwrap();
                    }
                    Command::SET(key, value, ttl) => {
                        let ttl = match ttl {
                            Some(ttl) => SystemTime::now().checked_add(Duration::from_millis(ttl)),
                            None => None,
                        };
                        self.set(key, value, ttl);
                        stream.write_all(b"+OK\r\n").unwrap();
                    }
                    Command::GET(key) => {
                        if let Some(value) = self.get(&key) {
                            stream
                                .write_all(format!("+{}\r\n", value).as_bytes())
                                .unwrap();
                        } else {
                            stream.write_all(b"$-1\r\n").unwrap();
                        }
                    }
                },
                Err(_e) => {
                    stream.write_all(b"-ERR unknown command\r\n").unwrap();
                }
            }
        }
    }
}
