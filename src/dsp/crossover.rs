use std::f32::consts::PI;

#[derive(Clone, Copy)]
struct Biquad {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl Biquad {
    fn new() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    fn reset(&mut self) {
        self.x1 = 0.0; self.x2 = 0.0; self.y1 = 0.0; self.y2 = 0.0;
    }

    fn set_lr2_lowpass(&mut self, freq: f32, sr: f32) {
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * 0.5);
        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0;
        self.a1 = (-2.0 * cos_w0) / a0; self.a2 = (1.0 - alpha) / a0;
    }

    fn set_lr2_highpass(&mut self, freq: f32, sr: f32) {
        let w0 = 2.0 * PI * freq / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * 0.5);
        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        self.b0 = b0 / a0; self.b1 = b1 / a0; self.b2 = b2 / a0;
        self.a1 = (-2.0 * cos_w0) / a0; self.a2 = (1.0 - alpha) / a0;
    }

    #[inline(always)]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = x;
        self.y2 = self.y1; self.y1 = y;
        y
    }
}

#[derive(Clone, Copy)]
struct StereoCrossover {
    lp_l: Biquad, hp_l: Biquad,
    lp_r: Biquad, hp_r: Biquad,
}

impl StereoCrossover {
    fn new() -> Self {
        Self { lp_l: Biquad::new(), hp_l: Biquad::new(), lp_r: Biquad::new(), hp_r: Biquad::new() }
    }

    fn set_freq(&mut self, freq: f32, sr: f32) {
        self.lp_l.set_lr2_lowpass(freq, sr); self.hp_l.set_lr2_highpass(freq, sr);
        self.lp_r.set_lr2_lowpass(freq, sr); self.hp_r.set_lr2_highpass(freq, sr);
    }

    fn reset(&mut self) {
        self.lp_l.reset(); self.hp_l.reset(); self.lp_r.reset(); self.hp_r.reset();
    }

    #[inline(always)]
    fn process(&mut self, l: f32, r: f32) -> ((f32, f32), (f32, f32)) {
        ((self.lp_l.tick(l), self.lp_r.tick(r)), (self.hp_l.tick(l), self.hp_r.tick(r)))
    }
}

/// 3-band stereo crossover (low / mid / high).
pub struct Crossover3Band {
    xover_low: StereoCrossover,
    xover_high: StereoCrossover,
    freq_low: f32,
    freq_high: f32,
    sample_rate: f32,
}

impl Crossover3Band {
    pub fn new(sample_rate: f32) -> Self {
        let mut c = Self {
            xover_low: StereoCrossover::new(),
            xover_high: StereoCrossover::new(),
            freq_low: 200.0,
            freq_high: 5000.0,
            sample_rate,
        };
        c.xover_low.set_freq(200.0, sample_rate);
        c.xover_high.set_freq(5000.0, sample_rate);
        c
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr;
        self.xover_low.set_freq(self.freq_low, sr);
        self.xover_high.set_freq(self.freq_high, sr);
    }

    pub fn reset(&mut self) {
        self.xover_low.reset();
        self.xover_high.reset();
    }

    pub fn set_freqs(&mut self, low: f32, high: f32) {
        self.freq_low = low;
        self.freq_high = high;
        self.xover_low.set_freq(low, self.sample_rate);
        self.xover_high.set_freq(high, self.sample_rate);
    }

    /// Split stereo input into 3 bands: (low_l, low_r), (mid_l, mid_r), (high_l, high_r).
    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> ((f32, f32), (f32, f32), (f32, f32)) {
        let (low, rest) = self.xover_low.process(l, r);
        let (mid, high) = self.xover_high.process(rest.0, rest.1);
        (low, mid, high)
    }
}
