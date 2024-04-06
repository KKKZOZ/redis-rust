use std::net::TcpListener;
use std::sync::Arc;
use std::{env, thread};

use redis_starter_rust::redis_store::RedisStore;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut port: &str = "6379"; // default port
    for (i, arg) in args.iter().enumerate() {
        if arg == "--port" {
            if i < args.len() - 1 {
                port = &args[i + 1];
            }
        }
    }
    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(address).unwrap();

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
