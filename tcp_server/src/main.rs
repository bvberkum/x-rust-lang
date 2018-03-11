extern crate tokio;

use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;


fn main() {
  let addr = "127.0.0.1:12345".parse().unwrap();
  let listener = TcpListener::bind(&addr).unwrap();
  
  let server = listener.incoming().for_each(|socket| {
    println!("accepted socket; addr={:?}", socket.peer_addr().unwrap());
  
  	let (reader, writer) = socket.split();
  	let conn = io::copy(reader, writer)
  		.map(|(n, _, _)| {
  			println!("wrote {} bytes", n)
  		})
        .map_err(|err| {
            println!("IO error {:?}", err)
        });
  
  	// Spawn the future as a concurrent task
  	tokio::spawn(conn);
  
  	Ok(())
  })
  .map_err(|err| {
  	println!("server error {:?}", err);
  });
  
  println!("server running on {}", addr);
  tokio::run(server);
}
