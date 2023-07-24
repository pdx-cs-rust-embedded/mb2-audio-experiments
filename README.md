# hello-audio: emit 1KHz square wave on MB2 speaker
Bart Massey 2023

This is the "hello world" of audio programs.

There are several branches of interest here.

* The `main` branch emits a 1KHz square
  wave. Super-straightforward straight-line manipulation of
  the speaker.

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
