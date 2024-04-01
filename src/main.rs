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
    while let Ok(n) = stream.read(&mut buffer) {
        if n == 0 {
            break;
        }
        stream.write_all(b"+PONG\r\n").unwrap();
        // let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
        // println!("request: {}", request);
        // match request.trim().to_uppercase().as_str() {
        //     "ping" => {
        //         stream.write_all(b"+PONG\r\n").unwrap();
        //     }
        //     _ => {
        //         stream.write_all(b"-ERR unknown command\r\n").unwrap();
        //     }
        // }
    }
    stream.write_all(b"+PONG\r\n").unwrap();
}
