
use sysfs_gpio::{Direction, Pin, Edge};
use tokio::{run};
use enigo::*;
use tokio::prelude::*;

static keymap: [[Key; 3]; 4] = [
	[Key::Layout('1'), Key::Layout('2'), Key::Layout('3')],
	[Key::Layout('4'), Key::Layout('5'), Key::Layout('6')],
	[Key::Layout('7'), Key::Layout('8'), Key::Layout('9')],
	[Key::Layout('*'), Key::Layout('0'), Key::Layout('#')]
];
//enigo.key_down(key) key_click(key) key_up(key)

pub struct Keypad {
	row_pins: [Pin; 4],
	col_pins: [Pin; 3],
	enigo: Enigo,
}
impl Keypad {
	pub fn new(row_pins: [u32; 4], col_pins: [u32; 3]) -> Self {
		let row_fn = |i| { || -> Result<Pin, sysfs_gpio::Error> {
			let pin = Pin::new(i as u64);
			pin.export()?;
			pin.set_direction(Direction::In);
			pin.set_edge(Edge::BothEdges);
			Ok(pin)
		}().expect("Failed to initalize row pins")};
		let col_fn = |i| { || -> Result<Pin, sysfs_gpio::Error> {
			let pin = Pin::new(i as u64);
			pin.export()?;
			pin.set_direction(Direction::High);
			Ok(pin)
		}().expect("Failed to initalize column pins")};
		Keypad {
			col_pins: [col_fn(col_pins[0]), col_fn(col_pins[1]), col_fn(col_pins[2])],
			row_pins: [row_fn(row_pins[0]), row_fn(row_pins[1]), row_fn(row_pins[3]), row_fn(row_pins[4])],
			enigo: Enigo::new(),
		}
	}
	pub fn run(self) {
		let stream = self.row_pins.iter().map(|pin| {
			pin.get_value_stream(reactor.handle()).unwrap().for_each(move |val| {
				println!("Pin {} changed value to {}", pin.get_pin_num(), val);
				Ok(())
			}).map_err(|_| ())
		}).fold1(|l, r| { l.select(r) });
		tokio::run(stream);
	}
}
impl Drop for Keypad {
	fn drop(&mut self) {
		for pin in self.row_pins.iter().chain(self.col_pins.iter()) {
			pin.unexport().unwrap();
		}
	}
}
