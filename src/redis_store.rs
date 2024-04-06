// #![allow(dead_code)]
use std::{
    collections::HashMap,
    fmt,
    io::Read,
    net::TcpStream,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use anyhow::Result;

use crate::command::Command;

use super::data_item::DataItem;

mod request_writer;
mod response_handler;

use request_writer::*;
use response_handler::*;

#[derive(PartialEq)]
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
    address: String,
    data: Mutex<HashMap<String, DataItem<String>>>,
    repli_config: ReplicationConfig,
    master_stream: Mutex<Option<TcpStream>>,
}

impl RedisStore {
    pub fn new(role: Role, address: String, stream: Option<TcpStream>) -> Self {
        RedisStore {
            address: address,
            data: Mutex::new(HashMap::new()),
            repli_config: ReplicationConfig::new(role),
            master_stream: Mutex::new(stream),
        }
    }

    pub fn start(&self) -> Result<()> {
        if self.repli_config.role == Role::Slave {
            let mut master_stream = self.master_stream.lock().unwrap();
            let stream = master_stream.as_mut().unwrap();
            request(stream, "PING");
            assert_eq!(read_response(stream).unwrap(), "PONG", "expect PONG");
            let port = self.address.split(":").collect::<Vec<&str>>()[1];
            request(stream, format!("REPLCONF listening-port {}", port).as_str());
            assert_eq!(read_response(stream).unwrap(), "OK", "expect OK");
            request(stream, "REPLCONF capa psync2");
            assert_eq!(read_response(stream).unwrap(), "OK", "expect OK");
        }
        Ok(())
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
                        write_response(&mut stream, ResponseType::SimpleString, Some("PONG"));
                    }
                    Command::ECHO(content) => {
                        write_response(&mut stream, ResponseType::SimpleString, Some(&content));
                    }
                    Command::REPLCONF => {
                        write_response(&mut stream, ResponseType::SimpleString, Some("OK"));
                    }
                    Command::INFO(section) => match section.as_str() {
                        "replication" => {
                            write_response(
                                &mut stream,
                                ResponseType::BulkString,
                                Some(self.repli_config.to_string().as_str()),
                            );
                        }
                        &_ => {
                            write_response(
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
                        write_response(&mut stream, ResponseType::SimpleString, Some("OK"));
                    }
                    Command::GET(key) => {
                        if let Some(value) = self.get(&key) {
                            write_response(&mut stream, ResponseType::BulkString, Some(&value));
                        } else {
                            write_response(&mut stream, ResponseType::BulkString, None);
                        }
                    }
                },
                Err(_e) => {
                    write_response(
                        &mut stream,
                        ResponseType::SimpleError,
                        Some("ERR unknown command"),
                    );
                }
            }
        }
    }
}
