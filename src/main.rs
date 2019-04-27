#![allow(dead_code)]
use std::time::Duration;
use std::fs;
use std::os::unix::net::UnixListener;
use std::thread;

use crossbeam::queue::MsQueue;
use generic_array::arr;

mod socket;
//mod gpio;
mod keypad;
mod motor;
use socket::handle_client;

static SOCK_FILE: &str = "./test_socket";

//pin numbers are labeled "BCM" in 'gpio readall'
static ENABLE_PIN: u32 = 0; //physical pin 12, BCM 18
static RESET_PIN: u32 = 23;
static STEP_PIN: u32 = 24;
static SLEEP0_PIN: u32 = 25;

static COL_PINS: [u32; 3] = [10, 9, 11];
static ROW_PINS: [u32; 4] = [0, 5, 6, 13];

fn main() -> std::io::Result<()> {
	let queue = MsQueue::new();

	let keypad = keypad::Keypad::new(ROW_PINS, COL_PINS);
	keypad.run();
	println!("After run()");
	//thread::spawn(|| { keypad.run(); });

	let sleep_pins = arr![u32; 25, 8, 7]; // 1
	let mut m = motor::DriverArray::new(ENABLE_PIN, STEP_PIN, RESET_PIN, sleep_pins);
	println!("initialized");
	thread::sleep(Duration::new(0,500_000_000));
	m.run(255);
	for i in 0..sleep_pins.len() {
		m.sleep(i, false);
		m.dir(motor::Dir::CW, motor::Dir::Stop);
		thread::sleep(Duration::new(2,0));
		m.dir(motor::Dir::Stop, motor::Dir::CW);
		thread::sleep(Duration::new(2,0));
		m.sleep(i, true);
	}
	println!("stopping!");
	m.stop();
	return Ok(());

	fs::remove_file(SOCK_FILE)?;
	let listener = UnixListener::bind(SOCK_FILE).expect("Couldn't bind socket");
	for stream in listener.incoming() {
		if let Err(e) = stream {
			eprintln!("Failed to connect with error: {}", e);
			eprintln!("  kind: {:?}", e.kind());
			if let Some(e) = e.get_ref() {
				eprintln!("  inner error: {:?}", e);
			}
			//restart client here
			continue;
		}
		let mut stream = stream.unwrap();
		stream.set_read_timeout(Some(Duration::new(5,0))).expect("Couldn't set read timeout");
		stream.set_write_timeout(Some(Duration::new(5,0))).expect("Couldn't set write timeout");
		match handle_client(&mut stream, &queue) {
			Ok(()) => eprintln!("Client exiting normally"),
			Err(e) => {
				eprintln!("Client exiting with error: {}", e);
				eprintln!("  kind: {:?}", e.kind());
				if let Some(e) = e.get_ref() {
					eprintln!("  inner error: {:?}", e);
				}
			},
		}
		stream.shutdown(std::net::Shutdown::Both).expect("Failed to shutdown socket");
	}
	fs::remove_file(SOCK_FILE)?;
	Ok(())
}
