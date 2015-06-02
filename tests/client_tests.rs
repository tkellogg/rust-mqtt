extern crate mqtt;

use std::default::Default;
use mqtt::client::{Client, ConnectOptions, MqttError};
use mqtt::parser::{Message, QoS};

#[test]
fn connect_to_broker() {
	let mut client = connect();
	assert_ok(client.disconnect())
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
		Ok(_) => panic!("Not a SUBSCRIBE"),
		Err(e) => panic!(e)
	};

	assert_ok(client.unsubscribe(vec!["foo/bar"]));
	match client.recv() {
		Ok(Message::UnsubAck) => (),
		Ok(_) => panic!("Not an UNSUBSCRIBE"),
		Err(e) => panic!(e)
	}
}

fn connect<'a>() -> &'a mut Client<'a> {
	let client = &mut Client { 
		options : ConnectOptions {
			host_port: "localhost:1883",
			client_id: "rust-test", 
			clean: true,
			..Default::default()
		}, 
		..Default::default() 
	};

	match client.connect() {
		Ok(_) => &mut *client,
		Err(e) => panic!(e)
	}
}

fn assert_ok<A>(res: Result<A, MqttError>) {
	match res {
		Ok(_) => (),
		Err(e) => panic!(e)
	}
}
