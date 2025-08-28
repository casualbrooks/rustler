use std::fmt;
use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Suit { Clubs, Diamonds, Hearts, Spades }

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Rank {
    Two=2, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven,
        Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace
    ];
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self.rank {
            Rank::Two => "2", Rank::Three => "3", Rank::Four => "4", Rank::Five => "5",
            Rank::Six => "6", Rank::Seven => "7", Rank::Eight => "8", Rank::Nine => "9",
            Rank::Ten => "T", Rank::Jack => "J", Rank::Queen => "Q", Rank::King => "K",
            Rank::Ace => "A",
        };
        let s = match self.suit {
            Suit::Clubs => "♣", Suit::Diamonds => "♦", Suit::Hearts => "♥", Suit::Spades => "♠"
        };
        write!(f, "{}{}", r, s)
    }
}

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