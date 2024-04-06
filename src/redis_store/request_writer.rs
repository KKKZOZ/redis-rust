use std::{io::Write, net::TcpStream};

pub fn request(stream: &mut TcpStream, content: &str) {
    let tokens: Vec<&str> = content.split(" ").collect();
    let mut cmd = format!("*{}\r\n", tokens.len());
    for token in tokens {
        cmd.push_str(format!("${}\r\n{}\r\n", token.len(), token).as_str());
    }
    stream.write_all(cmd.as_bytes()).unwrap();
    stream.flush().unwrap()
}
