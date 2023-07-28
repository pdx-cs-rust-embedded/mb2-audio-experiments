#![no_main]
#![no_std]

/// This uses code from the microbit crate speaker-v2 demo.
///
/// This demo plays 8-bit audio — a sample hard-compiled
/// into the code — out the speaker via high-frequency
/// hardware PWM.
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

static SAMPLE: &[u8] = include_bytes!("sample.u8");
const BLOCK_SIZE: usize = 4096;

fn blocks() -> impl Iterator<Item = [u8; BLOCK_SIZE]> {
    let mut index = 0;
    let nsample = SAMPLE.len();
    core::iter::from_fn(move || {
        if index >= nsample {
            return None;
        }
        let mut block = [0; BLOCK_SIZE];
        if index + BLOCK_SIZE > nsample {
            let rest = &SAMPLE[index..];
            block[..rest.len()].copy_from_slice(&rest);
        } else {
            block.copy_from_slice(&SAMPLE[index..index + BLOCK_SIZE]);
        };
        index += BLOCK_SIZE;
        Some(block)
    })
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();

    // Set up the speaker GPIO pin as an output.
    #[cfg(not(feature = "external_out"))]
    let speaker_pin = board.speaker_pin;
    #[cfg(feature = "external_out")]
    // Send output to edge connector P0 instead of speaker pin.
    let speaker_pin = board.pins.p0_02;
    let speaker_pin = speaker_pin.into_push_pull_output(gpio::Level::High);

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

    // Set up the wave. This has to be in RAM for the
    // PWM unit to access it. It needs to be a 16-bit buffer
    // even though we will have only 8-bit (ish) sample
    // resolution.
    static mut SAMPS: [u16; BLOCK_SIZE] = [0; BLOCK_SIZE];

    // Start the sine wave.
    let _pwm_seq = unsafe { speaker.load(Some(&SAMPS), Some(&SAMPS), true).unwrap() };

    loop {
        asm::wfi();
    }
}
