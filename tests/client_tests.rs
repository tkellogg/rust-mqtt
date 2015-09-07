extern crate mqtt;

use std::default::Default;
use mqtt::client::{Client, ConnectOptions, MqttError};
use mqtt::parser::{Message, QoS};

#[test]
fn connect_to_broker() {
	let client = &mut new_client();
  assert_ok(client.connect());
	assert_ok(client.disconnect())
}

#[test]
fn blind_publish() {
	let client = &mut new_client();
  assert_ok(client.connect());
	assert_ok(client.publish("foo/bar", "test message", QoS::AtMostOnce, false, false));
}

#[test]
fn subscribe_and_receive_suback() {
	let client = &mut new_client();
  assert_ok(client.connect());
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

fn new_client<'a>() -> Client<'a> {
	Client { 
		options : ConnectOptions {
			host_port: "iot.eclipse.og:1883",
			client_id: "rust-test", 
			clean: true,
			..Default::default()
		}, 
		..Default::default() 
	}
}

fn assert_ok<A>(res: Result<A, MqttError>) {
	match res {
		Ok(_) => (),
		Err(e) => panic!(e)
	}
}
