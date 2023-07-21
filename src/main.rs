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

    loop {
        for v in [30u16, 50, 150, 300, 400] {
            let mut rng = fastrand::Rng::with_seed(0);
            let dither: [bool; 500] = core::array::from_fn(|_| {
                rng.f32() * 500.0 < v as f32
            });
            for _ in 0..500 {
                for b in dither {
                    if b {
                        speaker.set_high().unwrap();
                    } else {
                        speaker.set_low().unwrap();
                    }
                    delay.delay_us(1u8);
                }
                for b in dither {
                    if !b {
                        speaker.set_high().unwrap();
                    } else {
                        speaker.set_low().unwrap();
                    }
                    delay.delay_us(1u8);
                }
            }
        }
    }
}
