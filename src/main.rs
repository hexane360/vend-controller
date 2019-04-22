#![allow(dead_code)]
use std::time::Duration;
use std::fs;
use std::os::unix::net::UnixListener;
use std::thread;

use crossbeam::queue::MsQueue;
use sysfs_gpio::{Pin,Direction};
use sysfs_pwm::Pwm;
use generic_array::arr;

mod socket;
//mod gpio;
mod motor;
use socket::handle_client;

static SOCK_FILE: &str = "./test_socket";

//pin numbers are labeled "BCM" in 'gpio readall'
static ENABLE_PIN: u32 = 0; //physical pin 12, BCM 18
static RESET_PIN: u32 = 23;
static STEP_PIN: u32 = 24;
static SLEEP0_PIN: u32 = 25;

fn main() -> std::io::Result<()> {
	let queue = MsQueue::new();

	let SLEEP_PINS = arr![u32; 25, 8, 7, 1];
	let mut m = motor::DriverArray::new(ENABLE_PIN, STEP_PIN, RESET_PIN, arr![u32; SLEEP0_PIN]);
	m.sleep(0, false);
	for i in 255..0 {
		m.run(i);
		thread::sleep_ms(50);
	}
	m.dir_a(motor::Dir::CCW);
	for i in 255..0 {
		m.run(i);
		thread::sleep_ms(50);
	}
	m.stop();
	/*let enable_pin = Pin::new(ENABLE_PIN);
	enable_pin.export().expect("Failed to access GPIO pin");
	enable_pin.set_direction(Direction::Low);
	for _ in 0..30 {
		thread::sleep_ms(500);
		enable_pin.set_value(1).unwrap();
		thread::sleep_ms(500);
		enable_pin.set_value(0).unwrap();
	}
	enable_pin.unexport().expect("Failed to unexport");*/
	/*let enable_pin = Pwm::new(0, ENABLE_PIN).expect("Failed to create pin");
	enable_pin.export().expect("Failed to export");
	//enable_pin.enable(true).expect("Failed to enable");
	enable_pin.set_period_ns(5000).expect("Failed to set period");
	enable_pin.set_duty_cycle_ns(0);
	enable_pin.enable(true).expect("Failed to enable");
	for i in 0..5000 {
		enable_pin.set_duty_cycle_ns(i);
		thread::sleep_ms(50);
	}*/
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
