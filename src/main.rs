#![no_main]
#![no_std]

/// This uses code from the microbit crate speaker-v2 demo.
///
/// This tool plays audio out the speaker via high-frequency hardware PWM.
/// It has lots of controls to adjust the audio, in an attempt to
/// make the MB2 speaker work well.
///
/// If the Cargo feature `external_out` is enabled, this
/// code will output to P0 on the MB2 edge connector instead
/// of the speaker. Attach an RC low-pass antialiasing filter
/// and connect to an external speaker for quite decent audio.

use panic_rtt_target as _;

use cortex_m::asm;
use cortex_m_rt::entry;
use microbit::hal::{gpio, pwm};
use microbit::Board;
use rtt_target::rtt_init_print;

/// Fill `samples` with a sine wave at frequency `f0` given
/// sample rate `r` sps.  The quantization function `q`
/// converts from normalized `f32` values (-1..1) to 16-bit
/// unsigned values.
#[allow(unused)]
fn sine<Q>(samples: &mut [u16], f0: f32, r: u32, mut q: Q)
    where Q: FnMut(f32) -> u16
{
    use core::f32::consts::PI;
    
    let dp = 1.0 / r as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let x = libm::sinf(2.0 * PI * f0 * (i as f32) * dp);
        *s = q(x);
    }
}

/// Fill `samples` with a 2-exponential sine sweep from
/// starting frequency `f0` given sample rate `r` sps.
/// The quantization function `q` converts from normalized
/// `f32` values (-1..1) to 16-bit unsigned values.
// https://en.wikipedia.org/wiki/Chirp
#[allow(unused)]
fn sweep<Q>(samples: &mut [u16], f0: f32, r: u32, mut q: Q)
    where Q: FnMut(f32) -> u16
{
    use core::f32::consts::PI;
    
    let dp = 1.0 / r as f32;
    let ds = 1.0 / libm::logf(2.0);
    for (i, s) in samples.iter_mut().enumerate() {
        let p = libm::powf(2.0, i as f32 * dp);
        let x = libm::sinf(2.0 * PI * f0 * ds * (p - 1.0));
        *s = q(x);
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
        // Stop after each playback.
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
    static mut SAMPS: [u16; 31_250] = [0; 31_250];
    unsafe {
        sweep(
            &mut SAMPS,
            1000.0,
            62500,
            |x| libm::floorf(x * 127.0 + 127.0) as u16
        );
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

    // Start the sweep.
    let samps = unsafe { SAMPS.as_ref() };
    let _pwm_seq = speaker.load(Some(samps), Some(samps), true).unwrap();

    loop {
        asm::wfi();
    }
}
