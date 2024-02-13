# mb2-audio-experiments: audio output on the MicroBit v2
Bart Massey 2024

These are experiments with speaker and external audio on the
MicroBit 2. The branches here started as branches of
<https://github.com/pdx-cs-rust-embedded/hello-audio>, with
which this repository shares a common root.

There are many branches here. Several of note:

* The `main` branch emits a 1KHz square wave while button A
  is held down. Super-straightforward straight-line
  manipulation of the speaker.

* The `handrolled-pwm` branch tries to do programmatic PWM
  to make a sine wave. I never got it to work, but it's
  interesting to look at.

* The `hw-pwm` branch emits a sine(ish) wave using the
  hardware PWM unit.

* The `v2-speaker-demo` branch is a cleaned-up clone of the
  `microbit` crate example `v2-speaker`.

# Acknowledgements

Thanks to the `microbit` crate authors for a demo to get
started with. Thanks to Keith Packard for helping debug the
`hw-pwm` branch.

# License

This work is licensed under the "MIT License". Please see the file
`LICENSE.txt` in this distribution for license terms.
