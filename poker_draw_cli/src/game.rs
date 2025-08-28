use std::cmp::Ordering;
use std::process;

use crate::deck::Deck;
use crate::hand;
use crate::player::Player;
use crate::timer::read_line_timeout;

pub struct GameSettings {
    pub num_players: usize,
    pub starting_chips: u32,
    pub bet_increment: u32,
    pub turn_timeout_secs: u64,
    pub max_discards: usize,
}

pub struct Game {
    pub settings: GameSettings,
    pub players: Vec<Player>,
    dealer: usize,
}

impl Game {
    pub fn new(settings: GameSettings) -> Self {
        Self { settings, players: Vec::new(), dealer: 0 }
    }

    pub fn setup_players(&mut self) {
        self.players.clear();
        for i in 0..self.settings.num_players {
            self.players.push(Player::new(i, self.settings.starting_chips));
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
        self.players.iter().enumerate()
            .filter(|(_,p)| p.chips > 0)
            .map(|(i,_)| i).collect()
    }

    fn find_table_winner(&self) -> Option<&Player> {
        let alive: Vec<&Player> = self.players.iter().filter(|p| p.chips > 0).collect();
        if alive.len() == 1 { Some(alive[0]) } else { None }
    }

    fn rotate_dealer(&mut self) {
        let actives = self.active_player_ids();
        if actives.is_empty() { return; }
        // move dealer to next active
        let mut idx = actives.iter().position(|&i| i == self.dealer).unwrap_or(0);
        idx = (idx + 1) % actives.len();
        self.dealer = actives[idx];
    }

    pub fn play_hand(&mut self) {
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

        // deal 5 cards to each active player
        for _ in 0..5 {
            for pid in self.seat_order_from(self.next_seat(self.dealer)) {
                if let Some(hand) = self.players[pid].hand.as_mut() {
                    if let Some(card) = deck.deal() { hand.add(card); }
                }
            }
        }

        let mut pot: u32 = 0;

        // First betting round
        pot += self.betting_round("First betting round", &mut deck);

        // Draw phase
        self.draw_phase(&mut deck);

        // Second betting round
        pot += self.betting_round("Second betting round", &mut deck);

        // Showdown
        let mut contenders: Vec<usize> = self.players.iter().enumerate()
            .filter(|(_,p)| !p.folded && p.hand.is_some())
            .map(|(i,_)| i).collect();

        if contenders.is_empty() {
            // everyone folded, pot stays (unlikely); give pot to last folder? Simplify: do nothing.
            self.rotate_dealer();
            return;
        }

        contenders.sort_by(|&a, &b| {
            let ha = self.players[a].hand.as_ref().unwrap();
            let hb = self.players[b].hand.as_ref().unwrap();
            hand::compare(hb, ha) // desc
        });

        let best_pid = contenders[0];
        // If multiple tie, split evenly (simplified; ignores odd chip)
        let mut winners = vec![best_pid];
        for &pid in contenders.iter().skip(1) {
            let ord = {
                let ha = self.players[best_pid].hand.as_ref().unwrap();
                let hb = self.players[pid].hand.as_ref().unwrap();
                hand::compare(ha, hb)
            };
            if ord == Ordering::Equal {
                winners.push(pid);
            } else {
                break;
            }
        }

        let share = if winners.is_empty() { 0 } else { pot / winners.len() as u32 };
        for &pid in &winners {
            self.players[pid].chips += share;
        }
        println!("Showdown:");
        for pid in &winners {
            let p = &self.players[*pid];
            println!("  {} wins {} chips with [{}]", p.name, share, p.hand.as_ref().unwrap().fmt_inline());
        }

        // Clear hands
        for p in self.players.iter_mut() {
            p.hand = None;
        }

        self.rotate_dealer();
    }

    fn seat_order_from(&self, start: usize) -> Vec<usize> {
        let n = self.players.len();
        (0..n).map(|i| (start + i) % n)
            .filter(|&i| self.players[i].chips > 0)
            .collect()
    }

    fn next_seat(&self, i: usize) -> usize {
        (i + 1) % self.players.len()
    }

    fn betting_round(&mut self, title: &str, _deck: &mut Deck) -> u32 {
        println!("--- {} ---", title);
        let mut pot: u32 = 0;
        for p in self.players.iter_mut() {
            p.contributed_this_round = 0;
        }
        let mut current_bet: u32 = 0;
        let mut last_raiser: Option<usize> = None;

        let order = self.seat_order_from(self.next_seat(self.dealer));
        let mut idx = 0usize;
        let mut seen_since_raise: Vec<bool> = vec![false; self.players.len()];

        loop {
            let pid = order[idx];

            let can_continue = self.players.iter().enumerate()
                .any(|(i,p)| order.contains(&i) && !p.folded && !p.all_in && (p.contributed_this_round < current_bet));
            let need_more = if let Some(lr) = last_raiser { !seen_since_raise[lr] } else { false };
            let someone_can_act = self.players.iter().enumerate()
                .any(|(i,p)| order.contains(&i) && p.can_act());

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
            let chips = self.players[pid].chips;

            println!();
            println!("Pot: {}", pot + self.players.iter().map(|pl| pl.contributed_this_round).sum::<u32>());
            for pl in &self.players {
                if pl.chips == 0 { continue; }
                println!("  {:<10} chips:{:<5} bet:{:<5} {}", pl.name, pl.chips, pl.contributed_this_round, if pl.folded { "(folded)" } else { "" });
            }
            let hand_str = self.players[pid].hand.as_ref().map(|h| h.fmt_inline()).unwrap_or_default();
            println!("{} to act. Hand: [{}]. You have {} seconds.", self.players[pid].name, hand_str, self.settings.turn_timeout_secs);
            println!("Allowed: {}  | Type 'quit' to exit.",
                if current_bet == self.players[pid].contributed_this_round { "check, bet <amount>, fold" } else { "call, raise <amount>, fold, all-in" }
            );
            let prompt = format!("(call {} chips) > ", call_diff);

            let line = read_line_timeout(&prompt, self.settings.turn_timeout_secs).unwrap_or_default();
            let action = line.trim().to_lowercase();

            // quit/exit command with confirmation
            if action == "quit" || action == "exit" {
                println!("Are you sure you want to quit? [y/N]");
                let ans = read_line_timeout("> ", 0).unwrap_or_default();
                if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                    process::exit(0);
                } else {
                    println!("Continuing game.");
                    continue; // same player still to act
                }
            }

            let mut fold = false;
            let mut check = false;
            let mut call = false;
            let mut raise_amt: u32 = 0;
            let mut bet_amt: u32 = 0;
            let mut all_in = false;

            if action.is_empty() {
                println!("Time out or empty input: fold.");
                fold = true;
            } else if action.starts_with("fold") {
                fold = true;
            } else if action.starts_with("check") && current_bet == self.players[pid].contributed_this_round {
                check = true;
            } else if action.starts_with("call") && current_bet > self.players[pid].contributed_this_round {
                call = true;
            } else if action.starts_with("all") {
                all_in = true;
            } else if action.starts_with("raise") {
                let amt = action.split_whitespace().nth(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                raise_amt = amt;
            } else if action.starts_with("bet") && current_bet == self.players[pid].contributed_this_round {
                let amt = action.split_whitespace().nth(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                bet_amt = amt;
            } else {
                println!("Unrecognized/invalid action. Fold by default.");
                fold = true;
            }

            let inc = self.settings.bet_increment;

            if fold {
                self.players[pid].folded = true;
                println!("{} folds.", self.players[pid].name);
            } else if check {
                println!("{} checks.", self.players[pid].name);
            } else if call {
                let mut need = call_diff;
                if need > self.players[pid].chips { need = self.players[pid].chips; }
                self.players[pid].chips -= need;
                self.players[pid].contributed_this_round += need;
                self.players[pid].contributed_total += need;
                pot += need;
                if self.players[pid].chips == 0 { self.players[pid].all_in = true; }
                println!("{} calls {}.", self.players[pid].name, need);
            } else if all_in {
                let chips_now = self.players[pid].chips;
                let need = call_diff.min(chips_now);
                let raise_by = chips_now.saturating_sub(need);
                self.players[pid].chips = 0;
                self.players[pid].contributed_this_round += need + raise_by;
                self.players[pid].contributed_total += need + raise_by;
                pot += need + raise_by;
                self.players[pid].all_in = true;
                if self.players[pid].contributed_this_round > current_bet {
                    current_bet = self.players[pid].contributed_this_round;
                    last_raiser = Some(pid);
                    seen_since_raise.fill(false);
                }
                println!("{} goes all-in for {}.", self.players[pid].name, need + raise_by);
            } else if bet_amt > 0 && current_bet == self.players[pid].contributed_this_round {
                if bet_amt % inc != 0 || bet_amt == 0 || bet_amt > self.players[pid].chips {
                    println!("Invalid bet. Must be multiple of {} and <= your chips.", inc);
                    self.players[pid].folded = true;
                    println!("{} folds (invalid bet).", self.players[pid].name);
                } else {
                    self.players[pid].chips -= bet_amt;
                    self.players[pid].contributed_this_round += bet_amt;
                    self.players[pid].contributed_total += bet_amt;
                    pot += bet_amt;
                    current_bet = self.players[pid].contributed_this_round;
                    last_raiser = Some(pid);
                    seen_since_raise.fill(false);
                    println!("{} bets {}.", self.players[pid].name, bet_amt);
                }
            } else if raise_amt > 0 && current_bet > self.players[pid].contributed_this_round {
                if raise_amt % inc != 0 {
                    println!("Invalid raise increment.");
                    self.players[pid].folded = true;
                    println!("{} folds (invalid raise).", self.players[pid].name);
                } else {
                    let chips_now = self.players[pid].chips;
                    let need = call_diff + raise_amt;
                    if need > chips_now {
                        println!("Insufficient chips for that raise. Going all-in for {}.", chips_now);
                        let to_put = chips_now;
                        self.players[pid].chips = 0;
                        self.players[pid].contributed_this_round += to_put;
                        self.players[pid].contributed_total += to_put;
                        pot += to_put;
                        self.players[pid].all_in = true;
                        if self.players[pid].contributed_this_round > current_bet {
                            current_bet = self.players[pid].contributed_this_round;
                            last_raiser = Some(pid);
                            seen_since_raise.fill(false);
                        }
                    } else {
                        self.players[pid].chips -= need;
                        self.players[pid].contributed_this_round += need;
                        self.players[pid].contributed_total += need;
                        pot += need;
                        current_bet = self.players[pid].contributed_this_round;
                        last_raiser = Some(pid);
                        seen_since_raise.fill(false);
                        println!("{} raises {} (total to {}).", self.players[pid].name, raise_amt, current_bet);
                    }
                }
            } else {
                println!("Invalid/illegal action in this spot. Fold.");
                self.players[pid].folded = true;
            }

            seen_since_raise[pid] = true;
            idx = (idx + 1) % order.len();
        }

        pot
    }

    fn draw_phase(&mut self, deck: &mut Deck) {
        println!("--- Draw phase (up to {} cards) ---", self.settings.max_discards);
        for pid in self.seat_order_from(self.next_seat(self.dealer)) {
            if self.players[pid].folded || self.players[pid].all_in { continue; }
            let pname = self.players[pid].name.clone();
            let before = {
                let h = self.players[pid].hand.as_ref().unwrap();
                h.fmt_inline()
            };
            println!("{}'s hand: [{}]", pname, before);
            println!(
                "Enter indices to discard (0-4, space-separated), or 'stand'. Type 'quit' to exit. You have {} seconds.",
                self.settings.turn_timeout_secs
            );

            let line = read_line_timeout("> ", self.settings.turn_timeout_secs).unwrap_or_else(|| "stand".to_string());
            let s = line.trim().to_lowercase();

            if s == "quit" || s == "exit" {
                println!("Are you sure you want to quit? [y/N]");
                let ans = read_line_timeout("> ", 0).unwrap_or_default();
                if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                    process::exit(0);
                } else {
                    println!("Continuing game.");
                    continue; // same player continues to draw choice next loop iteration
                }
            }

            if s == "stand" || s.is_empty() {
                println!("{} stands pat.", pname);
                continue;
            }

            let mut idxs: Vec<usize> = s.split_whitespace().filter_map(|t| t.parse::<usize>().ok()).collect();
            if idxs.len() > self.settings.max_discards {
                idxs.truncate(self.settings.max_discards);
            }

            {
                let ph = self.players[pid].hand.as_mut().unwrap();
                ph.discard_indices(idxs);
                while ph.cards.len() < 5 {
                    if let Some(c) = deck.deal() { ph.add(c); } else { break; }
                }
            }

            let after = {
                let h = self.players[pid].hand.as_ref().unwrap();
                h.fmt_inline()
            };
            println!("{} draws. New hand: [{}]", pname, after);
        }
    }
}