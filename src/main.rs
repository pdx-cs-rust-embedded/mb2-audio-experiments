#![no_main]
#![no_std]

/// This uses code from the microbit crate speaker-v2 demo.
///
/// This demo plays 8-bit audio — a roughly 1043Hz C6 —
/// out the speaker via high-frequency hardware PWM.
///
/// The chipping rate for the PWM (and thus the "sample
/// rate" for the audio) is 62.5K samples/second. This means
/// that the chipping noise is at the Nyquist frequency of
/// 31.25KHz, which might be beyond what the speaker can do
/// (it probably isn't) and should be above the likely human
/// hearing range.
///
/// If we wanted a higher sample rate, we'd have to cut down
/// the bits-per-sample: in the limiting case we'd end up
/// with one bit per sample sigma-delta at 16MHz, which
/// might actually be better but without modeling the output
/// filtering would more likely be garbage.
///
/// If the Cargo feature `external_out` is enabled, this
/// code will output to P0 on the MB2 edge connector instead
/// of the speaker.

use panic_rtt_target as _;

use cortex_m::asm;
use cortex_m_rt::entry;
use microbit::hal::{gpio, pwm};
use microbit::Board;
use rtt_target::rtt_init_print;

/// Fill `samples` with `n` full cycles of samples of a sine
/// wave, with samples quantized to `q` *values* 0..`q`-1.
fn sine(samples: &mut [u16], q: u16, n: usize) {
    use core::f32::consts::PI;
    let step = 2.0 * PI * n as f32 / samples.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        // Get the next value.
        let v = libm::sinf(i as f32 * step);
        // Normalize to the range 0.0..=q-1.
        let v = (q - 1) as f32 * (v + 1.0) / 2.0;
        // Save a value in the range 0..=q-1.
        *s = libm::floorf(v + 0.5) as u16;
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();

    // Set up the speaker GPIO pin as an output.
    #[cfg(not(feature = "external_out"))]
    let speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::Low);
    #[cfg(feature = "external_out")]
    // Send output to edge connector P0 instead of speaker pin.
    let speaker_pin = board.pins.p0_02.into_push_pull_output(gpio::Level::Low);

    // Use the PWM peripheral to generate a waveform for the speaker
    // The base counter rate for the PWM is 16MHz.
    // https://jimmywongiot.com/2021/06/01/advanced-pulse-width-modulation-pwm-on-nordic-nrf52-series/
    // This lets us run at 62500 sps with 256 ticks per sample, since 62500 * 256 = 16M.
    // We can thus run 8-bit samples at this rate.
    let speaker = pwm::Pwm::new(board.PWM0);
    speaker
        // output the waveform on the speaker pin
        .set_output_pin(pwm::Channel::C0, speaker_pin.degrade())
        // Prescaler set for 16MHz.
        .set_prescaler(pwm::Prescaler::Div1)
        // Configure for up counter mode.
        .set_counter_mode(pwm::CounterMode::Up)
        // Read duty cycle values from sequence.
        .set_load_mode(pwm::LoadMode::Common)
        // Be sure to be advancing the thing.
        .set_step_mode(pwm::StepMode::Auto)
        // Set maximum duty cycle = PWM period in ticks.
        .set_max_duty(256)
        // Set no delay between samples.
        .set_seq_refresh(pwm::Seq::Seq0, 0)
        // Set no delay at end of sequence.
        .set_seq_end_delay(pwm::Seq::Seq0, 0)
        // Set no delay between samples.
        .set_seq_refresh(pwm::Seq::Seq1, 0)
        // Set no delay at end of sequence.
        .set_seq_end_delay(pwm::Seq::Seq1, 0)
        // Enable sample channel.
        .enable_channel(pwm::Channel::C0)
        // Enable sample group.
        .enable_group(pwm::Group::G0)
        // Keep playing forever.
        .loop_inf()
        // Enable PWM.
        .enable();

    // The `unsafe`s here are to assure the Rust compiler
    // that nothing else is going to mess with this buffer
    // while a mutable reference is held.
    //
    // Safety: Because we are single-threaded, the only
    // thing that can access `SAMPS` once created is the HW
    // PWM unit, and it will be read-only access.

    // Set up the sine wave. This has to be in RAM for the
    // PWM unit to access it. It needs to be a 16-bit buffer
    // even though we will have only 8-bit (ish) sample
    // resolution.
    static mut SAMPS: [u16; 240] = [0; 240];
    unsafe {
        // We generate the sine wave with a little
        // "headroom": want all values between 1 and 254 to
        // make sure the HW PWM doesn't get lost and does
        // output some energy on every cycle.
        sine(&mut SAMPS, 256, 4);
        for s in &mut SAMPS {
            // The default counter mode is to set low up to
            // the count, then set high until the end of the
            // cycle. Setting the high bit in the count
            // register inverts this (and is otherwise
            // ignored), giving a "right-side-up" sine wave.
            //
            // We could instead complement all the sine
            // values, but either way works.
            *s |= 0x8000;
        }
    };

    // Start the sine wave.
    let _pwm_seq = unsafe { speaker.load(Some(&SAMPS), Some(&SAMPS), true).unwrap() };

    loop {
        asm::wfi();
    }
}
