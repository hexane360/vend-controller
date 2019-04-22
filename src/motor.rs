use std::thread;
use std::time::Duration;
use sysfs_gpio::{Pin,Direction};
use sysfs_pwm::Pwm;
use generic_array::{GenericArray,ArrayLength, functional::FunctionalSequence};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dir {
	CCW = -1,
	Stop = 0,
	CW = 1
}
impl Dir {
	pub fn stopped(&self) -> bool {
		*self == Dir::Stop
	}
}
/* STEP MAP: (+ is CW) (A is near supply, +s are on outside)
step:   A:   B:
   0:  100    0
   1:   71   71  START POS
   2:    0  100
   3:  -71   71
   4: -100    0
   5:  -71  -71
   6:    0 -100
   7:   71  -71
*/
const PERIOD_MULTIPLIER: u8 = 30;

/*
struct to run an array of many DRV8825 drivers simultaneously.

*/
pub struct DriverArray<N: ArrayLength<Driver>> {
	speed: u8,
	dir_a: Dir,
	dir_b: Dir,
	step: u8,
	//slept: bool,
	enable_pin: Pwm, //low to enable
	step_pin: Pin,
	reset_pin: Pin, //low to reset
    drivers: GenericArray<Driver, N>
}
impl<N: ArrayLength<Driver> + ArrayLength<Pin> + ArrayLength<bool> + ArrayLength<u32> + ArrayLength<()>> DriverArray<N> {
	pub fn new(enable_pin: u32, step_pin: u32, reset_pin: u32, sleep_pins: GenericArray<u32, N>) -> Self {
		let drivers = sleep_pins.map(|pin| {
			Driver {
				slept: true,
				sleep_pin: Pin::new(pin as u64)
			}
		});
		let s = DriverArray {
			speed: 255,
			step: 1,
			dir_a: Dir::CW,
			dir_b: Dir::CW,
			enable_pin: Pwm::new(1, enable_pin).expect("Failed to access PWM gpio"),
			step_pin: Pin::new(step_pin as u64),
			reset_pin: Pin::new(reset_pin as u64),
			drivers: drivers,
		};
		//initalize pins
		|| -> Result<(), sysfs_pwm::Error> {
			s.enable_pin.export()?;
			s.enable_pin.enable(true)?;
			s.enable_pin.set_period_ns(255*PERIOD_MULTIPLIER as u32)?;
			s.set_speed(0);
			Ok(())
		}().expect("Failed to access PWM GPIO");

		|| -> Result<(), sysfs_gpio::Error> {
			s.step_pin.export()?;
			s.reset_pin.export()?;
			s.step_pin.set_direction(Direction::Low)?;
			s.reset_pin.set_direction(Direction::Low)?;
			Ok(())
		}().expect("Failed to access GPIO");
		for driver in &s.drivers {
			driver.sleep_pin.set_direction(Direction::Low).unwrap(); //high to wake
		}
		s
	}
	pub fn reset(&mut self) { //reset drivers and update state
		self.reset_pin.set_value(0).unwrap();
		thread::sleep(Duration::new(0, 50_000));
		self.reset_pin.set_value(1).unwrap();
		self.dir_a = Dir::CW;
		self.dir_b = Dir::CW;
		self.step = 1;
	}
	pub fn restep(&mut self) { //reset and resend step state
		self.reset_pin.set_value(0).unwrap();
		thread::sleep(Duration::new(0, 50_000));
		self.reset_pin.set_value(1).unwrap();
		let step = self.step;
		self.step = 1;
		self.step_to(step);
	}
	pub fn run(&mut self, speed: u8) {
		self.speed = speed;
		if self.dir_a != Dir::Stop || self.dir_b != Dir::Stop {
			self.set_speed(speed);
		}
	}
	pub fn stop(&mut self) {
		self.run(0);
	}
	pub fn sleep(&mut self, driver: usize, sleep: bool) {
		if self.drivers[driver].slept == sleep {return;}
		self.drivers[driver].slept = sleep;
		thread::sleep(Duration::new(0,1_700_000));
		self.restep();
	}
	pub fn sleep_arr(&mut self, sleep_arr: GenericArray<bool, N>) {
		let mut changed = false;
		sleep_arr.zip(&mut self.drivers, |slept, driver| {
			if driver.slept != slept {
				driver.slept = slept;
				changed = true;
			}
		});
		if changed { //re-step to setting
			thread::sleep(Duration::new(0,1_700_000));
			self.restep();
		}
	}
	pub fn dir_a(&mut self, dir: Dir) {
		self.dir(dir, self.dir_b);
	}
	pub fn dir_b(&mut self, dir: Dir) {
		self.dir(self.dir_a, dir);
	}
	pub fn dir(&mut self, dir_a: Dir, dir_b: Dir) {
		self.dir_a = dir_a;
		self.dir_b = dir_b;
		self.step_to(match dir_b {
			Dir::Stop => match dir_a {
				Dir::Stop => {
					self.set_speed(0);
					return;
				},
				Dir::CW => 0,
				Dir::CCW => 4
			},
			Dir::CW => 2 - dir_a as u8,
			Dir::CCW => 6 + dir_a as u8,
		});
	}
	fn step_to(&mut self, end: u8) {
		self.step(end - self.step + if end < self.step {8} else {0});
	}
	fn step(&mut self, num: u8) {
		for _ in 0..num {
			self.step_pin.set_value(0).unwrap();
			thread::sleep(Duration::new(0, 20000));
			self.step_pin.set_value(1).unwrap();
			thread::sleep(Duration::new(0, 20000));
		}
		self.step = (self.step + num)%8;
	}
	fn set_speed(&self, speed: u8) {
		self.enable_pin.set_duty_cycle_ns(((255-speed)*PERIOD_MULTIPLIER) as u32).unwrap();
	}
}
impl<N: ArrayLength<Driver>> Drop for DriverArray<N> {
	fn drop(&mut self) {
		self.enable_pin.unexport().unwrap();
		self.step_pin.unexport().unwrap();
		self.reset_pin.unexport().unwrap();
	}
}

pub struct Driver { //represents a single motor driver
	slept: bool,
	sleep_pin: Pin
}
impl Driver {
	pub fn new(sleep_pin: Pin) -> Self {
		Driver {
			slept: false,
			sleep_pin: sleep_pin
		}
	}
}
impl Drop for Driver {
	fn drop(&mut self) {
		self.sleep_pin.unexport().unwrap();
	}
}
