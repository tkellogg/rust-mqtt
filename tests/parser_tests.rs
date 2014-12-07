extern crate mqtt;

use std::io::TcpStream;
use mqtt::parser::{encode, decode, Message, QoS, SubAckCode};
use std::time::duration::Duration;
use std::io::timer::sleep;

#[test]
fn send_connect_msg() {
	let d = Duration::milliseconds(10);

	let mut socket = TcpStream::connect("127.0.0.1:1883").unwrap();
	let mut socket_readable = socket.clone();

	let connect_buf = encode::connect("tim-rust", None, None, 60, true, None);

	let mut res = socket.write(connect_buf.as_slice());
	res = res.and_then(|_| socket.flush());

	//sleep(d);

	let mut buf = [0, ..1024];
	match socket_readable.read(buf.as_mut_slice()) {
		Ok(len) => {
			println!("{} bytes read, buf.len() == {}", len, buf.len());
			match decode(buf.slice_to(len)) {
				Some(Message::Connack(code)) => println!("Connected with code '{}'", code),
				Some(other) => println!("Not CONACK"),
				_ => panic!("CONACK was not returned")
			};
		},
		Err(e) => println!("Couldn't read CONACK becase '{}'", e)
	};

	let msg_buf = encode::publish("io.m2m/rust/thingy", "{\"foo\":\"39.737567,-104.9847178\"}", QoS::AtMostOnce, false, false, None);
	res = res.and_then(|_| socket.write(msg_buf.as_slice()));
	println!("Write publish: {}", res);
	res = res.and_then(|_| socket.flush());
	println!("Flush: {}", res);

	res = res.and_then(|_| socket.write(encode::pingreq().as_slice()));
	res = res.and_then(|_| socket.flush());

	sleep(d);

	let mut ping_buf = [0 as u8, 4];
	match socket_readable.read(ping_buf.as_mut_slice()) {
		Ok(len) => assert!(decode(ping_buf.slice_to(len)) == Some(Message::PingResp)),
		Err(e) => panic!("Couldn't read e: {}", e)
	};

	res = res.and_then(|_| socket.write(encode::subscribe("io.m2m/rust/y", QoS::AtMostOnce, 1).as_slice()));
	res = res.and_then(|_| socket.flush());

	sleep(d);

	let mut buf2 = [0, ..1024];
	match socket_readable.read(buf2.as_mut_slice()) {
		Ok(len) => {
			println!("Decode SUBACK!");
			let suback = decode(buf2.slice_to(len));
			match suback {
				Some(Message::SubAck(subs)) => {
					assert_eq!(subs.len(), 1);
					assert!((*subs).get(0) == Some(&SubAckCode::SubAckSuccess(QoS::AtMostOnce)));
				},
				Some(other) => panic!("Expected SUBACK"),
					
				_ => panic!("Was not successful")
			}
		},
		Err(e) => panic!("Couldn't read SUBACK because '{}'", e)
	};

	res = res.and_then(|_| socket.write(encode::disconnect().as_slice()));
	res = res.and_then(|_| socket.flush());

	match res {
		Ok(_) => println!("success"),
		Err(e) => println!("Test failed: {}", e)
	};
}
