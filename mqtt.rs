#![crate_name = "mqtt"]
#![desc = "An Message Queue Telemetry Transport (MQTT) client"]
#![crate_type = "lib"]
#![feature(struct_variant)]


pub mod mqtt {
	
	#[deriving(FromPrimitive)] 
	pub enum QoS { 
		AtMostOnce = 0, 
		AtLeastOnce = 1, 
		ExactlyOnce = 2
	}

	#[deriving(FromPrimitive)] 
	pub enum MessageType {
		// Can an owned box reside on the stack? I don't think you can have a barrowed ptr in a struct.
		CONNECT = 1,
		CONNACK = 2,
		PUBLISH = 3,
		PUBACK = 4,
		PUBREC = 5,
		PUBREL = 6,
		PUBCOMP = 7,
		SUBSCRIBE = 8,
		SUBACK = 9,
		UNSUBSCRIBE = 10,
		UNSUBACK = 11,
		PINGREQ = 12,
		PINGRESP = 13,
		DISCONNECT = 14
	}

	pub struct LastWill {
		topic: Box<str>,
		msg: Box<str>,
		qos: QoS,
		retain: bool
	}

	/// Messages that can be parsed by `decode`.
	pub enum Message {
		Connack(u8),
		PingReq,
		PingResp,
		Disconnect
	}

	fn parse_short(data: &[u8], index: uint) -> Option<u16> {
		let b1 = data.get(index).and_then(|x| x.to_u16());
		let b2 = data.get(index + 1).and_then(|x| x.to_u16());
		b1.and_then(|x| b2.map(|y| {
			(x << 8) | y
		}))
	}

	pub mod encode {
		use mqtt::{LastWill, QoS};

		#[inline]
		fn msb(i: u16) -> u8 {
			((i & 0xff00) >> 8) as u8
		}

		#[inline]
		fn lsb(i: u16) -> u8 {
			(i & 0xff) as u8
		}

		#[inline]
		fn fshift<T>(opt: Option<&T>, by: uint) -> u8 {
			opt.map_or(0, |_| 1 << by)
		}

		fn rlen_size(remaining_len: uint) -> uint {
			match remaining_len {
				x if x < 128 => 1,
				x if x < 16384 => 2,
				x if x < 2097152 => 3,
				_ => 4
			}
		}

		fn rlen(buf: &mut Vec<u8>, remaining_length: u32) {
			fn nth_mask(n: u8, rl: u32) -> u8 {
				let shift = (((n as uint) - 1) * 7) + 8;
				let mask = 0x7F << shift;
				let val = rl & mask;
				let shifted = val >> shift;
				(shifted | 0x80) as u8
			}

			match remaining_length {
				x if x < 128 => buf.push(x as u8),
				x if x < 16384 => {
					buf.push(nth_mask(1, x));
					buf.push((x & 0xff) as u8);
				}
				x if x < 2097152 => {
					buf.push(nth_mask(2, x));
					buf.push(nth_mask(1, x));
					buf.push((x & 0xff) as u8);
				}
				x => {
					buf.push(nth_mask(3, x));
					buf.push(nth_mask(2, x));
					buf.push(nth_mask(1, x));
					buf.push((x & 0xff) as u8);
				}
			}
		}

		/// Encode connect message into a vector so it can be written into a TCP stream
		pub fn connect(client_id: &str, user: Option<&str>, pass: Option<&str>, keep_alive: u16, clean: bool, lwt: Option<&LastWill>) -> Vec<u8> {
			fn lwt_len(lwt: Option<&LastWill>) -> uint {
				lwt.map_or(0, |opt| { opt.topic.len() + opt.msg.len() })
			}

			fn lwt_flags(lwt: Option<&LastWill>) -> u8 {
				lwt.map_or(0, |lw| { (lw.retain as u8) << 5 | ((lw.qos as u8) & 0x03) << 3 | 1 << 2 })
			}
			
			let id_len = client_id.len() + 2;
			let user_len = user.map_or(0, |u| u.len());
			let pw_len = pass.map_or(0, |u| u.len());
			// 10 = fixed_header + size + "MQTT" + protocol_lvl_byte + connect_flags + client_id+ will_topic + will_msg + will_flags
			let msg_len = 10 + id_len + lwt_len(lwt) + user_len + pw_len;
			
			let flags = fshift(user.as_ref(), 7) |
									fshift(pass.as_ref(), 6)   |
									lwt_flags(lwt) |
									(clean as u8) << 1;

			let mut buf: Vec<u8> = Vec::with_capacity(msg_len);
			let remaining_len = 10 + client_id.len() + 2;
			buf.push_all([0x10, remaining_len as u8, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 4, flags, msb(keep_alive), lsb(keep_alive)]);

			let client_len = client_id.len() as u16;
			buf.push(msb(client_len));
			buf.push(lsb(client_len));
			buf.push_all(client_id.as_bytes());

			for will in lwt.iter() {
				let topic_len = will.topic.len() as u16;
				buf.push(msb(topic_len));
				buf.push(lsb(topic_len));
				buf.push_all(will.topic.as_bytes());

				let msg_len = will.msg.len() as u16;
				buf.push(msb(msg_len));
				buf.push(lsb(msg_len));
				buf.push_all(will.msg.as_bytes());
			}

			for user in user.iter() {
				let len = user.len() as u16;
				buf.push(msb(len));
				buf.push(lsb(len));
				buf.push_all(user.as_bytes());
			}

			for pw in pass.iter() {
				let len = pw.len() as u16;
				buf.push(msb(len));
				buf.push(lsb(len));
				buf.push_all(pw.as_bytes());
			}

			buf
		}

		pub fn publish(topic: &str, msg: &str, qos: QoS, retain: bool, dup: bool, id: Option<u16>) -> Vec<u8> {

			let pid_len = id.map_or(0, |_| 2);
			let remaining_len = 2 + topic.len() + msg.len() + pid_len;
			let buf_len = remaining_len + 1 + rlen_size(remaining_len) as uint;
			let mut buf: Vec<u8> = Vec::with_capacity(buf_len);

			let fixed_header = 0x30 | ((dup as u8) << 3) | ((qos as u8) << 1) | (retain as u8);
			buf.push(fixed_header);

			rlen(&mut buf, remaining_len as u32);

			let topic_len = topic.len() as u16;
			buf.push(msb(topic_len));
			buf.push(lsb(topic_len));
			buf.push_all(topic.as_bytes());

			for v in id.iter() {
				buf.push(msb(*v));
				buf.push(lsb(*v));
			}

			buf.push_all(msg.as_bytes());

			buf
		}

		pub fn pingreq() -> Vec<u8> {
			use mqtt::PINGREQ;
			let b: u8 = (PINGREQ as u8) << 4;
			vec!(b, 0 as u8)
		}

		pub fn disconnect() -> Vec<u8> {
			use mqtt::DISCONNECT;
			let b: u8 = (DISCONNECT as u8) << 4;
			vec!(b, 0 as u8)
		}
	}

	// 987326 Tuesday 8-10am

	pub fn decode(data: &[u8]) -> Option<Message> {
		let hd = data.head();
		let msg: Option<MessageType> = hd.and_then(|x| FromPrimitive::from_u8(*x));
		msg.and_then(|x| match x {
			CONNECT => None,
			CONNACK => parse_connack(data),
			PUBLISH => None,
			PUBACK => None,
			PUBREC => None,
			PUBREL => None,
			PUBCOMP => None,
			SUBSCRIBE => None,
			SUBACK => None,
			UNSUBSCRIBE => None,
			UNSUBACK => None,
			PINGREQ => Some(PingReq),
			PINGRESP => Some(PingResp),
			DISCONNECT => Some(Disconnect)
		})
	}

	fn parse_connack(data: &[u8]) -> Option<Message> {
		let remaining_length = parse_short(data, 1);
		let ret_code = remaining_length.and_then(|x| match x {
			2 => data.get(4),
			_ => None
		});
		ret_code.and_then(|r| Some(Connack(*r)))
	}

	#[cfg(test)]
	pub mod tests {
		use std::io::net::tcp::TcpStream;
		use mqtt::{AtMostOnce};
		use mqtt::{encode, decode};
		use std::time::duration::Duration;
		use std::io::timer::sleep;

		#[test]
		fn send_connect_msg() {
			println!("connecting to localhost");
			let mut socket = TcpStream::connect("127.0.0.1", 1883).unwrap();
			println!("connected?!");
			let connect_buf = encode::connect("tim-rust", None, None, 60, true, None);

			let mut res = socket.write(connect_buf.as_slice());
			//res = res.and_then(|_| socket.flush());

			let msg_buf = encode::publish("io.m2m/rust/thingy", "{\"foo\":\"39.737567,-104.9847178\"}", AtMostOnce, false, false, None);
			res = res.and_then(|_| socket.write(msg_buf.as_slice()));
			res = res.and_then(|_| socket.flush());

			let d = Duration::milliseconds(250);
			sleep(d);

			let mut buf = Vec::with_capacity(2);
			socket.read(buf.as_mut_slice());
			let connack = decode(buf.as_slice());

			res = res.and_then(|_| socket.write(encode::pingreq().as_slice()));
			res = res.and_then(|_| socket.flush());

			sleep(d);

			res = res.and_then(|_| socket.write(encode::disconnect().as_slice()));
			res = res.and_then(|_| socket.flush());

			match res {
				Ok(_) => println!("success"),
				Err(e) => println!("Test failed: {}", e)
			};
		}
	}

}

