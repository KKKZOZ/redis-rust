#![allow(unused_variables, unused_assignments)]
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use redis_starter_rust::redis_store::{RedisStore, Role};

use clap::Parser;

use tracing::info;
use tracing_subscriber;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    port: Option<String>,
    #[arg(short, long, num_args = 2)]
    replicaof: Option<Vec<String>>,
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let port = cli.port.unwrap_or("6379".to_string());
    let mut master_address = None;

    let role = match cli.replicaof {
        Some(replicaof) => {
            master_address = Some(format!("{}:{}", replicaof[0], replicaof[1]));
            Role::Slave
        }
        None => Role::Master,
    };

    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&address).unwrap();
    let master_stream = match master_address {
        Some(address) => Some(TcpStream::connect(address).unwrap()),
        _ => None,
    };
    let store = Arc::new(RedisStore::new(role, address, master_stream));
    match store.start() {
        Ok(_) => info!("Server started on port {}", port),
        Err(e) => info!("Error starting server: {}", e),
    }

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
