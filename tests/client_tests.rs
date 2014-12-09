extern crate mqtt;

use std::default::Default;
use mqtt::client::{Client, ConnectOptions};
use mqtt::parser::{Message};

#[test]
fn connect_to_broker() {
	let mut client = Client { 
		options : ConnectOptions {
			host_port: "localhost:1883",
			client_id: "rust-test", 
			..Default::default()
		}, 
		..Default::default() 
	};

	match client.connect() {
		Ok(Message::Connack(0)) => (),
		Ok(msg) => panic!(msg),
		Err(e) => panic!(e)
	};
}
