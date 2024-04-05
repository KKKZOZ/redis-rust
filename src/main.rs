use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    let store = Arc::new(RedisStore::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = Arc::clone(&store);
                thread::spawn(move || {
                    store.handle_connection(stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

struct DataItem<T: Clone> {
    value: T,
    ttl: Option<SystemTime>,
}

impl<T: Clone> DataItem<T> {
    fn new(value: T, ttl: Option<SystemTime>) -> Self {
        DataItem { value, ttl }
    }
    fn expired_or_return(&self) -> Option<T> {
        if let Some(ttl) = self.ttl {
            if ttl < SystemTime::now() {
                return None;
            }
        }
        Some(self.value.clone())
    }
}

struct RedisStore {
    data: Mutex<HashMap<String, DataItem<String>>>,
}

impl RedisStore {
    fn new() -> Self {
        RedisStore {
            data: Mutex::new(HashMap::new()),
        }
    }

    fn set(&self, key: String, value: String, ttl: Option<SystemTime>) {
        self.data
            .lock()
            .unwrap()
            .insert(key, DataItem::new(value, ttl));
    }

    fn get(&self, key: &str) -> Option<String> {
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

#[derive(Debug)]
pub enum Command {
    PING,
    ECHO(String),
    SET(String, String, Option<u64>),
    GET(String),
}

impl Command {
    pub fn new(command: String) -> Result<Command> {
        let cmd_array = parse_command_array(&command)?;
        let cmd = parse_to_cmd(cmd_array)?;
        Ok(cmd)
    }
}

fn parse_to_cmd(arr: Vec<&str>) -> Result<Command> {
    match arr[0].to_uppercase().as_str() {
        "PING" => Ok(Command::PING),
        "ECHO" => Ok(Command::ECHO(arr[1].to_string())),
        "SET" => {
            let key = arr[1].to_string();
            let value = arr[2].to_string();
            if arr.len() == 5 && arr[3].to_uppercase() == "PX" {
                let ttl = arr[4].parse::<u64>()?;
                return Ok(Command::SET(key, value, Some(ttl)));
            } else {
                Ok(Command::SET(key, value, None))
            }
        }
        "GET" => Ok(Command::GET(arr[1].to_string())),
        _ => Err(anyhow!("unknown command")),
    }
}

//*2\r\n$4\r\necho\r\n$3\r\nhey\r\n
fn parse_command_array(command: &str) -> Result<Vec<&str>> {
    let mut iter = command.split("\r\n");

    if let Some(cmd_len) = iter.next() {
        if cmd_len.starts_with("*") {
            let cmd_len = cmd_len[1..].parse::<usize>()?;
            let mut cmd_array = Vec::with_capacity(cmd_len);
            for _ in 0..cmd_len {
                // TODO: Deal with nested arrays.
                let _len: usize = iter.next().unwrap()[1..].parse()?;
                cmd_array.push(iter.next().unwrap())
            }
            return Ok(cmd_array);
        } else {
            Result::Err(anyhow!("invalid command"))
        }
    } else {
        Result::Err(anyhow!("invalid command"))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new_command() {
        let cmd = Command::new("*2\r\n$4\r\necho\r\n$3\r\nhey\r\n".to_string()).unwrap();
        if let Command::ECHO(content) = cmd {
            assert_eq!(content, "hey".to_string())
        }
    }
}
