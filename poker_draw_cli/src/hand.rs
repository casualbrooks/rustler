use std::cmp::Ordering;
use std::collections::HashMap;

use crate::card::{Card, Rank, Suit};

#[derive(Clone, Debug)]
pub struct Hand {
    pub cards: Vec<Card>, // always 5 during evaluation
}

impl Hand {
    pub fn new() -> Self {
        Self { cards: Vec::with_capacity(5) }
    }

    pub fn add(&mut self, c: Card) { self.cards.push(c); }

    pub fn discard_indices(&mut self, mut idxs: Vec<usize>) {
        idxs.sort_unstable();
        idxs.dedup();
        // remove from highest index to lowest to keep indices valid
        for &i in idxs.iter().rev() {
            if i < self.cards.len() {
                self.cards.remove(i);
            }
        }
    }

    pub fn fmt_inline(&self) -> String {
        self.cards.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Category {
    HighCard = 1,
    OnePair = 2,
    TwoPair = 3,
    ThreeKind = 4,
    Straight = 5,
    Flush = 6,
    FullHouse = 7,
    FourKind = 8,
    StraightFlush = 9,
}

#[derive(Clone, Debug, Eq)]
pub struct Evaluated {
    cat: Category,
    // tiebreakers from highest to lowest significance
    keys: [u8; 5], // ranks as 2..14 (Ace high), use 1 (Ace low) handled in straight
}

impl PartialEq for Evaluated {
    fn eq(&self, other: &Self) -> bool {
        self.cat == other.cat && self.keys == other.keys
    }
}

impl Ord for Evaluated {
    fn cmp(&self, other: &Self) -> Ordering {
        let cat_cmp = self.cat.cmp(&other.cat);
        if cat_cmp != Ordering::Equal { return cat_cmp; }
        self.keys.cmp(&other.keys)
    }
}

impl PartialOrd for Evaluated {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

fn rank_val(r: Rank) -> u8 { r as u8 }

pub fn evaluate(hand: &Hand) -> Evaluated {
    let cards = &hand.cards;
    debug_assert_eq!(cards.len(), 5);

    let mut by_rank: HashMap<u8, u8> = HashMap::new();
    let mut suits: HashMap<Suit, u8> = HashMap::new();

    for c in cards {
        *by_rank.entry(rank_val(c.rank)).or_default() += 1;
        *suits.entry(c.suit).or_default() += 1;
    }

    let mut ranks: Vec<u8> = cards.iter().map(|c| rank_val(c.rank)).collect();
    ranks.sort_unstable();
    ranks.dedup();

    let is_flush = suits.values().any(|&n| n == 5);

    // straight detection with wheel (A=14 can be 1)
    let mut rank_vals: Vec<u8> = cards.iter().map(|c| rank_val(c.rank)).collect();
    rank_vals.sort_unstable();
    let mut is_straight = false;
    let mut high_in_straight = *rank_vals.last().unwrap();
    if rank_vals.windows(2).all(|w| w[1] == w[0] + 1) {
        is_straight = true;
    } else {
        // wheel: A(14),5,4,3,2 -> treat Ace as 1
        if rank_vals == vec![2,3,4,5,14] {
            is_straight = true;
            high_in_straight = 5; // 5-high straight
        }
    }

    // counts sorted by (count desc, rank desc)
    let mut counts: Vec<(u8,u8)> = by_rank.into_iter().collect(); // (rank, count)
    counts.sort_by(|a,b| {
        let c = b.1.cmp(&a.1);
        if c != Ordering::Equal { return c; }
        b.0.cmp(&a.0) // higher rank first
    });

    let keys_from_counts = || {
        let mut ks = Vec::with_capacity(5);
        for (rank, cnt) in counts.iter() {
            for _ in 0..*cnt { ks.push(*rank); }
        }
        // pad to 5
        ks.resize(5, 0);
        [ks[0], ks[1], ks[2], ks[3], ks[4]]
    };

    if is_straight && is_flush {
        // keys: high card of straight, rest zeros
        return Evaluated { cat: Category::StraightFlush, keys: [high_in_straight,0,0,0,0] };
    }

    if counts[0].1 == 4 {
        // four kind: quad rank, kicker
        let quad = counts[0].0;
        let kicker = counts.iter().find(|(_,c)| *c==1).map(|(r,_)| *r).unwrap_or(0);
        return Evaluated { cat: Category::FourKind, keys: [quad, quad, quad, quad, kicker] };
    }

    if counts[0].1 == 3 && counts[1].1 == 2 {
        // full house: trips rank, pair rank
        return Evaluated { cat: Category::FullHouse, keys: [counts[0].0, counts[0].0, counts[0].0, counts[1].0, counts[1].0] };
    }

    if is_flush {
        // high cards
        let mut ks: Vec<u8> = cards.iter().map(|c| rank_val(c.rank)).collect();
        ks.sort_unstable_by(|a,b| b.cmp(a));
        return Evaluated { cat: Category::Flush, keys: [ks[0],ks[1],ks[2],ks[3],ks[4]] };
    }

    if is_straight {
        return Evaluated { cat: Category::Straight, keys: [high_in_straight,0,0,0,0] };
    }

    if counts[0].1 == 3 {
        return Evaluated { cat: Category::ThreeKind, keys: keys_from_counts() };
    }

    if counts[0].1 == 2 && counts[1].1 == 2 {
        // two pair: high pair, low pair, kicker
        // counts already sorted
        let highp = counts[0].0;
        let lowp = counts[1].0;
        let kicker = counts.iter().find(|(_,c)| *c==1).map(|(r,_)| *r).unwrap_or(0);
        return Evaluated { cat: Category::TwoPair, keys: [highp, highp, lowp, lowp, kicker] };
    }

    if counts[0].1 == 2 {
        return Evaluated { cat: Category::OnePair, keys: keys_from_counts() };
    }

    // high card
    let mut ks: Vec<u8> = cards.iter().map(|c| rank_val(c.rank)).collect();
    ks.sort_unstable_by(|a,b| b.cmp(a));
    Evaluated { cat: Category::HighCard, keys: [ks[0],ks[1],ks[2],ks[3],ks[4]] }
}

pub fn compare(h1: &Hand, h2: &Hand) -> std::cmp::Ordering {
    evaluate(h1).cmp(&evaluate(h2))
}