use std::default::Default;
use num::FromPrimitive;

enum_from_primitive! {
	#[derive(PartialEq, Debug, Clone)]
	pub enum QoS { 
		AtMostOnce = 0, 
		AtLeastOnce = 1, 
		ExactlyOnce = 2
	}
}

impl Default for QoS {
	fn default() -> QoS { QoS::AtMostOnce }
}

enum_from_primitive! {
	#[derive(Debug)]
	#[derive(PartialEq)]
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
}

#[derive(Debug)]
pub struct LastWill {
	topic: Box<str>,
	msg: Box<str>,
	qos: QoS,
	retain: bool
}

/// Messages that can be parsed by `decode`.
#[derive(PartialEq, Debug)]
pub enum Message {
	Connack(u8),
	SubAck(Box<Vec<SubAckCode>>),
	UnsubAck,
	PingReq,
	PingResp,
	Disconnect
}

#[derive(PartialEq, Debug)]
pub enum SubAckCode { SubAckSuccess(QoS), SubAckFailure }

fn parse_short(data: &[u8], index: usize) -> Option<u16> {
	let b1 = data.get(index).map(|x| *x as u16);
	let b2 = data.get(index + 1).map(|x| *x as u16);
	b1.and_then(|x| b2.map(|y| {
		(x << 8) | y
	}))
}

fn parse_rlen(data: &[u8], index: usize) -> (usize, usize) {
	match data.get(index) {
		Some(&a) if a < 128 => (index + 1, a as usize),
		Some(&a) => match data.get(index + 1) {
			Some(&b) if b < 128 => (index + 2, ((a as usize) << 8) | (b as usize)),
			Some(&b) => match data.get(index + 2) {
				Some(&c) if b < 128 => (index + 3, ((a as usize) << 15) | ((b as usize) << 8) | (c as usize)),
				Some(&c) => match data.get(index + 3) {
					Some(&d) => (index + 4, ((a as usize) << 22) | ((b as usize) << 15) | ((c as usize) << 8) | (d as usize)),
					None => (index, 0)
				},
				None => (index, 0)
			},
			None => (index, 0)
		},
		None => (index, 0)
	}
}

pub mod encode {
	use parser::{LastWill, QoS, MessageType};

	#[inline]
	fn msb(i: u16) -> u8 {
		((i & 0xff00) >> 8) as u8
	}

	#[inline]
	fn lsb(i: u16) -> u8 {
		(i & 0xff) as u8
	}

	#[inline]
	fn fshift<T>(opt: Option<&T>, by: usize) -> u8 {
		opt.map_or(0, |_| 1 << by)
	}

	fn rlen_size(remaining_len: usize) -> usize {
		match remaining_len {
			x if x < 128 => 1,
			x if x < 16384 => 2,
			x if x < 2097152 => 3,
			_ => 4
		}
	}

	fn rlen(buf: &mut Vec<u8>, remaining_length: u32) {
		fn nth_mask(n: u8, rl: u32) -> u8 {
			let shift = (((n as usize) - 1) * 7) + 8;
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
		fn lwt_len(lwt: Option<&LastWill>) -> usize {
			lwt.map_or(0, |opt| { opt.topic.len() + opt.msg.len() })
		}

		fn lwt_flags(lwt: Option<&LastWill>) -> u8 {
			lwt.map_or(0, |lw| { (lw.retain as u8) << 5 | ((lw.qos.clone() as u8) & 0x03) << 3 | 1 << 2 })
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
		let bytes = &[0x10, remaining_len as u8, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 4, flags, msb(keep_alive), lsb(keep_alive)];
		buf.extend(bytes.iter().cloned());

		let client_len = client_id.len() as u16;
		buf.push(msb(client_len));
		buf.push(lsb(client_len));
		buf.extend(client_id.as_bytes().iter().cloned());

		for will in lwt.iter() {
			let topic_len = will.topic.len() as u16;
			buf.push(msb(topic_len));
			buf.push(lsb(topic_len));
			buf.extend(will.topic.as_bytes().iter().cloned());

			let msg_len = will.msg.len() as u16;
			buf.push(msb(msg_len));
			buf.push(lsb(msg_len));
			buf.extend(will.msg.as_bytes().iter().cloned());
		}

		for user in user.iter() {
			let len = user.len() as u16;
			buf.push(msb(len));
			buf.push(lsb(len));
			buf.extend(user.as_bytes().iter().cloned());
		}

		for pw in pass.iter() {
			let len = pw.len() as u16;
			buf.push(msb(len));
			buf.push(lsb(len));
			buf.extend(pw.as_bytes().iter().cloned());
		}

		buf
	}

	pub fn publish(topic: &str, msg: &str, qos: QoS, retain: bool, dup: bool, id: Option<u16>) -> Vec<u8> {

		let pid_len = id.map_or(0, |_| 2);
		let remaining_len = 2 + topic.len() + msg.len() + pid_len;
		let buf_len = remaining_len + 1 + rlen_size(remaining_len) as usize;
		let mut buf: Vec<u8> = Vec::with_capacity(buf_len);

		let fixed_header = 0x30 | ((dup as u8) << 3) | ((qos as u8) << 1) | (retain as u8);
		buf.push(fixed_header);

		rlen(&mut buf, remaining_len as u32);

		let topic_len = topic.len() as u16;
		buf.push(msb(topic_len));
		buf.push(lsb(topic_len));
		buf.extend(topic.as_bytes().iter().cloned());

		for v in id.iter() {
			buf.push(msb(*v));
			buf.push(lsb(*v));
		}

		buf.extend(msg.as_bytes().iter().cloned());

		buf
	}

	pub fn subscribe(subs: Vec<(&str, QoS)>, msg_id: u16) -> Vec<u8> {
		fn remaining_len(subs: &[(&str, QoS)]) -> usize {
			subs.iter().fold(2, |acc, tuple| acc + 3 + tuple.0.len())
		}

		let rlength = remaining_len(&subs);
		let buf_len = 1 + rlen_size(rlength) + rlength;
		let mut buf: Vec<u8> = Vec::with_capacity(buf_len);

		let fixed: u8 = ((MessageType::SUBSCRIBE as u8) << 4) | 2;
		buf.push(fixed);

		rlen(&mut buf, rlength as u32);

		// packet ID
		buf.push(msb(msg_id));
		buf.push(lsb(msg_id));

		for &(topic, ref qos) in subs.iter() {
			let topic_len = topic.len() as u16;
			buf.push(msb(topic_len));
			buf.push(lsb(topic_len));
			buf.extend(topic.as_bytes().iter().cloned());

			buf.push(qos.clone() as u8);
		}

		buf
	}

	pub fn unsubscribe(topics: Vec<&str>, msg_id: u16) -> Vec<u8> {
		let rlength = topics.iter().fold(2, |acc, topic| acc + 2 + topic.len());
		let buf_len = 1 + rlen_size(rlength) + rlength;
		let mut buf: Vec<u8> = Vec::with_capacity(buf_len);

		let fixed: u8 = ((MessageType::UNSUBSCRIBE as u8) << 4) | 2;
		buf.push(fixed);

		rlen(&mut buf, rlength as u32);

		// packet ID
		buf.push(msb(msg_id));
		buf.push(lsb(msg_id));

		for topic in topics.iter() {
			let topic_len = topic.len() as u16;
			buf.push(msb(topic_len));
			buf.push(lsb(topic_len));
			buf.extend(topic.as_bytes().iter().cloned());
		}

		buf
	}

	pub fn pingreq() -> Vec<u8> {
		use parser::MessageType::PINGREQ;
		let b: u8 = (PINGREQ as u8) << 4;
		vec!(b, 0 as u8)
	}

	pub fn disconnect() -> Vec<u8> {
		use parser::MessageType::DISCONNECT;
		let b: u8 = (DISCONNECT as u8) << 4;
		vec!(b, 0 as u8)
	}
}

pub fn decode(data: &[u8]) -> Option<Message> {
	use parser::Message;
	use parser::MessageType::*;

	let hd = data.get(0);
	let msg: Option<MessageType> = hd.and_then(|x| MessageType::from_u8(*x >> 4));
	println!("Decoding a message: {} bytes, data[0] ({:?}), msg ({:?})", data.len(), hd, msg);
	msg.and_then(|x| match x {
		CONNECT => None,
		CONNACK => parse_connack(data),
		PUBLISH => None,
		PUBACK => None,
		PUBREC => None,
		PUBREL => None,
		PUBCOMP => None,
		SUBSCRIBE => None,
		SUBACK => parse_suback(data),
		UNSUBSCRIBE => None,
		UNSUBACK => parse_unsuback(data),
        PINGREQ => Some(Message::PingReq),
		PINGRESP => Some(Message::PingResp),
		DISCONNECT => Some(Message::Disconnect)
	})
}

fn parse_connack(data: &[u8]) -> Option<Message> {
	use parser::Message;

	let (index, remaining_length) = parse_rlen(data, 1);
	let ret_code = match remaining_length {
		2 => data.get(index),
		_ => None
	};
	ret_code.and_then(|r| Some(Message::Connack(*r)))
}

fn parse_suback(data: &[u8]) -> Option<Message> {
	let (index, remaining_length) = parse_rlen(data, 1);
	parse_short(data, index); // msg_id
	let rlen = remaining_length + index;
	
	let mut i = index + 2;
	let mut codes: Box<Vec<SubAckCode>> = Box::new(Vec::with_capacity(remaining_length));
	while i < rlen {
		let ret_code = data.get(i).map_or(SubAckCode::SubAckFailure, |c| {
			let qos = QoS::from_u8(*c);
			SubAckCode::SubAckSuccess(qos.unwrap_or(QoS::AtMostOnce))
		});
		codes.push(ret_code);
		i += 1;
	}
	Some(Message::SubAck(codes))
}

fn parse_unsuback(data: &[u8]) -> Option<Message> {
	let (index, _) = parse_rlen(data, 1);
	let msg_id_opt = parse_short(data, index);
	msg_id_opt.map(|_| Message::UnsubAck)
}


