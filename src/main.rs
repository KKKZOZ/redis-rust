// Uncomment this block to pass the first stage
use std::io::Write;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();
    stream.write_all(b"+PONG\r\n").unwrap();
}
