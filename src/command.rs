use core::fmt;

use anyhow::{anyhow, Ok, Result};

#[derive(Debug)]
pub enum Command {
    PING,
    ECHO(String),
    SET(String, String, Option<u64>),
    GET(String),
    INFO(String),
    REPLCONF(Option<(String, String)>),
    PSYNC(String, String),
}

impl Command {
    pub fn new(command: String) -> Result<Command> {
        let cmd_array = parse_command_array(&command)?;
        let cmd = parse_to_cmd(cmd_array)?;
        Ok(cmd)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::PING => write!(f, "PING"),
            Command::ECHO(content) => write!(f, "ECHO {}", content),
            Command::SET(key, value, ttl) => {
                if let Some(ttl) = ttl {
                    write!(f, "SET {} {} PX {}", key, value, ttl)
                } else {
                    write!(f, "SET {} {}", key, value)
                }
            }
            Command::GET(key) => write!(f, "GET {}", key),
            Command::INFO(section) => write!(f, "INFO {}", section),
            Command::REPLCONF(content) => match content {
                Some((part, offset)) => write!(f, "REPLCONF {} {}", part, offset),
                None => write!(f, "REPLCONF"),
            },
            Command::PSYNC(replid, offset) => write!(f, "PSYNC {} {}", replid, offset),
        }
    }
}

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

pub fn parse_to_cmd(arr: Vec<&str>) -> Result<Command> {
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
        "INFO" => Ok(Command::INFO(arr[1].to_string())),
        "REPLCONF" => {
            if arr.len() == 1 {
                Ok(Command::REPLCONF(None))
            } else {
                Ok(Command::REPLCONF(Some((
                    arr[1].to_string(),
                    arr[2].to_string(),
                ))))
            }
        }
        "PSYNC" => Ok(Command::PSYNC(arr[1].to_string(), arr[2].to_string())),
        _ => Err(anyhow!("unknown command")),
    }
}
