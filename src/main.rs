mod audio;
mod cli;
mod app;
mod app_command;
mod preset;
mod gui;

use clap::Parser;
use cli::{Cli, Commands, PresetCommands};
use app::TapeheadApp;
use app_command::AppCommand;
use audio::engine::AudioEngine;
use audio::load_audio;
use preset::{load_preset, save_preset, SessionPreset, DeckPreset};
use gui::TapeheadGui;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle Batch command separately to avoid starting CPAL
    if let Some(Commands::Batch { input, output, preset: preset_path }) = &cli.command {
        println!("Batch processing: {:?} -> {:?}", input, output);
        
        let source = load_audio(input)?;
        let mut deck = audio::engine::Deck::default();
        let sample_rate = source.sample_rate as f32;
        deck.load(source);
        deck.is_playing = true;

        if let Some(pp) = preset_path {
            let preset = load_preset(pp)?;
            let dp = &preset.decks[0]; // Use first deck preset
            deck.wow_flutter.depth = dp.wow_flutter_depth;
            deck.dropout.probability = dp.dropout_probability;
            deck.saturation_l.drive = dp.saturation_drive;
            deck.saturation_r.drive = dp.saturation_drive;
            deck.volume = dp.volume;
        }

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(output, spec)?;

        let total_frames = deck.buffer.len() / deck.channels;
        let mut last_progress = -1;

        while deck.is_playing {
            let (l, r) = deck.next_stereo_sample(sample_rate);
            writer.write_sample(l)?;
            writer.write_sample(r)?;

            let progress = ((deck.cursor / total_frames as f32) * 100.0) as i32;
            if progress % 5 == 0 && progress != last_progress {
                eprint!("\rProgress: {}%", progress);
                std::io::stderr().flush()?;
                last_progress = progress;
            }
        }
        eprintln!("\rProgress: 100%");
        writer.finalize()?;
        println!("Done!");
        return Ok(());
    }

    let engine = Arc::new(AudioEngine::new());
    let app = Arc::new(TapeheadApp::new(engine.clone()));
    
    // Load preset if provided
    if let Some(preset_path) = &cli.preset {
        match load_preset(preset_path) {
            Ok(preset) => {
                println!("Loading preset: {}", preset.name);
                for (i, deck_preset) in preset.decks.iter().enumerate() {
                    app.send_command(AppCommand::SetWowFlutterDepth { deck: i, depth: deck_preset.wow_flutter_depth });
                    app.send_command(AppCommand::SetDropoutProbability { deck: i, prob: deck_preset.dropout_probability });
                    app.send_command(AppCommand::SetSaturationDrive { deck: i, drive: deck_preset.saturation_drive });
                    app.send_command(AppCommand::SetVolume { deck: i, volume: deck_preset.volume });
                }
            }
            Err(e) => eprintln!("Error loading preset {:?}: {}", preset_path, e),
        }
    }

    // Set up CPAL
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let config = device.default_output_config()?;

    let app_cb = app.clone();
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                app_cb.process_commands();
                for frame in data.chunks_mut(channels) {
                    let mut mixed_left = 0.0;
                    let mut mixed_right = 0.0;
                    for deck_arc in &app_cb.engine.decks {
                        if let Ok(mut deck) = deck_arc.lock() {
                            let (left, right) = deck.next_stereo_sample(sample_rate);
                            mixed_left += left;
                            mixed_right += right;
                        }
                    }
                    if channels >= 2 {
                        frame[0] = mixed_left;
                        frame[1] = mixed_right;
                    } else {
                        frame[0] = (mixed_left + mixed_right) * 0.5;
                    }
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None,
        )?,
        _ => anyhow::bail!("Unsupported sample format"),
    };

    stream.play()?;

    match cli.command {
        Some(Commands::Batch { .. }) => unreachable!(),
        Some(Commands::Preset { command }) => {
            match command {
                PresetCommands::List => {
                    println!("Presets list: (Not implemented: scanning presets directory)");
                }
                PresetCommands::Export { name, output } => {
                    let mut decks = [
                        DeckPreset::default(),
                        DeckPreset::default(),
                        DeckPreset::default(),
                        DeckPreset::default(),
                    ];
                    
                    for i in 0..4 {
                        if let Ok(deck) = engine.decks[i].lock() {
                            decks[i] = DeckPreset {
                                wow_flutter_depth: deck.wow_flutter.depth,
                                dropout_probability: deck.dropout.probability,
                                saturation_drive: deck.saturation_l.drive,
                                volume: deck.volume,
                            };
                        }
                    }
                    
                    let session = SessionPreset { name, decks };
                    match save_preset(&session, &output) {
                        Ok(_) => println!("Preset exported to {:?}", output),
                        Err(e) => eprintln!("Error exporting preset: {}", e),
                    }
                }
            }
        }
        None => {
            if !cli.files.is_empty() {
                println!("Loading files: {:?}", cli.files);
                for (i, file_path) in cli.files.iter().enumerate().take(4) {
                    app.send_command(AppCommand::LoadIntoDeck {
                        deck: i,
                        path: file_path.clone(),
                    });
                }
            }
            
            println!("Launching GUI...");
            let native_options = eframe::NativeOptions::default();
            let gui_app = TapeheadGui::new(engine.clone(), app.tx.clone());
            eframe::run_native(
                "Tapehead",
                native_options,
                Box::new(|_cc| Box::new(gui_app)),
            ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
        }
    }

    Ok(())
}
