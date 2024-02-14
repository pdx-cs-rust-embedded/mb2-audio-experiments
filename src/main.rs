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

use core::f32::consts::PI;
    
use panic_rtt_target as _;

use cortex_m_rt::entry;
use microbit::hal::{prelude::*, gpio, pwm, delay};
use microbit::Board;
use rtt_target::rtt_init_print;

const SAMPLE_RATE: u32 = 125_000;
const BASE_FREQ: f32 = 1000.0;
const TARGET_BUFFER_LENGTH: usize = 16_384;
const SAMPLE_RANGE: u32 = 16_000_000 / SAMPLE_RATE;

/// Fill `samples` with silence.
fn silence(samples: &mut [u16], _f0: f32, _r: u32, q: fn(f32) -> u16) {
    for s in samples.iter_mut() {
        *s = q(0.0);
    }
}

/// Fill `samples` with a sine wave at frequency `f0` given
/// sample rate `r` sps.  The quantization function `q`
/// converts from normalized `f32` values (-1..1) to 16-bit
/// unsigned values.
fn sine(samples: &mut [u16], f0: f32, r: u32, q: fn(f32) -> u16) {
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
fn sweep(samples: &mut [u16], f0: f32, r: u32, q: fn(f32) -> u16) {
    let dp = 1.0 / r as f32;
    let ds = 1.0 / libm::logf(2.0);
    for (i, s) in samples.iter_mut().enumerate() {
        let p = libm::powf(2.0, i as f32 * dp);
        let x = libm::sinf(2.0 * PI * f0 * ds * (p - 1.0));
        *s = q(x);
    }
}

/// Fill `samples` with a chord at root frequency `f0` given
/// sample rate `r` sps.  The quantization function `q`
/// converts from normalized `f32` values (-1..1) to 16-bit
/// unsigned values.
fn chord(samples: &mut [u16], f0: f32, r: u32, q: fn(f32) -> u16) {
    let dp = 1.0 / r as f32;
    let freqs = [
        f0,
        f0 * libm::powf(2.0, 4.0 / 12.0),
        f0 * libm::powf(2.0, 7.0 / 12.0),
    ];
    let gain = 1.0 / freqs.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let mut acc = 0.0;
        for f in freqs {
            acc += libm::sinf(2.0 * PI * f * (i as f32) * dp);
        }
        *s = q(acc * gain);
    }
}

/// Convert the input sample from -1.0..1.0 to a
/// half-amplitude four-bit value represented as u16.
fn conv(x: f32) -> u16 {
    let x = (0.5 * x + 1.0) * 0.5;
    libm::floorf(x * (SAMPLE_RANGE - 1) as f32) as u16
}

/// Set up the waveform. This has to be in RAM for the
/// PWM unit to access it. It needs to be a 16-bit buffer
/// even though we will have only 8-bit (ish) sample
/// resolution.
fn make_wave(g: fn(&mut [u16], f32, u32, fn(f32) -> u16)) -> &'static [u16] {
    // Length of 1 cycle of waveform.
    const CYCLE: usize = (SAMPLE_RATE / BASE_FREQ as u32) as usize;
    // Fit as many cycles into buffer as is feasible to allow for
    // polyphony.
    const LEN: usize = (TARGET_BUFFER_LENGTH / CYCLE) * CYCLE;
    static mut SAMPS: [u16; LEN] = [0; LEN];

    // Safety: This is just Rust being dumb about globals.

    g(
        unsafe { &mut SAMPS },
        BASE_FREQ,
        SAMPLE_RATE,
        conv,
    );

    for s in unsafe { &mut SAMPS } {
        // The default counter mode is to set low up to
        // the count, then set high until the end of the
        // cycle. Setting the high bit in the count
        // register inverts this (and is otherwise
        // ignored).
        *s |= 0x8000;
    }

    unsafe { SAMPS.as_ref() }
}


#[entry]
fn main() -> ! {
    rtt_init_print!();
    let mut board = Board::take().unwrap();

    // Set up the speaker GPIO pin as an output.
    #[cfg(not(feature = "external_out"))]
    let speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::Low);
    #[cfg(feature = "external_out")]
    // Send output to edge connector P0 instead of speaker pin.
    let speaker_pin = board.pins.p0_02.into_push_pull_output(gpio::Level::Low);

    // Use the PWM peripheral to generate a waveform for the speaker
    // The base counter rate for the PWM is 16MHz.
    // https://jimmywongiot.com/2021/06/01/advanced-pulse-width-modulation-pwm-on-nordic-nrf52-series/
    // This lets us run at 1M sps with 16 ticks per sample, since 1M * 16 = 16M.
    // We can thus run 4-bit samples at this rate.
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
        .set_max_duty((16_000_000 / SAMPLE_RATE) as u16 - 1)
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
        // Loop after each playback.
        .loop_inf()
        // Enable PWM.
        .enable();

    let button_a = board.buttons.button_a.degrade();
    let button_b = board.buttons.button_b.degrade();
    let mut delay = delay::Delay::new(board.SYST);

    board.display_pins.row1.set_high().unwrap();
    let mut leds = [
        board.display_pins.col1.degrade(),
        board.display_pins.col2.degrade(),
        board.display_pins.col3.degrade(),
        board.display_pins.col4.degrade(),
        board.display_pins.col5.degrade(),
    ];
    for led in &mut leds {
        led.set_high().unwrap();
    }

    let waves = [silence, sine, sweep, chord];
    let n_waves = waves.len();
    let mut cur_wave = 0;

    leds[cur_wave].set_low().unwrap();
    let samps = make_wave(waves[cur_wave]);
    let mut pwm_seq = speaker.load(Some(samps), Some(samps), true).unwrap();
    loop {
        let a = button_a.is_low().unwrap();
        let b = button_b.is_low().unwrap();
        if a || b {
            let mut button_release = |b: &gpio::Pin<gpio::Input<gpio::Floating>>| {
                delay.delay_us(10_000u16);
                while b.is_low().unwrap() {
                    delay.delay_us(1000u16);
                }
            };

            let prev_wav = cur_wave;
            if b {
                cur_wave = (cur_wave + n_waves + 1) % n_waves;
                leds[cur_wave].set_low().unwrap();
                button_release(&button_b);
            } else {
                cur_wave = (cur_wave + n_waves - 1) % n_waves;
                leds[cur_wave].set_low().unwrap();
                button_release(&button_a);
            }
            pwm_seq.stop();
            let (_, _, speaker) = pwm_seq.split();
            let samps = make_wave(waves[cur_wave]);
            pwm_seq = speaker.load(Some(samps), Some(samps), true).unwrap();
            leds[prev_wav].set_high().unwrap();
        }
    }
}
