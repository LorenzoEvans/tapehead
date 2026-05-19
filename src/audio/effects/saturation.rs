pub struct Saturation {
    pub drive: f32,
    pub lp_alpha: f32, // 0.0 to 1.0 (1.0 = no filter)
    last_sample: f32,
}

impl Default for Saturation {
    fn default() -> Self {
        Self {
            drive: 1.0,
            lp_alpha: 0.8, // Gentle rolloff
            last_sample: 0.0,
        }
    }
}

impl Saturation {
    pub fn new(drive: f32, lp_alpha: f32) -> Self {
        Self {
            drive,
            lp_alpha,
            last_sample: 0.0,
        }
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        // Soft-clip tape saturation
        let x = sample;
        let saturated = x / (1.0 + x.abs() * self.drive);

        // Single-pole IIR lowpass
        let output = self.lp_alpha * saturated + (1.0 - self.lp_alpha) * self.last_sample;
        self.last_sample = output;
        
        output
    }

    pub fn reset(&mut self) {
        self.last_sample = 0.0;
    }
}
