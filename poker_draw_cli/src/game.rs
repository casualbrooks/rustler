use std::cmp::Ordering;
use std::io::{self, Write};
use std::process;

use crate::deck::Deck;
use crate::hand;
use crate::player::Player;
use crate::timer::read_line_timeout;

pub struct GameSettings {
    pub num_players: usize,
    pub starting_chips: u32,
    pub min_bet: u32,
    pub turn_timeout_secs: u64,
    pub max_discards: usize,
}

pub struct Game {
    pub settings: GameSettings,
    pub players: Vec<Player>,
    dealer: usize,
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

impl Game {
    pub fn new(settings: GameSettings) -> Self {
        Self {
            settings,
            players: Vec::new(),
            dealer: 0,
        }
    }

    pub fn setup_players(&mut self) {
        self.players.clear();
        for i in 0..self.settings.num_players {
            let mut player = Player::new(i, self.settings.starting_chips);
            loop {
                println!("Enter name for Player {} (max 20 chars):", i + 1);
                if let Some(line) = read_line_timeout("> ", 0) {
                    let name = line.trim().to_string();
                    if !name.is_empty() && name.chars().count() <= 20 {
                        player.name = name;
                        break;
                    }
                }
                println!("Invalid name. Try again.");
            }
            self.players.push(player);
        }
    }

    pub fn play_until_winner(&mut self) -> Player {
        loop {
            self.play_hand();
            if let Some(w) = self.find_table_winner() {
                return w.clone();
            }
        }
    }

    fn active_player_ids(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.chips > 0)
            .map(|(i, _)| i)
            .collect()
    }

    fn find_table_winner(&self) -> Option<&Player> {
        let alive: Vec<&Player> = self.players.iter().filter(|p| p.chips > 0).collect();
        if alive.len() == 1 {
            Some(alive[0])
        } else {
            None
        }
    }

    fn rotate_dealer(&mut self) {
        let actives = self.active_player_ids();
        if actives.is_empty() {
            return;
        }
        // move dealer to next active
        let mut idx = actives.iter().position(|&i| i == self.dealer).unwrap_or(0);
        idx = (idx + 1) % actives.len();
        self.dealer = actives[idx];
    }

    pub fn play_hand(&mut self) {
        clear_screen();
        let mut deck = Deck::new_shuffled();
        // reset per-player state
        for p in self.players.iter_mut() {
            if p.chips > 0 {
                p.reset_for_hand();
            } else {
                p.folded = true;
                p.hand = None;
            }
        }

        let order = self.seat_order_from(self.next_seat(self.dealer));
        let names: Vec<String> = order
            .iter()
            .map(|&pid| self.players[pid].name.clone())
            .collect();
        println!(
            "{} shuffles and deals one card at a time clockwise around the table to {} x5",
            self.players[self.dealer].name,
            names.join(" then ")
        );

        // deal 5 cards to each active player
        for _ in 0..5 {
            for pid in self.seat_order_from(self.next_seat(self.dealer)) {
                if let Some(hand) = self.players[pid].hand.as_mut() {
                    if let Some(card) = deck.deal() {
                        hand.add(card);
                    }
                }
            }
        }

        let mut pot: u32 = 0;

        // First betting round
        pot += self.betting_round("First betting round", &mut deck);

        // If only one player remains after the first round, award the pot
        let remaining: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.folded && p.hand.is_some())
            .map(|(i, _)| i)
            .collect();
        if remaining.len() == 1 {
            let winner = remaining[0];
            self.players[winner].chips += pot;
            println!(
                "{} wins {} chips as all others folded.",
                self.players[winner].name, pot
            );
            self.offer_reveal(winner);
            for p in self.players.iter_mut() {
                p.hand = None;
            }
            self.rotate_dealer();
            return;
        }

        // Draw phase
        self.draw_phase(&mut deck);

        // Second betting round
        pot += self.betting_round("Second betting round", &mut deck);

        // Check again after the second round for a single remaining player
        let remaining: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.folded && p.hand.is_some())
            .map(|(i, _)| i)
            .collect();
        if remaining.len() == 1 {
            let winner = remaining[0];
            self.players[winner].chips += pot;
            println!(
                "{} wins {} chips as all others folded.",
                self.players[winner].name, pot
            );
            self.offer_reveal(winner);
            for p in self.players.iter_mut() {
                p.hand = None;
            }
            self.rotate_dealer();
            return;
        }

        // Showdown with side pots
        let mut contribs: Vec<(usize, u32, bool)> = self
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.contributed_total, !p.folded && p.hand.is_some()))
            .filter(|(_, c, _)| *c > 0)
            .collect();
        contribs.sort_by_key(|k| k.1);

        if contribs.is_empty() {
            self.rotate_dealer();
            return;
        }

        let mut pots: Vec<(u32, Vec<usize>)> = Vec::new();
        let mut prev = 0;
        for i in 0..contribs.len() {
            let contrib = contribs[i].1;
            if contrib == prev {
                continue;
            }
            let involved = &contribs[i..];
            let pot_amount = (contrib - prev) * involved.len() as u32;
            let eligible: Vec<usize> = involved
                .iter()
                .filter(|(_, _, e)| *e)
                .map(|(pid, _, _)| *pid)
                .collect();
            pots.push((pot_amount, eligible));
            prev = contrib;
        }

        println!("Showdown:");
        for (amt, elig) in pots {
            if elig.is_empty() {
                continue;
            }
            let mut best = vec![elig[0]];
            for &pid in elig.iter().skip(1) {
                let ord = {
                    let ha = self.players[best[0]].hand.as_ref().unwrap();
                    let hb = self.players[pid].hand.as_ref().unwrap();
                    hand::compare(ha, hb)
                };
                if ord == Ordering::Less {
                    best = vec![pid];
                } else if ord == Ordering::Equal {
                    best.push(pid);
                }
            }
            let share = amt / best.len() as u32;
            for &pid in &best {
                self.players[pid].chips += share;
            }
            println!("  Pot of {} chips:", amt);
            for pid in &best {
                let p = &self.players[*pid];
                println!(
                    "    {} wins {} with [{}]",
                    p.name,
                    share,
                    p.hand.as_ref().unwrap().fmt_inline()
                );
            }
        }

        // Clear hands
        for p in self.players.iter_mut() {
            p.hand = None;
        }

        self.rotate_dealer();
    }

    fn seat_order_from(&self, start: usize) -> Vec<usize> {
        let n = self.players.len();
        (0..n)
            .map(|i| (start + i) % n)
            .filter(|&i| self.players[i].chips > 0)
            .collect()
    }

    fn next_seat(&self, i: usize) -> usize {
        (i + 1) % self.players.len()
    }

    fn offer_reveal(&self, pid: usize) {
        println!("Reveal your cards? [y/N]");
        if let Some(ans) = read_line_timeout("> ", self.settings.turn_timeout_secs) {
            if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                if let Some(h) = self.players[pid].hand.as_ref() {
                    println!("{} reveals [{}]", self.players[pid].name, h.fmt_inline());
                }
            }
        }
    }

    fn handle_player_quit(&mut self, pid: usize) {
        let chips = self.players[pid].chips;
        if chips > 0 {
            let recipients: Vec<usize> = self
                .players
                .iter()
                .enumerate()
                .filter(|(i, p)| *i != pid && p.chips > 0)
                .map(|(i, _)| i)
                .collect();
            if !recipients.is_empty() {
                let share = chips / recipients.len() as u32;
                let mut rem = chips % recipients.len() as u32;
                for &i in &recipients {
                    let extra = if rem > 0 {
                        rem -= 1;
                        1
                    } else {
                        0
                    };
                    self.players[i].chips += share + extra;
                }
            }
        }
        self.players[pid].chips = 0;
        self.players[pid].folded = true;
        self.players[pid].hand = None;
        self.players[pid].last_action = "quit".to_string();
        println!("{} leaves the game.", self.players[pid].name);
    }

    fn betting_round(&mut self, title: &str, _deck: &mut Deck) -> u32 {
        println!("--- {} ---", title);
        let mut pot: u32 = 0;
        for p in self.players.iter_mut() {
            p.contributed_this_round = 0;
            p.last_action.clear();
        }
        let mut current_bet: u32 = 0;
        let mut last_raiser: Option<usize> = None;

        let order = self.seat_order_from(self.next_seat(self.dealer));
        let mut idx = 0usize;
        let mut seen_since_raise: Vec<bool> = vec![false; self.players.len()];

        loop {
            let pid = order[idx];

            clear_screen();

            let can_continue = self.players.iter().enumerate().any(|(i, p)| {
                order.contains(&i)
                    && !p.folded
                    && !p.all_in
                    && (p.contributed_this_round < current_bet
                        || (current_bet == 0 && !seen_since_raise[i]))
            });
            let need_more = if let Some(lr) = last_raiser {
                !seen_since_raise[lr]
            } else {
                false
            };
            let someone_can_act = self
                .players
                .iter()
                .enumerate()
                .any(|(i, p)| order.contains(&i) && p.can_act());

            if !someone_can_act || (!can_continue && !need_more) {
                break;
            }

            let folded = self.players[pid].folded;
            let all_in = self.players[pid].all_in;
            let chips_zero = self.players[pid].chips == 0;

            if folded || all_in || chips_zero {
                seen_since_raise[pid] = true;
                idx = (idx + 1) % order.len();
                continue;
            }

            let call_diff = current_bet.saturating_sub(self.players[pid].contributed_this_round);

            let chips_after_call = self.players[pid].chips.saturating_sub(call_diff);
            let others_can_call_more = self
                .players
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    order.contains(i) && *i != pid && !p.folded && !p.all_in && p.chips > 0
                })
                .any(|(_, p)| p.chips + p.contributed_this_round > current_bet);
            let can_raise = chips_after_call >= self.settings.min_bet && others_can_call_more;

            let total_pot = pot
                + self
                    .players
                    .iter()
                    .map(|pl| pl.contributed_this_round)
                    .sum::<u32>();
            let active_players: Vec<String> = self
                .players
                .iter()
                .filter(|p| !p.folded && p.hand.is_some())
                .map(|p| p.name.clone())
                .collect();
            let folded_players: Vec<String> = self
                .players
                .iter()
                .filter(|p| p.folded && p.hand.is_some())
                .map(|p| {
                    if !p.revealed_on_fold.is_empty() {
                        let hand_str = p
                            .hand
                            .as_ref()
                            .map(|h| {
                                p.revealed_on_fold
                                    .iter()
                                    .filter_map(|&i| h.cards.get(i))
                                    .map(|c| c.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        format!("{} [{}]", p.name, hand_str)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            println!("Players still in: {}", active_players.join(", "));
            if folded_players.is_empty() {
                println!("Players folded: none");
            } else {
                println!("Players folded: {}", folded_players.join(", "));
            }
            println!("Pot: {}", total_pot);
            println!("Current bet: {}", current_bet);
            println!(
                "Action on: {}. Stack: {} chips. You have {} seconds.",
                self.players[pid].name, self.players[pid].chips, self.settings.turn_timeout_secs
            );
            // numeric action selection with validation
            let mut choice: u32 = 0;
            let mut amount: u32 = 0;
            let mut player_left = false;
            let mut timed_out: bool;
            let mut reveal_idxs: Vec<usize> = Vec::new();
            loop {
                timed_out = false;
                if current_bet == self.players[pid].contributed_this_round {
                    if can_raise {
                        println!(
                            "Actions: [0] Check  [1] Bet <amt>=min {}  [2] Fold  [3] All-in  [4] View cards",
                            self.settings.min_bet
                        );
                    } else {
                        println!("Actions: [0] Check  [2] Fold  [3] All-in  [4] View cards");
                    }
                } else if can_raise {
                    println!(
                        "Actions: [0] Call {}  [1] Raise <amt>=min {}  [2] Fold  [3] All-in  [4] View cards",
                        call_diff, self.settings.min_bet
                    );
                } else {
                    println!(
                        "Actions: [0] Call {}  [2] Fold  [3] All-in  [4] View cards",
                        call_diff
                    );
                }
                println!("Type action number (and amount if needed). Type 'quit' to fold and leave game or 'exit' to quit program.");
                let prompt = if current_bet == self.players[pid].contributed_this_round {
                    "> ".to_string()
                } else {
                    format!("(call {} chips) > ", call_diff)
                };
                let line_opt = read_line_timeout(&prompt, self.settings.turn_timeout_secs);
                let line = match line_opt {
                    Some(l) => l,
                    None => {
                        timed_out = true;
                        String::new()
                    }
                };
                let s = line.trim().to_lowercase();
                if s == "exit" {
                    println!("Are you sure you want to exit? [y/N]");
                    let ans = read_line_timeout("> ", 0).unwrap_or_default();
                    if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                        process::exit(0);
                    } else {
                        println!("Continuing game.");
                        continue;
                    }
                }

                if s == "quit" {
                    println!("Are you sure you want to leave the game? [y/N]");
                    let ans = read_line_timeout("> ", 0).unwrap_or_default();
                    if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                        self.handle_player_quit(pid);
                        seen_since_raise[pid] = true;
                        idx = (idx + 1) % order.len();
                        player_left = true;
                        break;
                    } else {
                        println!("Continuing game.");
                        continue;
                    }
                }
                if timed_out || s.is_empty() {
                    timed_out = true;
                    choice = 2;
                    break;
                }
                let mut parts = s.split_whitespace();
                if let Some(cstr) = parts.next() {
                    if let Ok(c) = cstr.parse::<u32>() {
                        if c == 1 && !can_raise {
                            println!("Invalid option.");
                            continue;
                        }
                        match c {
                            0 => {
                                choice = 0;
                                break;
                            }
                            1 => {
                                if let Some(astr) = parts.next() {
                                    if let Ok(a) = astr.parse::<u32>() {
                                        amount = a;
                                        choice = 1;
                                        break;
                                    }
                                }
                                println!("Need an amount for that action.");
                            }
                            2 => {
                                reveal_idxs = parts
                                    .filter_map(|p| p.parse::<usize>().ok())
                                    .filter(|&i| i < 5)
                                    .collect();
                                reveal_idxs.sort_unstable();
                                reveal_idxs.dedup();
                                choice = 2;
                                break;
                            }
                            3 => {
                                choice = 3;
                                break;
                            }
                            4 => {
                                let hand_str = self.players[pid]
                                    .hand
                                    .as_ref()
                                    .map(|h| h.fmt_inline())
                                    .unwrap_or_default();
                                println!("Hand: [{}]", hand_str);
                                continue;
                            }
                            _ => println!("Invalid option."),
                        }
                    } else {
                        println!("Invalid option.");
                    }
                } else {
                    println!("Invalid option.");
                }
            }

            if player_left {
                continue;
            }

            if choice == 2 {
                self.players[pid].folded = true;
                self.players[pid].last_action = "folded".to_string();
                self.players[pid].revealed_on_fold = reveal_idxs.clone();
                if timed_out {
                    println!("{} folds (timeout).", self.players[pid].name);
                } else {
                    println!("{} folds.", self.players[pid].name);
                    if !reveal_idxs.is_empty() {
                        let hand_str = self.players[pid]
                            .hand
                            .as_ref()
                            .map(|h| {
                                reveal_idxs
                                    .iter()
                                    .filter_map(|&i| h.cards.get(i))
                                    .map(|c| c.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        println!("Shows: [{}]", hand_str);
                    }
                }
            } else if choice == 0 && current_bet == self.players[pid].contributed_this_round {
                self.players[pid].last_action = "checked".to_string();
                println!("{} checks.", self.players[pid].name);
            } else if choice == 0 {
                let mut need = call_diff;
                if need > self.players[pid].chips {
                    need = self.players[pid].chips;
                }
                self.players[pid].chips -= need;
                self.players[pid].contributed_this_round += need;
                self.players[pid].contributed_total += need;
                pot += need;
                if self.players[pid].chips == 0 {
                    self.players[pid].all_in = true;
                }
                if self.players[pid].chips == 0 {
                    self.players[pid].last_action = format!("all-in {}", need);
                } else {
                    self.players[pid].last_action = format!("called {}", need);
                }
                println!("{} calls {}.", self.players[pid].name, need);
            } else if choice == 3 {
                let chips_now = self.players[pid].chips;
                let need = call_diff.min(chips_now);
                let raise_by = chips_now.saturating_sub(need);
                self.players[pid].chips = 0;
                self.players[pid].contributed_this_round += need + raise_by;
                self.players[pid].contributed_total += need + raise_by;
                pot += need + raise_by;
                self.players[pid].all_in = true;
                let prev_bet = current_bet;
                if self.players[pid].contributed_this_round > current_bet {
                    current_bet = self.players[pid].contributed_this_round;
                }
                if self.players[pid].contributed_this_round > prev_bet
                    && self.players[pid].contributed_this_round - prev_bet >= self.settings.min_bet
                {
                    last_raiser = Some(pid);
                    seen_since_raise.fill(false);
                }
                self.players[pid].last_action = format!("all-in {}", need + raise_by);
                println!(
                    "{} goes all-in for {}.",
                    self.players[pid].name,
                    need + raise_by
                );
            } else if choice == 1 && current_bet == self.players[pid].contributed_this_round {
                let chips_now = self.players[pid].chips;
                if amount > chips_now {
                    println!(
                        "Invalid bet. Must be between {} and your chips.",
                        self.settings.min_bet
                    );
                    continue;
                } else if amount < self.settings.min_bet && amount != chips_now {
                    println!(
                        "Invalid bet. Must be at least {} or all-in.",
                        self.settings.min_bet
                    );
                    continue;
                } else {
                    self.players[pid].chips -= amount;
                    self.players[pid].contributed_this_round += amount;
                    self.players[pid].contributed_total += amount;
                    pot += amount;
                    let prev_bet = current_bet;
                    if self.players[pid].contributed_this_round > current_bet {
                        current_bet = self.players[pid].contributed_this_round;
                    }
                    if self.players[pid].contributed_this_round > prev_bet
                        && self.players[pid].contributed_this_round - prev_bet
                            >= self.settings.min_bet
                    {
                        last_raiser = Some(pid);
                        seen_since_raise.fill(false);
                    }
                    if self.players[pid].chips == 0 {
                        self.players[pid].all_in = true;
                        self.players[pid].last_action = format!("all-in {}", amount);
                        println!("{} bets {} and is all-in.", self.players[pid].name, amount);
                    } else {
                        self.players[pid].last_action = format!("bet {}", amount);
                        println!("{} bets {}.", self.players[pid].name, amount);
                    }
                }
            } else if choice == 1 {
                let chips_now = self.players[pid].chips;
                let need = call_diff + amount;
                if need > chips_now {
                    println!("Insufficient chips for that raise. Calling instead.");
                    let mut to_put = call_diff;
                    if to_put > chips_now {
                        to_put = chips_now;
                    }
                    self.players[pid].chips -= to_put;
                    self.players[pid].contributed_this_round += to_put;
                    self.players[pid].contributed_total += to_put;
                    pot += to_put;
                    if self.players[pid].chips == 0 {
                        self.players[pid].all_in = true;
                        self.players[pid].last_action = format!("all-in {}", to_put);
                    } else {
                        self.players[pid].last_action = format!("called {}", to_put);
                    }
                    println!("{} calls {}.", self.players[pid].name, to_put);
                } else if amount < self.settings.min_bet {
                    println!("Invalid raise. Minimum is {}.", self.settings.min_bet);
                    continue;
                } else {
                    self.players[pid].chips -= need;
                    self.players[pid].contributed_this_round += need;
                    self.players[pid].contributed_total += need;
                    pot += need;
                    let prev_bet = current_bet;
                    if self.players[pid].contributed_this_round > current_bet {
                        current_bet = self.players[pid].contributed_this_round;
                    }
                    if self.players[pid].contributed_this_round > prev_bet
                        && self.players[pid].contributed_this_round - prev_bet
                            >= self.settings.min_bet
                    {
                        last_raiser = Some(pid);
                        seen_since_raise.fill(false);
                    }
                    self.players[pid].last_action = format!("raised to {}", current_bet);
                    println!(
                        "{} raises {} (total to {}).",
                        self.players[pid].name, amount, current_bet
                    );
                }
            }

            let active_remaining = self
                .players
                .iter()
                .filter(|p| !p.folded && p.hand.is_some())
                .count();
            if active_remaining <= 1 {
                seen_since_raise[pid] = true;
                break;
            }

            seen_since_raise[pid] = true;
            idx = (idx + 1) % order.len();
        }
        clear_screen();
        pot
    }

    fn draw_phase(&mut self, deck: &mut Deck) {
        println!(
            "--- Draw phase (up to {} cards) ---",
            self.settings.max_discards
        );
        for pid in self.seat_order_from(self.next_seat(self.dealer)) {
            if self.players[pid].folded || self.players[pid].all_in {
                continue;
            }
            clear_screen();
            let pot_total: u32 = self.players.iter().map(|p| p.contributed_total).sum();
            let active_players: Vec<String> = self
                .players
                .iter()
                .filter(|p| !p.folded && p.hand.is_some())
                .map(|p| p.name.clone())
                .collect();
            let folded_players: Vec<String> = self
                .players
                .iter()
                .filter(|p| p.folded && p.hand.is_some())
                .map(|p| {
                    if !p.revealed_on_fold.is_empty() {
                        let hand_str = p
                            .hand
                            .as_ref()
                            .map(|h| {
                                p.revealed_on_fold
                                    .iter()
                                    .filter_map(|&i| h.cards.get(i))
                                    .map(|c| c.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        format!("{} [{}]", p.name, hand_str)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            println!("Players still in: {}", active_players.join(", "));
            if folded_players.is_empty() {
                println!("Players folded: none");
            } else {
                println!("Players folded: {}", folded_players.join(", "));
            }
            println!("Pot: {}", pot_total);
            println!("Action on: {}", self.players[pid].name);
            let pname = self.players[pid].name.clone();
            let mut player_left = false;
            loop {
                println!(
                    "Enter indices to discard (0-4, space-separated), 'stand', or 'view'. Type 'quit' to fold and leave game or 'exit' to quit program. You have {} seconds.",
                    self.settings.turn_timeout_secs
                );
                let line_opt = read_line_timeout("> ", self.settings.turn_timeout_secs);
                let line = match line_opt {
                    Some(l) => l,
                    None => {
                        self.players[pid].folded = true;
                        self.players[pid].last_action = "folded".to_string();
                        println!("{} folds (timeout).", pname);
                        break;
                    }
                };
                let s = line.trim().to_lowercase();

                if s == "exit" {
                    println!("Are you sure you want to exit? [y/N]");
                    let ans = read_line_timeout("> ", 0).unwrap_or_default();
                    if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                        process::exit(0);
                    } else {
                        println!("Continuing game.");
                        continue;
                    }
                }

                if s == "quit" {
                    println!("Are you sure you want to leave the game? [y/N]");
                    let ans = read_line_timeout("> ", 0).unwrap_or_default();
                    if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                        self.handle_player_quit(pid);
                        player_left = true;
                        break;
                    } else {
                        println!("Continuing game.");
                        continue;
                    }
                }

                if s == "view" {
                    if let Some(h) = self.players[pid].hand.as_ref() {
                        println!("Hand: [{}]", h.fmt_inline());
                    }
                    continue;
                }

                if s == "stand" || s.is_empty() {
                    println!("{} stands pat.", pname);
                    break;
                }

                let mut idxs: Vec<usize> = s
                    .split_whitespace()
                    .filter_map(|t| t.parse::<usize>().ok())
                    .collect();
                if idxs.is_empty() {
                    println!("Invalid option.");
                    continue;
                }
                if idxs.len() > self.settings.max_discards {
                    idxs.truncate(self.settings.max_discards);
                }

                {
                    let ph = self.players[pid].hand.as_mut().unwrap();
                    ph.discard_indices(idxs);
                    while ph.cards.len() < 5 {
                        if let Some(c) = deck.deal() {
                            ph.add(c);
                        } else {
                            break;
                        }
                    }
                }

                let after = {
                    let h = self.players[pid].hand.as_ref().unwrap();
                    h.fmt_inline()
                };
                println!("{} discards, new hand: [{}]", pname, after);
                break;
            }
            if player_left {
                continue;
            }
        }
        clear_screen();
    }
}
