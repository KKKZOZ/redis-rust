#![allow(unused_variables, unused_assignments)]
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use redis_starter_rust::redis_store::{RedisStore, Role};

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    port: Option<String>,
    #[arg(short, long, num_args = 2)]
    replicaof: Option<Vec<String>>,
}

fn main() {
    let cli = Cli::parse();

    let port = cli.port.unwrap_or("6379".to_string());
    let master_host: &str;
    let master_port: &str;

    let role = match cli.replicaof {
        Some(replicaof) => {
            master_host = &replicaof[0];
            master_port = &replicaof[1];
            Role::Slave
        }
        None => Role::Master,
    };

    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(address).unwrap();

    let store = Arc::new(RedisStore::new(role));

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

#[cfg(test)]
mod tests {

    use redis_starter_rust::command::*;

    #[test]
    fn test_new_command() {
        let cmd = Command::new("*2\r\n$4\r\necho\r\n$3\r\nhey\r\n".to_string()).unwrap();
        if let Command::ECHO(content) = cmd {
            assert_eq!(content, "hey".to_string())
        }
    }
}
