use crate::app_command::AppCommand;
use crate::audio::engine::AudioEngine;
use crate::audio::load_audio;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::Arc;

pub struct TapeheadApp {
    pub engine: Arc<AudioEngine>,
    pub tx: Sender<AppCommand>,
    rx: Receiver<AppCommand>,
}

impl TapeheadApp {
    pub fn new(engine: Arc<AudioEngine>) -> Self {
        let (tx, rx) = unbounded();
        Self { engine, tx, rx }
    }

    pub fn send_command(&self, cmd: AppCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn process_commands(&self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                AppCommand::LoadIntoDeck { deck, path } => {
                    let engine = self.engine.clone();
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());
                    tokio::spawn(async move {
                        match load_audio(&path) {
                            Ok(source) => {
                                if let Some(deck_arc) = engine.decks.get(deck) {
                                    if let Ok(mut deck) = deck_arc.lock() {
                                        deck.load(source);
                                        deck.filename = filename;
                                        deck.is_playing = true;
                                    }
                                }
                            }
                            Err(e) => eprintln!("Error loading {:?}: {}", path, e),
                        }
                    });
                }
                AppCommand::SetWowFlutterDepth { deck, depth } => {
                    if let Some(deck_arc) = self.engine.decks.get(deck) {
                        if let Ok(mut deck) = deck_arc.lock() {
                            deck.wow_flutter.depth = depth;
                        }
                    }
                }
                AppCommand::SetDropoutProbability { deck, prob } => {
                    if let Some(deck_arc) = self.engine.decks.get(deck) {
                        if let Ok(mut deck) = deck_arc.lock() {
                            deck.dropout.probability = prob;
                        }
                    }
                }
                AppCommand::SetSaturationDrive { deck, drive } => {
                    if let Some(deck_arc) = self.engine.decks.get(deck) {
                        if let Ok(mut deck) = deck_arc.lock() {
                            deck.saturation_l.drive = drive;
                            deck.saturation_r.drive = drive;
                        }
                    }
                }
                AppCommand::SetPlaying { deck, playing } => {
                    if let Some(deck_arc) = self.engine.decks.get(deck) {
                        if let Ok(mut deck) = deck_arc.lock() {
                            deck.is_playing = playing;
                        }
                    }
                }
                AppCommand::SetVolume { deck, volume } => {
                    if let Some(deck_arc) = self.engine.decks.get(deck) {
                        if let Ok(mut deck) = deck_arc.lock() {
                            deck.volume = volume;
                        }
                    }
                }
            }
        }
    }
}
