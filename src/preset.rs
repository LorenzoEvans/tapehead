use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use anyhow::Result;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeckPreset {
    pub wow_flutter_depth: f32,
    pub dropout_probability: f32,
    pub saturation_drive: f32,
    pub volume: f32,
}

impl Default for DeckPreset {
    fn default() -> Self {
        Self {
            wow_flutter_depth: 0.003,
            dropout_probability: 0.0001,
            saturation_drive: 1.0,
            volume: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPreset {
    pub name: String,
    pub decks: [DeckPreset; 4],
}

pub fn save_preset(preset: &SessionPreset, path: &Path) -> Result<()> {
    let toml = toml::to_string_pretty(preset)?;
    fs::write(path, toml)?;
    Ok(())
}

pub fn load_preset(path: &Path) -> Result<SessionPreset> {
    let content = fs::read_to_string(path)?;
    let preset = toml::from_str(&content)?;
    Ok(preset)
}
