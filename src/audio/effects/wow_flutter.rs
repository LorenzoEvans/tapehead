use rand::RngExt;
use std::f32::consts::PI;

pub struct WowFlutter {
    pub phase: f32,
    pub rate_hz: f32,
    pub depth: f32,
}

impl Default for WowFlutter {
    fn default() -> Self {
        Self {
            phase: 0.0,
            rate_hz: 0.5,
            depth: 0.003,
        }
    }
}

impl WowFlutter {
    pub fn new(rate_hz: f32, depth: f32) -> Self {
        Self {
            phase: 0.0,
            rate_hz,
            depth,
        }
    }

    pub fn process(&mut self, sample_rate: f32) -> f32 {
        let mut rng = rand::rng();
        
        // Sine LFO for "wow" (slow)
        let lfo = (self.phase * 2.0 * PI).sin();
        
        // Advance phase
        self.phase = (self.phase + self.rate_hz / sample_rate) % 1.0;
        
        // "Flutter" (fast/random) simulated by small amount of white noise
        let noise: f32 = rng.random_range(-1.0..1.0);
        let flutter_depth = self.depth * 0.2; // Flutter is usually smaller but faster
        
        // Modulation factor around 1.0
        1.0 + (lfo * self.depth) + (noise * flutter_depth)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}
