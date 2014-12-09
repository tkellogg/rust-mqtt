use std::io::{TcpStream, IoError};
use parser::{LastWill, Message, encode, decode};

#[deriving(Show, PartialEq, FromPrimitive)]
pub enum ConnectError { WrongProtocolVersion = 1, IdentifierRejected = 2, ServerUnavailable = 3,
												BadUserOrPassword = 4, NotAuthorized = 5, OtherError }

#[deriving(Show, PartialEq)]
pub enum MqttError { BrokenIO(IoError), NoConnection, MqttParseError, NoData, 
										 ConnectRefused(ConnectError), WrongMessage(Message) }

#[deriving(Default)]
pub struct ConnectOptions<'a> {
	pub host_port: &'a str,
	pub client_id: &'a str, 
	pub user: Option<&'a str>, 
	pub pass: Option<&'a str>, 
	pub keep_alive: u16, 
	pub clean: bool, 
	pub lwt: Option<&'a LastWill>
}

#[deriving(Default)]
pub struct Client<'a> {
	pub options: ConnectOptions<'a>,
	stream: Option<TcpStream>,
	last_id: u16
}

impl<'a> Client<'a> {
	pub fn connect(&'a mut self) -> Result<(), MqttError> {
		match TcpStream::connect(self.options.host_port) {
			Ok(stream) => {
				self.stream = Some(stream);

				let buf = encode::connect(self.options.client_id, 
																	self.options.user, 
																	self.options.pass, 
																	self.options.keep_alive, 
																	self.options.clean, 
																	self.options.lwt);

				self.stream.as_mut().map(|x| (*x).write(buf.as_slice()));

				match self.recv() {
					Ok(Message::Connack(0)) => Ok(()),
					Ok(Message::Connack(failure)) => {
						let codeOpt: Option<ConnectError> = FromPrimitive::from_u8(failure);
						let code = codeOpt.unwrap_or(ConnectError::OtherError);
						Err(MqttError::ConnectRefused(code))
					},
					Ok(other) => Err(MqttError::WrongMessage(other)),
					Err(e) => Err(e)
				}
			},
			Err(e) => Err(MqttError::BrokenIO(e))
		}
	}

	/// Read a message from TCP socket. This will `Err(NoData)` if there were no
	/// messages waiting to be read.
	pub fn recv(&mut self) -> Result<Message, MqttError> {
		match self.stream {
			Some(ref mut stream) => {

				let mut buf = [0, ..1024];

				match (*stream).read(&mut buf) {
					Ok(0) => Err(MqttError::NoData),
					Ok(length) => {
						let slice = buf.slice_to(length);
						match decode(slice) {
							Some(msg) => Ok(msg),
							None => Err(MqttError::MqttParseError)
						}
					},
					Err(e) => Err(MqttError::BrokenIO(e))
				}

			},
			None => Err(MqttError::NoConnection)
		}
	}
}

