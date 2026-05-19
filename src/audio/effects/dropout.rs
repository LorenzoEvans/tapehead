use rand::RngExt;

pub struct Dropout {
    pub probability: f32,
    pub duration_samples: usize,
    pub remaining_samples: usize,
}

impl Default for Dropout {
    fn default() -> Self {
        Self {
            probability: 0.0001,
            duration_samples: 200, // Default duration
            remaining_samples: 0,
        }
    }
}

impl Dropout {
    pub fn new(probability: f32, duration_samples: usize) -> Self {
        Self {
            probability,
            duration_samples,
            remaining_samples: 0,
        }
    }

    /// Updates the dropout state machine. Should be called once per frame.
    pub fn update_state(&mut self) {
        let mut rng = rand::rng();

        if self.remaining_samples == 0 {
            if rng.random_range(0.0..1.0) < self.probability {
                self.remaining_samples = self.duration_samples;
            }
        } else {
            self.remaining_samples -= 1;
        }
    }

    /// Applies the current dropout state to a sample.
    pub fn apply(&self, sample: f32) -> f32 {
        if self.remaining_samples > 0 {
            // Near-zero output (small bleed for realism)
            sample * 0.02
        } else {
            sample
        }
    }

    pub fn reset(&mut self) {
        self.remaining_samples = 0;
    }
}
