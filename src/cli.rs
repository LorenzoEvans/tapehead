use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tapehead")]
#[command(about = "A Cassette-Tape Emulator", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files to load into decks on startup
    pub files: Vec<PathBuf>,

    /// Session preset to load on startup
    #[arg(short, long)]
    pub preset: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run in batch processing mode
    Batch {
        /// Input file
        input: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: PathBuf,
        /// Preset to apply
        #[arg(short, long)]
        preset: Option<PathBuf>,
    },
    /// Manage presets
    Preset {
        #[command(subcommand)]
        command: PresetCommands,
    },
}

#[derive(Subcommand)]
pub enum PresetCommands {
    /// List available presets
    List,
    /// Export current state as a preset
    Export {
        /// Name of the preset
        name: String,
        /// Output TOML file
        output: PathBuf,
    },
}
