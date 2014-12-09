extern crate mqtt;

use std::default::Default;
use mqtt::client::{Client, ConnectOptions};
use mqtt::parser::{Message};

#[test]
fn connect_to_broker() {
	connect();
	()
}

fn connect<'a>() -> Client<'a> {
	let mut client = Client { 
		options : ConnectOptions {
			host_port: "localhost:1883",
			client_id: "rust-test", 
			clean: true,
			..Default::default()
		}, 
		..Default::default() 
	};

	match client.connect() {
		Ok(()) => client,
		Err(e) => panic!(e)
	}
}
