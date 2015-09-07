use std::net::TcpStream;
use std::io::{Error, Write, Read};
use parser::{LastWill, Message, encode, decode, QoS};
use num::FromPrimitive;

enum_from_primitive! {
	#[derive(Debug, PartialEq)]
	pub enum ConnectError { WrongProtocolVersion = 1, IdentifierRejected = 2, ServerUnavailable = 3,
													BadUserOrPassword = 4, NotAuthorized = 5, OtherError }
}

#[derive(Debug)]
pub enum MqttError { BrokenIO(Error), NoConnection, MqttParseError(&'static str), NoData, 
										 ConnectRefused(ConnectError), WrongMessage(Message) }

#[derive(Default)]
pub struct ConnectOptions<'a> {
	pub host_port: &'a str,
	pub client_id: &'a str, 
	pub user: Option<&'a str>, 
	pub pass: Option<&'a str>, 
	pub keep_alive: u16, 
	pub clean: bool, 
	pub lwt: Option<&'a LastWill>
}

#[derive(Default)]
pub struct Client<'a> {
	pub options: ConnectOptions<'a>,
	pub stream: Option<TcpStream>,
	pub last_id: u16
}

impl<'a> Client<'a> {

	pub fn is_connected(&self) -> bool { self.stream.is_some() }

	fn next_id(&mut self) -> u16 {
		self.last_id += 1;
		self.last_id
	}

	/// Synchronously connect to the MQTT broker using the options set during creation of
	/// the Client struct.
	pub fn connect(&mut self) -> Result<(), MqttError> {
		match TcpStream::connect(self.options.host_port) {
			Ok(stream) => {
				self.stream = Some(stream);

				let buf = encode::connect(self.options.client_id, 
																	self.options.user, 
																	self.options.pass, 
																	self.options.keep_alive, 
																	self.options.clean, 
																	self.options.lwt);

				// TODO: check this result
				let _ = self.write(&buf);

				match self.recv() {
					Ok(Message::Connack(0)) => Ok(()),
					Ok(Message::Connack(failure)) => {
						let code_opt: Option<ConnectError> = ConnectError::from_u8(failure);
						let code = code_opt.unwrap_or(ConnectError::OtherError);
						Err(MqttError::ConnectRefused(code))
					},
					Ok(other) => Err(MqttError::WrongMessage(other)),
					Err(e) => Err(e)
				}
			},
			Err(e) => Err(MqttError::BrokenIO(e))
		}
	}

	/// Cleanly end the MQTT session. Always do this unless you unexpectedly crash and
	/// need to receive missed QoS > 0 messages.
	pub fn disconnect(&'a mut self) -> Result<usize, MqttError> {
		let buf = encode::disconnect();
		self.write(&buf)
	}

	fn write(&mut self, buf: &[u8]) -> Result<usize, MqttError> {
		match self.stream.as_mut().map(|x| (*x).write(buf as &[u8])) {
			Some(Err(e)) => Err(MqttError::BrokenIO(e)),
			Some(Ok(res)) => Ok(res),
			None => Err(MqttError::NoConnection)
		}
	}

	/// Read a message from TCP socket. This will `Err(NoData)` if there were no
	/// messages waiting to be read.
	pub fn recv(&mut self) -> Result<Message, MqttError> {
		match self.stream {
			Some(ref mut stream) => {

				let mut buf = [0; 1024];

				match (*stream).read(&mut buf) {
					Ok(0) => Err(MqttError::NoData),
					Ok(length) => {
						let (slice, _) = buf.split_at(length);
						match decode(slice) {
							Some(msg) => Ok(msg),
							None => Err(MqttError::MqttParseError("Message unrecognized"))
						}
					},
					Err(e) => Err(MqttError::BrokenIO(e))
				}

			},
			None => Err(MqttError::NoConnection)
		}
	}

	pub fn publish(&mut self, topic: &str, msg: &str, qos: QoS, retained: bool, dup: bool) -> Result<usize, MqttError> {
		let id = match qos {
			QoS::AtMostOnce => None,
			_ => Some(self.next_id())
		};

		let buf = encode::publish(topic, msg, qos, retained, dup, id);

		self.write(&buf)
	}

	pub fn subscribe(&mut self, subscriptions: Vec<(&str, QoS)>) -> Result<usize, MqttError> { 
		let id = self.next_id();
		let buf = encode::subscribe(subscriptions, id);
		self.write(&buf)
	}

	pub fn unsubscribe(&mut self, topics: Vec<&str>) -> Result<usize, MqttError> {
		let id = self.next_id();
		let buf = encode::unsubscribe(topics, id);
		self.write(&buf)
	}
}

