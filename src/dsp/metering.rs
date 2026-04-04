/// Simple stereo peak meter with smoothed decay.
pub struct PeakMeter {
    peak_l: f32,
    peak_r: f32,
    decay_coeff: f32,
}

impl PeakMeter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            peak_l: 0.0,
            peak_r: 0.0,
            decay_coeff: Self::calc_decay(sample_rate),
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.decay_coeff = Self::calc_decay(sr);
    }

    pub fn reset(&mut self) {
        self.peak_l = 0.0;
        self.peak_r = 0.0;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) {
        let al = l.abs();
        let ar = r.abs();
        self.peak_l = if al > self.peak_l { al } else { self.peak_l * self.decay_coeff };
        self.peak_r = if ar > self.peak_r { ar } else { self.peak_r * self.decay_coeff };
    }

    pub fn peak_db(&self) -> (f32, f32) {
        (lin_to_db(self.peak_l), lin_to_db(self.peak_r))
    }

    pub fn peak_linear(&self) -> (f32, f32) {
        (self.peak_l, self.peak_r)
    }

    /// ~300ms decay to -inf at given sample rate.
    fn calc_decay(sr: f32) -> f32 {
        (-1.0 / (0.3 * sr)).exp()
    }
}

#[inline(always)]
fn lin_to_db(x: f32) -> f32 {
    if x > 1e-12 { 20.0 * x.log10() } else { -120.0 }
}
