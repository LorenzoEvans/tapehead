use std::path::PathBuf;

pub enum AppCommand {
    LoadIntoDeck { deck: usize, path: PathBuf },
    SetWowFlutterDepth { deck: usize, depth: f32 },
    SetDropoutProbability { deck: usize, prob: f32 },
    SetSaturationDrive { deck: usize, drive: f32 },
    SetPlaying { deck: usize, playing: bool },
    SetVolume { deck: usize, volume: f32 },
}
