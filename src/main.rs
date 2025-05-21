#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m_rt::entry;
use embedded_hal::{delay::DelayNs, digital::InputPin, digital::OutputPin};
use microbit::Board;
use microbit::hal::{delay::Delay, gpio::Level};

#[entry]
fn main() -> ! {
    let board = Board::take().unwrap();
    let mut delay = Delay::new(board.SYST);
    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);
    let mut button = board.buttons.button_a;
    loop {
        if button.is_low().unwrap() {
            speaker.set_high().unwrap();
            delay.delay_us(500);
            speaker.set_low().unwrap();
            delay.delay_us(500);
        }
    }
}
