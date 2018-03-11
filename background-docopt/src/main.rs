//! Test
#![deny(warnings)]

extern crate tokio;
extern crate tokio_io;
#[macro_use]
extern crate futures;
extern crate bytes;
#[macro_use]
extern crate serde_derive;
extern crate docopt;


use docopt::Docopt;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use futures::sync::mpsc;
use futures::future::{self, Either};
use bytes::{BytesMut, Bytes, BufMut};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};



const USAGE: &'static str = "
background-docopt

Usage:
  local-bg cmd1 --opt <arg>
  local-bg --serve
  local-bg --exit
  local-bg --version
  local-bg -h|--help

Options
  -h, --help    Show this screen.
  --version     Show version.
  --opt         Example option for cmd1.
  --address=<addr>
                Set TCP IP address and post for background process
                [default: 127.0.0.1:12345].
  --serve       Sart server.
  --exit        Close server.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_opt: bool,
    flag_address: SocketAddr,
    flag_serve: bool,
    flag_exit: bool,
    arg_arg: String,
    cmd_cmd1: bool,
}

//const USAGE_NF: &'static str = "
//Naval Fate.
//
//Usage:
//  naval_fate.py ship new <name>...
//  naval_fate.py ship <name> move <x> <y> [--speed=<kn>]
//  naval_fate.py ship shoot <x> <y>
//  naval_fate.py mine (set|remove) <x> <y> [--moored | --drifting]
//  naval_fate.py (-h | --help)
//  naval_fate.py --version
//
//Options:
//  -h --help     Show this screen.
//  --version     Show version.
//  --speed=<kn>  Speed in knots [default: 10].
//  --moored      Moored (anchored) mine.
//  --drifting    Drifting mine.
//";
//
//#[derive(Debug, Deserialize)]
//struct NfArgs {
//    flag_speed: isize,
//    flag_drifting: bool,
//    arg_name: Vec<String>,
//    arg_x: Option<i32>,
//    arg_y: Option<i32>,
//    cmd_ship: bool,
//    cmd_mine: bool,
//}


type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;

struct Shared {

    peers: HashMap<SocketAddr, Tx>,
}

struct Peer {

    name: BytesMut,
    lines: Lines,
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    addr: SocketAddr,
}

#[derive(Debug)]
struct Lines {

    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}


impl Shared {
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }
}

impl Peer {
    fn new(name: BytesMut,
           state: Arc<Mutex<Shared>>,
           lines: Lines) -> Peer
    {
        let addr = lines.socket.peer_addr().unwrap();
        let (tx, rx) = mpsc::unbounded();
        state.lock().unwrap()
            .peers.insert(addr, tx);
        Peer {
            name,
            lines,
            state,
            rx,
            addr,
        }
    }
}

impl Future for Peer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {

        const LINES_PER_TICK: usize = 10;

        for i in 0..LINES_PER_TICK {
            match self.rx.poll().unwrap() {
                Async::Ready(Some(v)) => {
                    self.lines.buffer(&v);

                    if i+1 == LINES_PER_TICK {
                        task::current().notify();
                    }
                }
                _ => break,
            }
        }

        let _ = self.lines.poll_flush()?;

        while let Async::Ready(line) = self.lines.poll()? {
            println!("Received line ({:?}) : {:?}", self.name, line);

            if let Some(message) = line {
                // Append the peer's name to the front of the line:
                let mut line = self.name.clone();
                line.put(": ");
                line.put(&message);
                line.put("\r\n");

                // Freeze bytes before sending it to every peer as clones
                let line = line.freeze();

                for (addr, tx) in &self.state.lock().unwrap().peers {
                    if *addr != self.addr {
                        tx.unbounded_send(line.clone()).unwrap();
                    }
                }
            } else { // EOF
                return Ok(Async::Ready(()));
            }
        }

        Ok(Async::NotReady)
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.state.lock().unwrap().peers
            .remove(&self.addr);
    }
}


/// The lines codec
impl Lines {

    fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    fn buffer(&mut self, line: &[u8]) {
        // Ensure the buffer has capacity. Ideally this would not be unbounded,
        // but to keep the example simple, we will not limit this.
        self.wr.reserve(line.len());

        // Push the line onto the end of the write buffer.
        //
        // The `put` function is from the `BufMut` trait.
        self.wr.put(line);
    }

    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // As long as there is buffered data to write, try to write it.
        while !self.wr.is_empty() {
            // Try to read some bytes from the socket
            let n = try_ready!(self.socket.poll_write(&self.wr));

            // As long as the wr is not empty, a successful write should
            // never write 0 bytes.
            assert!(n > 0);

            // This discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }

    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            // Ensure the read buffer has capacity.
            //
            // This might result in an internal allocation.
            self.rd.reserve(1024);

            // Read data into the buffer.
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl Stream for Lines {

    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {

        // First, read any new data that might have been received off the socket
        let sock_closed = self.fill_read_buf()?.is_ready();

        // Now, try finding lines
        let pos = self.rd.windows(2).enumerate()
            .find(|&(_, bytes)| bytes == b"\r\n")
            .map(|(i, _)| i);

        if let Some(pos) = pos {
            // Remove the line from the read buffer and set it to `line`.
            let mut line = self.rd.split_to(pos + 2);

            // Drop the trailing \r\n
            line.split_off(pos);

            // Return the line
            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

//fn welcome(socket: TcpStream) -> Future<Item = u32, Error = io::Error> {

	//socket.write(b"Name please?\n").unwrap();
//}

fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {

    let lines = Lines::new(socket);

    let conn = lines.into_future()
        .map_err(|(e, _)| e)
        .and_then(|(name, lines)| {

            let name = match name {
                Some(name) => name,
                None => {
                    // The remote client closed the connection without sending
                    // any data.
                    return Either::A(future::ok(()));
                }
            };
            println!("`{:?}` is joining the chat", name);
            let peer = Peer::new(
                name,
                state,
                lines);

            // Wrap `peer` with `Either::B` to make the return type fit.
            Either::B(peer)
        })
        .map_err(|e| {
            println!("Connection error = {:?}", e);
        });

    tokio::spawn(conn);
}

pub fn main() {
    let d = Docopt::new(USAGE);
    let args: Args = d
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_serve {
        let state = Arc::new(Mutex::new(Shared::new()));

        //let argv = || vec!["cp", "-a", "file1", "file2", "dest/"];

        //let args: Args = d
        //    .and_then(|d| d.argv(argv().into_iter()).deserialize())
        //    .unwrap_or_else(|e| e.exit());

        let listener = TcpListener::bind(&args.flag_address).unwrap();
        let server = listener.incoming().for_each(move |socket| {
            
            println!("accepted socket; addr={:?}", socket.peer_addr().unwrap());
            process(socket, state.clone());
            Ok(())
        })
        .map_err(|err| {
            println!("accept error {:?}", err);
        });

        println!("server running on {}", &args.flag_address);
        tokio::run(server);

    } else if args.flag_exit {

    } else {

    }
}
