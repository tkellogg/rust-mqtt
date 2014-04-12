#![crate_id = "mqtt#0.10"]
#![desc = "An Message Queue Telemetry Transport (MQTT) client"]
#![crate_type = "lib"]
#![feature(struct_variant)]


pub mod mqtt {
	use std::slice;
	use std::slice::bytes::copy_memory;

	trait MqttHeader {
		fn asHeader(&self) -> u8;
	}
	
	#[deriving(FromPrimitive)] 
	pub enum QoS { 
		AtMostOnce = 0, 
		AtLeastOnce = 1, 
		ExactlyOnce = 2
	}

	impl MqttHeader for QoS {

		fn asHeader(&self) -> u8 {
			let i = *self as u8;
			i >> 5
		}
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

	impl MqttHeader for MessageType {
		fn asHeader(&self) -> u8 {
			*self as u8
		}
	}

	#[inline]
	fn parse_message_type(byte: u8) -> Option<MessageType> {
		let masked = byte & 0xf0;
		FromPrimitive::from_u8(masked)
	}

	fn parse_short(data: &[u8], index: uint) -> Option<u16> {
		let b1 = data.get(index).and_then(|x| x.to_u16());
		let b2 = data.get(index + 1).and_then(|x| x.to_u16());
		b1.and_then(|x| b2.map(|y| {
			(x << 8) | y
		}))
	}

	pub struct LastWillOptions {
		retain: bool,
		qos: u8,
		flag: bool,
		topic: ~str,
		msg: ~str
	}

	pub struct ConnectOptions {
		 clientId: ~str, 
		 user: Option<~str>, 
		 pw: Option<~str>, 
		 will: Option<LastWillOptions>,
		 clean: bool,
		 keepAlive: u16
	}

	pub enum Message {
		Connect(ConnectOptions),
		Connack ( u8 )
	}

	impl Message {
		pub fn encode(&self, result: |&[u8]| -> ()) {
			#[inline]
			fn opt_len(opt: &Option<~str>) -> uint {
				match *opt { 
					Some(ref s) => s.len() + 2,
					None => 0
				}
			}

			#[inline]
			fn lwt_len(lwt: &Option<LastWillOptions>) -> uint {
				match *lwt {
					Some(ref opt) => opt.topic.len() + opt.msg.len(),
					None => 0
				}
			}

			fn lwt_flags(lwt: &Option<LastWillOptions>) -> u8 {
				match *lwt {
					Some(ref opt) => 
						(opt.retain as u8) << 5 |
						(opt.qos & 0x03) << 3   |
						1 << 2,
					None => 0
				}
			}

			#[inline]
			fn msb(i: u16) -> u8 {
				((i & 0xff00) >> 8) as u8
			}

			#[inline]
			fn lsb(i: u16) -> u8 {
				(i & 0xff) as u8
			}

			#[inline]
			fn fshift<T>(opt: &Option<T>, by: uint) -> u8 {
				match opt.is_some() {
					true => 1 << by,
					false => 0
				}
			}

			match *self {
				Connect(ref opts) => {
					let id_len = opts.clientId.len() + 2;
					let lwt = lwt_len(&opts.will);
					let user_len = opt_len(&opts.user);
					let pw_len = opt_len(&opts.pw);
					let payload_len = 10 + id_len + lwt + user_len + pw_len;
					
					let flags = fshift(&opts.user, 7) |
											fshift(&opts.pw, 6)   |
											lwt_flags(&opts.will) |
											(opts.clean as u8) << 1;

					let mut buf = slice::with_capacity(payload_len + 12);
					copy_memory(buf, [0x10, 10, 0x00, 0x04, 0x4c, 0x51, 0x54, 0x54, 4, flags, msb(opts.keepAlive), lsb(opts.keepAlive)]);

					let client_len = opts.clientId.len() as u16;
					copy_memory(buf.mut_slice_from(12), [msb(client_len), lsb(client_len)]);
					copy_memory(buf.mut_slice_from(14), opts.clientId.as_bytes());

					let mut end = 14 + client_len as uint;
					for will in opts.will.iter() {
						let topic_len = will.topic.len() as u16;
						copy_memory(buf.mut_slice_from(end), [msb(topic_len), lsb(topic_len)]);
						copy_memory(buf.mut_slice_from(end + 2), will.topic.as_bytes());
						end += will.topic.len() + 2;

						let msg_len = will.msg.len() as u16;
						copy_memory(buf.mut_slice_from(end), [msb(msg_len), lsb(msg_len)]);
						copy_memory(buf.mut_slice_from(end + 2), will.msg.as_bytes());
						end += will.msg.len() + 2;
					}

					for user in opts.user.iter() {
						let len = user.len() as u16;
						copy_memory(buf.mut_slice_from(end), [msb(len), lsb(len)]);
						copy_memory(buf.mut_slice_from(end + 2), user.as_bytes());
						end += user.len() + 2;
					}

					for pw in opts.pw.iter() {
						let len = pw.len() as u16;
						copy_memory(buf.mut_slice_from(end), [msb(len), lsb(len)]);
						copy_memory(buf.mut_slice_from(end + 2), pw.as_bytes());
						end += pw.len() + 2;
					}

					result(buf)
				}
				Connack(_) => ()
			}
		}
	}

	pub fn decode(data: &[u8]) -> Option<Message> {
		let hd = data.head();
		let msg = hd.and_then(|x| parse_message_type(*x));
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
			PINGREQ=> None,
			PINGRESP=> None,
			DISCONNECT=> None
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
		use mqtt::ConnectOptions;

		#[test]
		fn example() {
			let con = ConnectOptions {
				clientId: ~"tim-rust",
				user: None,
				pw: None,
				will: None,
				clean: false,
				keepAlive: 60
			};
		}
	}

}

