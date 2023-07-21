#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m_rt::entry;
use microbit::Board;
use microbit::hal::{prelude::*, delay::Delay, gpio::Level};

/// Get a full cycle of `samples` of a sine wave, with
/// samples quantized to `q` - h *values* 0..`q`-h.
fn sin(samples: &mut [u8], q: u8, h: u8) {
    use core::f32::consts::PI;
    let step = 2.0 * PI / samples.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        // Get a value in the range 0.0..1.0.
        let v = (libm::sinf(i as f32 * step) + 1.0) / 2.0;
        // Save a value in the range 0..=q.
        *s = libm::floorf(((q - h) as f32 + 0.5) * v) as u8;
    }
}

#[entry]
fn main() -> ! {
    let board = Board::take().unwrap();
    let mut delay = Delay::new(board.SYST);
    let mut speaker = board.speaker_pin.into_push_pull_output(Level::Low);

    // Our desired output sample rate is 31250 samples/second.
    // This gives us a Nyquist limit of
    // 16KHz, which *should* be high enough to push the PWM
    // noise up where it's filtered by the speaker or at
    // least hard to hear.
    // 
    // We thus have 1/31250 = 32us / sample.  The size of
    // our buffer for a 1000 Hz wave will be 1/1000 sec, so
    // 31.25 samples. We will round to 31 and be a bit off
    // key. This will be swamped by the noise and
    // uncertainty of the switching and delay overheads.

    let mut samples = [0; 63];
    let ticks_per_sample = 31;
    let h = 25;
    sin(&mut samples, ticks_per_sample, h);

    loop {
        delay.delay_us(1000000u32);
        for _ in 0..1000 {
            for d in samples {
                if d > 0 {
                    speaker.set_high().unwrap();
                    delay.delay_us(d);
                }
                if d < ticks_per_sample {
                    speaker.set_low().unwrap();
                    delay.delay_us(ticks_per_sample - d);
                }
            }
        }
        for _ in 0..1000 {
            delay.delay_us(480u16);
            speaker.set_high().unwrap();
            delay.delay_us(480u16);
            speaker.set_low().unwrap();
        }
    }
}
