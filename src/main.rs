#![no_main]
#![no_std]

/// This is a lightly-adapted version of the microbit crate speaker-v2 demo.
///
/// This code sweeps the output square wave frequency up
/// from 1Hz at a given sweep rate and PWM duty cycle. When
/// it reaches a maximum frequency, it holds for a stop time
/// and then restarts.
///
/// See the code for constants controlling these parameters.

use panic_halt as _;

use cortex_m_rt::entry;
use microbit::Board;
use microbit::hal::{
    clocks::Clocks,
    gpio,
    pac::{self, interrupt},
    prelude::*,
    pwm,
    rtc::{Rtc, RtcInterrupt},
    time::Hertz,
};

// Ticks per second.
const TICK: u32 = 64;
// Stop frequency in Hertz.
const STOP_FREQUENCY: u32 = 500;
// Time to hold at stop frequency in seconds.
const HOLD_TIME: u32 = 5;
// Duty cycle is 65_536 / DUTY. Value must be at least 2 * 65_536.
// Values greater than 65_536 * STOP_FREQUENCY are not useful and
// should be avoided.
const DUTY: u32 = 65_536 * 23 / 5;

use core::cell::RefCell;
use cortex_m::{asm, interrupt::Mutex};

static RTC: Mutex<RefCell<Option<Rtc<pac::RTC0>>>> = Mutex::new(RefCell::new(None));
static SPEAKER: Mutex<RefCell<Option<pwm::Pwm<pac::PWM0>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut board = Board::take().unwrap();
    cortex_m::interrupt::free(move |cs| {
        // NB: The LF CLK pin is used by the speaker
        let _clocks = Clocks::new(board.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_synth()
            .start_lfclk();

        // Set up ticks: TICK = 32768 / (d + 1), so d = 32768 / TICK - 1.
        let mut rtc = Rtc::new(board.RTC0, 32768 / TICK - 1).unwrap();
        rtc.enable_counter();
        rtc.enable_interrupt(RtcInterrupt::Tick, Some(&mut board.NVIC));
        rtc.enable_event(RtcInterrupt::Tick);

        *RTC.borrow(cs).borrow_mut() = Some(rtc);

        let mut speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::High);
        speaker_pin.set_low().unwrap();

        // Use the PWM peripheral to generate a waveform for the speaker
        let speaker = pwm::Pwm::new(board.PWM0);
        speaker
            // output the waveform on the speaker pin
            .set_output_pin(pwm::Channel::C0, speaker_pin.degrade())
            // Use prescale by 16 to achive darker sounds
            .set_prescaler(pwm::Prescaler::Div16)
            // Initial frequency
            .set_period(Hertz(1u32))
            // Configure for up and down counter mode
            .set_counter_mode(pwm::CounterMode::UpAndDown)
            // Set initial maximum duty cycle. This will immediately
            // be changed in the interrupt handler.
            .set_max_duty(32767)
            // enable PWM
            .enable();

        speaker
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            .set_seq_end_delay(pwm::Seq::Seq0, 0);

        // Configure initial duty cycle to 50%. This will
        // immediately be changed in the interrupt handler.
        let max_duty = speaker.max_duty();
        speaker.set_duty_on_common(max_duty / 2);

        *SPEAKER.borrow(cs).borrow_mut() = Some(speaker);

        // Configure RTC interrupt
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::RTC0);
        }
        pac::NVIC::unpend(pac::Interrupt::RTC0);
    });

    loop {
        asm::wfi();
    }
}

// RTC interrupt, exectued for each RTC tick
#[interrupt]
fn RTC0() {
    static mut FREQUENCY: u32 = 1;
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        /* Borrow devices */
        if let (Some(speaker), Some(rtc)) = (
            SPEAKER.borrow(cs).borrow().as_ref(),
            RTC.borrow(cs).borrow().as_ref(),
        ) {
            if *FREQUENCY >= STOP_FREQUENCY + HOLD_TIME * TICK {
                // Reset the frequency for a new sweep.
                *FREQUENCY = 1;
            };

            if *FREQUENCY < STOP_FREQUENCY {
                // Configure the new frequency, must not be zero.
                // Will change the max_duty
                speaker.set_period(Hertz(*FREQUENCY));
            } else {
                // Continue at frequency
                speaker.set_period(Hertz(STOP_FREQUENCY));
            }

            // Restart the PWM at duty cycle
            let max_duty = speaker.max_duty() as u32;
            let duty = max_duty * 65536 / DUTY;
            let duty = duty.clamp(1, max_duty / 2);
            speaker.set_duty_on_common(duty as u16);

            // Clear the RTC interrupt
            rtc.reset_event(RtcInterrupt::Tick);
        }
    });
    // Increase the frequency
    *FREQUENCY += 1;
}
