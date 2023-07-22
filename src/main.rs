#![no_main]
#![no_std]

/// This uses code from the microbit crate speaker-v2 demo.
///
/// This demo plays 8-bit audio — a roughly 1046.5Hz C6 —
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

use panic_rtt_target as _;

use cortex_m::asm;
use cortex_m_rt::entry;
use microbit::hal::{
    clocks::Clocks,
    gpio,
    pwm,
};
use microbit::Board;
use rtt_target::rtt_init_print;

/// Fill `samples` with a full cycle of samples of a sine
/// wave, with samples quantized to `q`-1 *values* 0..`q`.
fn sine(samples: &mut [u16], q: u16) {
    use core::f32::consts::PI;
    let step = 2.0 * PI / samples.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        // Get a value in the range 0.0..1.0.
        let v = (libm::sinf(i as f32 * step) + 1.0) / 2.0;
        // Save a value in the range 0..=q.
        *s = libm::floorf((q - 1) as f32 * v + 0.5) as u16;
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // actual frequency with 60 samples is 1041.7 Hz. Can live with it.
    static mut SAMPLES: [u16; 60] = [0u16; 60];
    unsafe { sine(&mut SAMPLES, 256) };

    let board = Board::take().unwrap();

    // The LF CLK (16MHz) signal is used for PWM.
    let _clocks = Clocks::new(board.CLOCK)
        .enable_ext_hfosc()
        .set_lfclk_src_synth()
        .start_lfclk();

    // Set up the speaker GPIO pin as an output.
    let speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::Low);

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
        // Set maximum duty cycle = PWM period in
        // ticks. 16MHz / 256 = 62_500, our desired sample
        // rate.
        .set_max_duty(256)
        // Set no delay between samples.
        .set_seq_refresh(pwm::Seq::Seq0, 0)
        // Set no delay at end of sequence.
        .set_seq_end_delay(pwm::Seq::Seq0, 0)
        // Keep playing forever.
        .loop_inf()
        // Enable sample channel.
        .enable_channel(pwm::Channel::C0)
        // Enable sample group.
        .enable_group(pwm::Group::G0)
        // Enable PWM.
        .enable();

    // Start the sine wave.
    let _dma = unsafe { speaker.load(Some(&SAMPLES), None::<&[u16]>, true).unwrap() };

    loop {
        asm::wfi();
    }
}
