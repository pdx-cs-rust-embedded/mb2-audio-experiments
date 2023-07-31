// Fourth-order Butterworth 1/16-band lowpass filter as
// sequential second-order sections.  From Python
// scipy.signal.iirfilter:
//
// signal.iirfilter(4, 1/16, rs=40, btype='lowpass', ftype='butter', output='sos')
#[allow(clippy::excessive_precision)]
const COEFFS: [[[f32; 3]; 2]; 2] = [
    [
        [7.27725493e-05,  1.45545099e-04,  7.27725493e-05],
        [1.00000000e+00, -1.66200996e+00,  6.94570660e-01],
    ],
    [
        [1.00000000e+00,  2.00000000e+00,  1.00000000e+00],
        [1.00000000e+00, -1.82529778e+00,  8.61057480e-01],
    ],
];

fn biquad(x: &[f32; 3], y: &[f32; 2], c: &[[f32; 3]; 2]) -> f32 {
    let b = c[0];
    let a = c[1];
    b[0] * x[0] + b[1] * x[1] + b[2] * x[2] - a[1] * y[0] - a[2] * y[1]
}

// Run a single second-order filter section.
fn section(
    x: impl Iterator<Item=f32>,
    c: [[f32; 3]; 2],
) -> impl Iterator<Item=f32> {
    let mut xs = [0.0; 3];
    let mut ys = [0.0; 2];
    x.map(move |x0| {
        xs[2] = xs[1];
        xs[1] = xs[0];
        xs[0] = x0;
        let y0 = biquad(&xs, &ys, &c);
        ys[1] = ys[0];
        ys[0] = y0;
        y0
    })
}

pub struct Upsample16 {
    i_dest: usize,
    i_source: usize,
    source: &'static [f32],
}

impl Upsample16 {
    pub fn new(source: &'static f32) -> Self {
        Upsample16 { i_dest: 0, i_source: 0, source }
    }

    pub fn fill(&mut self, &mut dest) -> bool {
        for s_out in dest {
            let out = if self.i_out == 0 && self.i_in < self.source.len() {
                self.i_in += 1;
                16.0 * (self.source[self.i_in - 1] as f32 - 128.0)
            } else {
                0.0
            };
            self.i_out = (self.i_out + 1) % 16;
        }
        self.i_in < self.source.len()
    }
}



// Two-stage sequential SOS filter.
fn filter(x: impl Iterator<Item=f32>) -> impl Iterator<Item=f32> {
    let s0 = section(x, COEFFS[0]);
    section(s0, COEFFS[1])
}

// Resample the input signal to the output. Both input
// and output are 8-bit unsigned samples.
pub fn resample(x: impl Iterator<Item=u8>) -> impl Iterator<Item=u8> {
    let input = x.map(|s| {
        15.0 * (s as f32 - 128.0)
    });
    let interp = interpolate_zeros(input, 16);
    let filt = filter(interp);
    filt.map(|s| {
        (s + 128.0).clamp(0.0, 255.0) as u8
    })
}
