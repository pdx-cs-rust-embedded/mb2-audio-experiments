#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m_rt::entry;
use microbit::Board;
use microbit::hal::{prelude::*, delay::Delay, gpio::Level};

/// Get a full cycle of `samples` of a sine wave, with
/// samples quantized to `q` *values* 0..`q`-1. Note that
/// `q` is *not* in units of bits.
fn sin(samples: &mut [u8], q: u8) {
    use core::f32::consts::PI;
    let step = 2.0 * PI / samples.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let v = libm::sinf(i as f32 * step);
        *s = libm::floorf((q - 1) as f32 * (v + 1.0) / 2.0) as u8;
    }
}

#[entry]
fn main() -> ! {
    let board = Board::take().unwrap();
    let mut delay = Delay::new(board.SYST);
    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);

    // Assume accurate quantization to 1us, and a desired
    // output sample rate of 16K samples/second, or 0.016
    // samples per us. This allows us 1/0.016 values per
    // sample, or 62.5. We will use 62 values per sample.
    // The size of our buffer for a 1000 Hz wave will be
    // 1/1000 sec, so 16 values.
    let mut samples = [0; 16];
    sin(&mut samples, 62);

    loop {
        for d in samples {
            speaker.set_high().unwrap();
            if d > 0 {
                delay.delay_us(d);
            }
            speaker.set_low().unwrap();
            if d < 61 {
                delay.delay_us(61 - d);
            }
        }
    }
}
