use std::time::Duration;
use std::thread;
use std::slice;
use std::os::unix::net::UnixStream;
use std::io;
use std::io::{Read, Write, ErrorKind};

use num_traits::{FromPrimitive};
use num_derive::{ToPrimitive, FromPrimitive};
use crossbeam::queue::MsQueue;

static TRIES: usize = 10;

fn handle_msg(msg: ClientMsg) -> Response {
	eprintln!("Received message: {:?}", msg);
	Response::ACK
}

//new approach:
//server sends send/receive byte
//this gives server a chance to send any events it has
//client responds with either watchdog or any event it has

pub fn handle_client(mut stream: &mut UnixStream, queue: &MsQueue<ServerMsg>) -> std::io::Result<()> {
	let mut i = 0;
	loop {
		if i%5 == 0 {
			queue.push(ServerMsg::VendFailed);
		}
		thread::sleep(Duration::new(1,0));
		let receiving = queue.is_empty();
		let rw = if receiving {RW::Read} else {RW::Write};

		stream.write_all(slice::from_mut(&mut (rw as u8)))?;
		if receiving {
			eprintln!("Receiving.");
			for i in 1.. { //loop for repeated recvs
				let resp = match recv(&mut stream) {
					Ok(msg) => handle_msg(msg),
					Err(e) => {
						if e.kind() == ErrorKind::InvalidData {
							Response::ReadFail
						} else {
							Err(e)? //should bust outta here
						}
					}
				};
				eprintln!("Returning ack: {:?}", resp);
				stream.write_all(slice::from_mut(&mut (resp as u8)))?;
				if resp != Response::ReadFail { break; }
				if i == TRIES {return Err(io::Error::new(ErrorKind::InvalidData, "Couldn't parse response"));}
			}
		} else {
			for i in 1.. { //loop for repeated sends
				let msg = queue.try_pop().unwrap();
				eprintln!("Sending {:?}", msg);
				let mut buf = [0;8];
				let mut size = 1;
				match msg {
					ServerMsg::MoneyAdded(n) => {
						buf[1] = n;
						size = 2;
					}
					_ => {}
				};
				buf[0] = ServerMsgType::from(msg) as u8;
				stream.write_all(&buf[..size])?;
				//now receive ack:
				let n = stream.read(&mut buf)?;
				if n == 0 {
					return Err(io::Error::new(ErrorKind::BrokenPipe, "Socket closed"));
				}
				let resp = Response::from_u8(buf[0]).ok_or_else(|| {
					io::Error::new(ErrorKind::InvalidData, "Bad acknowledgement")
				})?;
				eprintln!("Received ack: {:?}", resp);
				if resp != Response::ReadFail { break; }
				if i == TRIES {return Err(io::Error::new(ErrorKind::InvalidData, "Client failed to read data"));}
			}
		}
		i += 1;
	}
}

fn recv(stream: &mut UnixStream) -> io::Result<ClientMsg> {
	let mut buf = [0;16];
	let mut p = 0;
	loop { //read loop
		let n = stream.read(&mut buf[p..])?;
		if n == 0 {
			return Err(io::Error::new(ErrorKind::UnexpectedEof, "Read 0 bytes"));
		}
		p += n;
		if let Some(msg) = parse_client_msg(&buf, p)? {
			return Ok(msg);
		}
	}
}

fn parse_client_msg(buf: &[u8], n: usize) -> io::Result<Option<ClientMsg>> {
	let msg_type = ClientMsgType::from_u8(buf[0])
		.ok_or(io::Error::new(ErrorKind::InvalidData, "Bad message header"))?;
	let msg_size = match msg_type {
		ClientMsgType::Watchdog | ClientMsgType::Shutdown => 1,
		ClientMsgType::Vend => 2,
		ClientMsgType::ConfigChannel => 4,
		ClientMsgType::ConfigBehavior => 3,
	};
	if n < msg_size { return Ok(None); }
	Ok(Some(match msg_type {
		ClientMsgType::Watchdog => ClientMsg::Watchdog,
		ClientMsgType::Shutdown => ClientMsg::Shutdown,
		ClientMsgType::Vend => ClientMsg::Vend(buf[1]),
		ClientMsgType::ConfigChannel => ClientMsg::ConfigChannel(buf[1], buf[2], buf[3]),
		ClientMsgType::ConfigBehavior => ClientMsg::ConfigBehavior(buf[1], buf[2]),
	}))
}

#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq)]
enum ClientMsgType {
	Watchdog = 119,     //"w"
	Vend = 118,         //"v"
	ConfigChannel = 99, //"c"
	ConfigBehavior = 98,//"b"R
	Shutdown = 115      //"s"
}
#[derive(Clone, Debug)]
pub enum ClientMsg {
	Watchdog,
	Vend(u8),
	ConfigChannel(u8, u8, u8),
	ConfigBehavior(u8, u8),
	Shutdown
}
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq)]
enum ServerMsgType {
	VendSucceed = 115,//"s"
	VendFailed = 102, //"f"
	MoneyAdded = 109, //"m"
	Tampering = 116   //"t"
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum ServerMsg {
	VendSucceed,
	VendFailed,
	MoneyAdded(u8),
	Tampering
}
impl From<ServerMsg> for ServerMsgType {
	fn from(msg: ServerMsg) -> ServerMsgType {
		match msg {
			ServerMsg::VendSucceed => ServerMsgType::VendSucceed,
			ServerMsg::VendFailed => ServerMsgType::VendFailed,
			ServerMsg::MoneyAdded(_) => ServerMsgType::MoneyAdded,
			ServerMsg::Tampering => ServerMsgType::Tampering,
		}
	}
}
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq)]
enum Response {
	NACK = 21,
	ACK = 6,
	ReadFail = 33
}
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq)]
enum RW {
	Read = 60,
	Write = 62
}
