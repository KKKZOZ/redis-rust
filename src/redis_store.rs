// #![allow(dead_code)]
use std::{
    collections::HashMap, fmt, io::Read, net::TcpStream, sync::Mutex, time::Duration,
    time::SystemTime,
};

use crate::command::Command;

use super::data_item::DataItem;

mod response_writer;

use response_writer::*;

pub enum Role {
    Master,
    Slave,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Role::Master => write!(f, "master"),
            Role::Slave => write!(f, "slave"),
        }
    }
}

pub struct ReplicationConfig {
    role: Role,
    master_replid: String,
    master_repl_offset: u64,
}

impl ReplicationConfig {
    pub fn new(role: Role) -> Self {
        ReplicationConfig {
            role,
            master_replid: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_owned(),
            master_repl_offset: 0,
        }
    }
}

impl fmt::Display for ReplicationConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "role:{}\r\nmaster_replid:{}\r\nmaster_repl_offset:{}",
            self.role, self.master_replid, self.master_repl_offset
        )
    }
}

pub struct RedisStore {
    data: Mutex<HashMap<String, DataItem<String>>>,
    repli_config: ReplicationConfig,
}

impl RedisStore {
    pub fn new(role: Role) -> Self {
        RedisStore {
            data: Mutex::new(HashMap::new()),
            repli_config: ReplicationConfig::new(role),
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
                    Command::INFO(section) => match section.as_str() {
                        "replication" => {
                            response(
                                &mut stream,
                                ResponseType::BulkString,
                                Some(self.repli_config.to_string().as_str()),
                            );
                        }
                        &_ => {
                            response(
                                &mut stream,
                                ResponseType::SimpleError,
                                Some("ERR unknown section"),
                            );
                        }
                    },
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
