#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m_rt::entry;
use microbit::Board;
use microbit::hal::{prelude::*, delay::Delay, gpio::Level};

#[entry]
fn main() -> ! {
    let board = Board::take().unwrap();
    let mut delay = Delay::new(board.SYST);
    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);

    let mut cycle = |count, duty| {
        for i in 0..count {
            if i % duty == 0 {
                speaker.set_high().unwrap();
            } else {
                speaker.set_low().unwrap();
            }
        }
        for i in 0..count {
            speaker.set_low().unwrap();
        }
    };

    loop {
        for i in 0..10000 {
            cycle(10, 1);
        }
        for i in 0..10000 {
            cycle(10, 2);
        }
    }
}
