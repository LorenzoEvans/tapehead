use crate::audio::AudioSource;
use crate::audio::effects::{WowFlutter, Dropout, Saturation};

pub struct Deck {
    pub buffer: Vec<f32>,
    pub cursor: f32,
    pub is_playing: bool,
    pub channels: usize,
    pub sample_rate: f32,
    pub volume: f32,
    pub last_peak: f32,
    pub filename: Option<String>,
    
    // Effects
    pub wow_flutter: WowFlutter,
    pub dropout: Dropout,
    pub saturation_l: Saturation,
    pub saturation_r: Saturation,

    // Toggles
    pub wow_flutter_enabled: bool,
    pub dropout_enabled: bool,
    pub saturation_enabled: bool,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            cursor: 0.0,
            is_playing: false,
            channels: 2,
            sample_rate: 44100.0,
            volume: 1.0,
            last_peak: 0.0,
            filename: None,
            
            wow_flutter: WowFlutter::default(),
            dropout: Dropout::default(),
            saturation_l: Saturation::default(),
            saturation_r: Saturation::default(),

            wow_flutter_enabled: true,
            dropout_enabled: true,
            saturation_enabled: true,
        }
    }
}

impl Deck {
    pub fn load(&mut self, source: AudioSource) {
        self.buffer = source.samples;
        self.channels = source.channels;
        self.sample_rate = source.sample_rate as f32;
        self.cursor = 0.0;
        self.wow_flutter.reset();
        self.dropout.reset();
        self.saturation_l.reset();
        self.saturation_r.reset();
        self.last_peak = 0.0;
    }

    pub fn next_stereo_sample(&mut self, output_sample_rate: f32) -> (f32, f32) {
        if !self.is_playing || self.buffer.is_empty() {
            self.last_peak *= 0.95; // Decay
            return (0.0, 0.0);
        }

        // ... (rest of the pitch modulation and interpolation logic)
        let modulation = if self.wow_flutter_enabled {
            self.wow_flutter.process(output_sample_rate)
        } else {
            1.0
        };
        
        let advance = (self.sample_rate / output_sample_rate) * modulation;

        let num_frames = self.buffer.len() / self.channels;
        if self.cursor >= (num_frames - 1) as f32 {
            self.is_playing = false;
            self.last_peak = 0.0;
            return (0.0, 0.0);
        }

        let index = self.cursor.floor() as usize;
        let fract = self.cursor - self.cursor.floor();

        let get_interpolated = |idx: usize, channel: usize| -> f32 {
            let offset = idx * self.channels + channel;
            let next_offset = (idx + 1) * self.channels + channel;
            
            if next_offset < self.buffer.len() {
                let s1 = self.buffer[offset];
                let s2 = self.buffer[next_offset];
                s1 + fract * (s2 - s1)
            } else {
                self.buffer[offset]
            }
        };

        let mut left = get_interpolated(index, 0);
        let mut right = if self.channels >= 2 {
            get_interpolated(index, 1)
        } else {
            left
        };

        // 2. Dropout
        if self.dropout_enabled {
            self.dropout.update_state();
            left = self.dropout.apply(left);
            right = self.dropout.apply(right);
        }

        // 3. Saturation
        if self.saturation_enabled {
            left = self.saturation_l.process(left);
            right = self.saturation_r.process(right);
        }

        self.cursor += advance;
        let out_l = left * self.volume;
        let out_r = right * self.volume;
        
        // Update peak with decay
        let peak = out_l.abs().max(out_r.abs());
        if peak > self.last_peak {
            self.last_peak = peak;
        } else {
            self.last_peak *= 0.999; // Very slow decay for meters
        }

        (out_l, out_r)
    }
}
