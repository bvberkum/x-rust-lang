extern crate encoding;

use std::env;
use std::io::prelude::*;
use std::net::TcpStream;
use encoding::{Encoding, EncoderTrap};
use encoding::all::ASCII;

const HOST: &'static str = "127.0.0.1:12345";

fn main() {
    let command = match env::args().nth(1) {
        Some(cmd) => cmd,
        None => {
            let my_name = env::args().nth(0).unwrap();
            panic!("Usage: {} [command]", my_name)
        }
    };
    let mut command_bytes = ASCII.encode(&command, EncoderTrap::Strict).unwrap();
    command_bytes.push('\n' as u8);
    let mut stream = TcpStream::connect(HOST).unwrap();
    stream.write_all(&command_bytes).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    println!("{}", response);
}
