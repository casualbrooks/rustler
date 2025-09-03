// filepath: /workspaces/rustler/poker_draw_cli/src/deck.rs
use crate::card::{Card, Rank, Suit};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new_shuffled() -> Self {
        let mut cards = Vec::with_capacity(52);
        let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
        for s in suits {
            for r in Rank::ALL {
                cards.push(Card { rank: r, suit: s });
            }
        }
        // Shuffle using a simple LCG-based algorithm so we don't rely on external crates.
        // This isn't cryptographically secure but suffices for card shuffling in the CLI.
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        // Fisher-Yates shuffle
        for i in (1..cards.len()).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (seed % (i as u64 + 1)) as usize;
            cards.swap(i, j);
        }
        Self { cards }
    }

    pub fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}