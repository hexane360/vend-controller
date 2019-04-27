use sysfs_gpio::{Direction, Pin, Edge};
use enigo::*;
use tokio;
use futures::stream::Stream;
use futures::Future;
use itertools::Itertools;

static KEYMAP: [[Key; 3]; 4] = [
	[Key::Layout('1'), Key::Layout('2'), Key::Layout('3')],
	[Key::Layout('4'), Key::Layout('5'), Key::Layout('6')],
	[Key::Layout('7'), Key::Layout('8'), Key::Layout('9')],
	[Key::Layout('*'), Key::Layout('0'), Key::Layout('#')]
];
//enigo.key_down(key) key_click(key) key_up(key)
fn select<T, U, V: Stream<Item = T, Error = U>>(s1: V, s2: V) -> impl Stream<Item = T, Error = U> {
	s1.select(s2)
}

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
			pin.set_direction(Direction::In)?;
			pin.set_edge(Edge::RisingEdge)?;
			Ok(pin)
		}().expect("Failed to initalize row pins")};
		let col_fn = |i| { || -> Result<Pin, sysfs_gpio::Error> {
			let pin = Pin::new(i as u64);
			pin.export()?;
			pin.set_direction(Direction::High)?;
			Ok(pin)
		}().expect("Failed to initalize column pins")};
		Keypad {
			col_pins: [col_fn(col_pins[0]), col_fn(col_pins[1]), col_fn(col_pins[2])],
			row_pins: [row_fn(row_pins[0]), row_fn(row_pins[1]), row_fn(row_pins[2]), row_fn(row_pins[3])],
			enigo: Enigo::new(),
		}
	}
	pub fn run(self) {
		let stream = self.row_pins[0].get_stream().unwrap()
             .select(self.row_pins[1].get_stream().unwrap())
             .select(self.row_pins[2].get_stream().unwrap())
			 .select(self.row_pins[3].get_stream().unwrap());

		tokio::run(stream.for_each(|()| {
			println!("Rising edge on pin");
			Ok(())
		}).map_err(|_| ()));
	}
}
impl Drop for Keypad {
	fn drop(&mut self) {
		for pin in self.row_pins.iter().chain(self.col_pins.iter()) {
			pin.unexport().unwrap();
		}
	}
}
