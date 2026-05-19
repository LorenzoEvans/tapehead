use crate::app_command::AppCommand;
use crate::audio::engine::AudioEngine;
use crate::preset::{load_preset, save_preset, SessionPreset, DeckPreset};
use crossbeam_channel::Sender;
use eframe::egui;
use std::sync::Arc;

pub struct TapeheadGui {
    engine: Arc<AudioEngine>,
    tx: Sender<AppCommand>,
    master_volume: f32,
}

impl TapeheadGui {
    pub fn new(engine: Arc<AudioEngine>, tx: Sender<AppCommand>) -> Self {
        Self {
            engine,
            tx,
            master_volume: 1.0,
        }
    }

    fn send(&self, cmd: AppCommand) {
        let _ = self.tx.send(cmd);
    }
}

impl eframe::App for TapeheadGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle drag and drop
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            for (i, file) in dropped.iter().enumerate().take(4) {
                if let Some(path) = &file.path {
                    self.send(AppCommand::LoadIntoDeck {
                        deck: i,
                        path: path.clone(),
                    });
                }
            }
        }

        egui::TopBottomPanel::bottom("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Master Volume:");
                if ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=2.0)).changed() {
                    for i in 0..4 {
                        self.send(AppCommand::SetVolume { deck: i, volume: self.master_volume });
                    }
                }

                if ui.button("Load Preset").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .pick_file() 
                    {
                        match load_preset(&path) {
                            Ok(preset) => {
                                for (i, dp) in preset.decks.iter().enumerate() {
                                    self.send(AppCommand::SetWowFlutterDepth { deck: i, depth: dp.wow_flutter_depth });
                                    self.send(AppCommand::SetDropoutProbability { deck: i, prob: dp.dropout_probability });
                                    self.send(AppCommand::SetSaturationDrive { deck: i, drive: dp.saturation_drive });
                                    self.send(AppCommand::SetVolume { deck: i, volume: dp.volume });
                                }
                            }
                            Err(e) => eprintln!("Failed to load preset: {}", e),
                        }
                    }
                }

                if ui.button("Save Preset").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .set_file_name("preset.toml")
                        .save_file()
                    {
                        let mut decks = [DeckPreset::default(), DeckPreset::default(), DeckPreset::default(), DeckPreset::default()];
                        for i in 0..4 {
                            if let Ok(deck) = self.engine.decks[i].lock() {
                                decks[i] = DeckPreset {
                                    wow_flutter_depth: deck.wow_flutter.depth,
                                    dropout_probability: deck.dropout.probability,
                                    saturation_drive: deck.saturation_l.drive,
                                    volume: deck.volume,
                                };
                            }
                        }
                        let session = SessionPreset { name: "Saved Preset".to_string(), decks };
                        let _ = save_preset(&session, &path);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("decks_grid")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for i in 0..4 {
                        self.render_deck(ui, i);
                        if i == 1 {
                            ui.end_row();
                        }
                    }
                });
        });

        ctx.request_repaint(); // Keep meters moving
    }
}

impl TapeheadGui {
    fn render_deck(&self, ui: &mut egui::Ui, id: usize) {
        ui.vertical(|ui| {
            ui.group(|ui| {
                ui.set_min_width(300.0);
                ui.heading(format!("Deck {}", id + 1));

                let deck_data = if let Ok(deck) = self.engine.decks[id].lock() {
                    Some((
                        deck.filename.clone().unwrap_or_else(|| "Empty".to_string()),
                        deck.is_playing,
                        deck.wow_flutter.depth,
                        deck.dropout.probability,
                        deck.saturation_l.drive,
                        deck.volume,
                        deck.last_peak,
                    ))
                } else {
                    None
                };

                if let Some((filename, is_playing, mut wf, mut dp, mut sat, mut vol, peak)) = deck_data {
                    ui.label(format!("File: {}", filename));

                    let play_label = if is_playing { "Stop" } else { "Play" };
                    if ui.button(play_label).clicked() {
                        self.send(AppCommand::SetPlaying { deck: id, playing: !is_playing });
                    }

                    ui.label("Wow/Flutter Depth");
                    if ui.add(egui::Slider::new(&mut wf, 0.0..=0.02)).changed() {
                        self.send(AppCommand::SetWowFlutterDepth { deck: id, depth: wf });
                    }

                    ui.label("Dropout Probability");
                    if ui.add(egui::Slider::new(&mut dp, 0.0..=0.01)).changed() {
                        self.send(AppCommand::SetDropoutProbability { deck: id, prob: dp });
                    }

                    ui.label("Saturation Drive");
                    if ui.add(egui::Slider::new(&mut sat, 0.5..=5.0)).changed() {
                        self.send(AppCommand::SetSaturationDrive { deck: id, drive: sat });
                    }

                    ui.label("Volume");
                    if ui.add(egui::Slider::new(&mut vol, 0.0..=1.5)).changed() {
                        self.send(AppCommand::SetVolume { deck: id, volume: vol });
                    }

                    // VU Meter
                    ui.horizontal(|ui| {
                        ui.label("VU:");
                        let progress = (peak.min(1.0)).max(0.0);
                        let color = if progress > 0.9 {
                            egui::Color32::RED
                        } else if progress > 0.7 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GREEN
                        };
                        
                        let (rect, _response) = ui.allocate_at_least(egui::vec2(200.0, 10.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, egui::Color32::BLACK);
                        let mut meter_rect = rect;
                        meter_rect.set_width(rect.width() * progress);
                        ui.painter().rect_filled(meter_rect, 2.0, color);
                    });
                } else {
                    ui.label("Locked...");
                }
            });
        });
    }
}
