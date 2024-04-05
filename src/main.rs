use anyhow::{anyhow, Result};
use std::io::Write;
use std::thread;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
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
            },
            Err(_e) => {
                stream.write_all(b"-ERR unknown command\r\n").unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub enum Command {
    PING,
    ECHO(String),
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
