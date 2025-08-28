// filepath: /workspaces/rustler/poker_draw_cli/src/deck.rs
use rand::{seq::SliceRandom, thread_rng};
use crate::card::{Card, Rank, Suit};

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
        let mut rng = thread_rng();
        cards.shuffle(&mut rng);
        Self { cards }
    }

    pub fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}