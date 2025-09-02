mod card;
mod deck;
mod hand;
mod player;
mod timer;
mod game;

use game::{Game, GameSettings};
use timer::read_line_timeout;

fn prompt_number(prompt: &str, min: u32, max: u32, step: Option<u32>) -> u32 {
    loop {
        let step_str = match step {
            Some(s) => format!(", step {}", s),
            None => String::new(),
        };
        println!("{} [{}..{}{}]:", prompt, min, max, step_str);

        if let Some(line) = read_line_timeout("> ", 0) { // 0 = no timeout for setup
            if let Ok(val) = line.trim().parse::<u32>() {
                if val >= min && val <= max && step.map_or(true, |s| val % s == 0) {
                    return val;
                }
            }
        }
        println!("Invalid input. Try again.");
    }
}

fn main() {
    println!("Five-Card Draw Poker (CLI)");

    loop {
        let num_players = prompt_number("Number of players", 2, 6, None);
        let starting_chips = prompt_number("Starting chips (increments of 10)", 10, 10_000, Some(10));
        let turn_secs = prompt_number("Turn timer (seconds)", 5, 300, None) as u64;

        let settings = GameSettings {
            num_players: num_players as usize,
            starting_chips,
            min_bet: 10,
            turn_timeout_secs: turn_secs,
            max_discards: 3, // common variant
        };

        let mut game = Game::new(settings);
        game.setup_players();

        let winner = game.play_until_winner();
        println!("Winner: {}", winner.name);

        println!("Start a new game with same settings? [y/N]");
        let again = read_line_timeout("> ", 0).unwrap_or_default();
        if again.trim().to_lowercase() != "y" {
            break;
        }
    }
}
