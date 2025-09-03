use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: u128,
    pub player: String,
    pub action: String,
}

impl LogEntry {
    fn new(player: &str, action: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            timestamp: ts,
            player: player.to_string(),
            action: action.to_string(),
        }
    }
}

#[derive(Default)]
pub struct HandLog {
    pub events: Vec<LogEntry>,
    pub private: Vec<LogEntry>,
}

pub struct TableLog {
    pub table_name: String,
    pub hands: Vec<HandLog>,
}

impl TableLog {
    pub fn new() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            table_name: format!("table-{}", ts),
            hands: Vec::new(),
        }
    }

    pub fn start_hand(&mut self) {
        self.hands.push(HandLog::default());
    }

    fn current_mut(&mut self) -> Option<&mut HandLog> {
        self.hands.last_mut()
    }

    pub fn log_action(&mut self, player: &str, action: &str) {
        if let Some(h) = self.current_mut() {
            h.events.push(LogEntry::new(player, action));
        }
    }

    pub fn log_private(&mut self, player: &str, action: &str) {
        if let Some(h) = self.current_mut() {
            h.private.push(LogEntry::new(player, action));
        }
    }

    pub fn dump(&self) {
        println!("=== Table Log: {} ===", self.table_name);
        for (i, hand) in self.hands.iter().enumerate() {
            println!("-- Hand {} --", i + 1);
            for e in &hand.events {
                println!("[{}] {}: {}", e.timestamp, e.player, e.action);
            }
        }
    }

    pub fn dump_private(&self) {
        println!("=== Private Card Log: {} ===", self.table_name);
        for (i, hand) in self.hands.iter().enumerate() {
            println!("-- Hand {} --", i + 1);
            for e in &hand.private {
                println!("[{}] {}: {}", e.timestamp, e.player, e.action);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_public_and_private_actions() {
        let mut log = TableLog::new();
        log.start_hand();
        log.log_action("Alice", "bets 10");
        log.log_private("Alice", "Ah Kd");
        assert_eq!(log.hands.len(), 1);
        assert_eq!(log.hands[0].events[0].player, "Alice");
        assert_eq!(log.hands[0].private[0].action, "Ah Kd");
    }
}
