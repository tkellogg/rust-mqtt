extern crate mqtt;

use std::default::Default;
use mqtt::client::{Client, ConnectOptions, MqttError};
use mqtt::parser::{Message, QoS};

#[test]
fn connect_to_broker() {
	connect();
	()
}

#[test]
fn blind_publish() {
	let mut client = connect();
	client.publish("foo/bar", "test message", QoS::AtMostOnce, false, false);
}

#[test]
fn subscribe_and_receive_suback() {
	let mut client = connect();
	let subs = vec![("foo/bar", QoS::AtMostOnce)];
	assert_ok(client.subscribe(subs));

	match client.recv() {
		Ok(Message::SubAck(_)) => (),
		Ok(m) => panic!("Wrong kind of message"),
		Err(e) => panic!(e)
	}
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

fn assert_ok<A>(res: Result<A, MqttError>) {
	match res {
		Ok(_) => (),
		Err(e) => panic!(e)
	}
}
