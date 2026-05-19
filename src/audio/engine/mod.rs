pub mod deck;

use std::sync::{Arc, Mutex};
pub use deck::Deck;

pub struct AudioEngine {
    pub decks: Vec<Arc<Mutex<Deck>>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let mut decks = Vec::with_capacity(4);
        for _ in 0..4 {
            decks.push(Arc::new(Mutex::new(Deck::default())));
        }
        Self { decks }
    }
}
