#![feature(globs)]
extern crate mqtt;
//use mqtt::{Connect, ConnectOptions};
use std::io::net::ip::SocketAddr;
use std::io::net::tcp::TcpStream;

fn main() {
	let addr = from_str::<SocketAddr>("127.0.0.1:1883").unwrap();
	let mut socket = TcpStream::connect(addr).unwrap();
	let opts = ::mqtt::ConnectOptions {
		clientId: ~"tim-rust",
		user: None,
		pw: None,
		will: None,
		clean: false,
		keepAlive: 60
	};
	let connect = ::mqtt::Connect(opts);

	connect.encode(socket.write);
	socket.flush();
	//socket.close();
}
