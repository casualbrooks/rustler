use crate::hand::Hand;

#[derive(Clone)]
pub struct Player {
    #[allow(dead_code)]
    pub id: usize,
    pub name: String,
    pub chips: u32,
    pub folded: bool,
    pub all_in: bool,
    pub hand: Option<Hand>,
    pub contributed_this_round: u32,
    pub contributed_total: u32,
    pub last_action: String,
    pub revealed_on_fold: Vec<usize>,
}

impl Player {
    pub fn new(id: usize, chips: u32) -> Self {
        Self {
            id,
            name: format!("Player {}", id + 1),
            chips,
            folded: false,
            all_in: false,
            hand: None,
            contributed_this_round: 0,
            contributed_total: 0,
            last_action: String::new(),
            revealed_on_fold: Vec::new(),
        }
    }

    pub fn reset_for_hand(&mut self) {
        self.folded = false;
        self.all_in = false;
        self.hand = Some(Hand::new());
        self.contributed_this_round = 0;
        self.contributed_total = 0;
        self.last_action.clear();
        self.revealed_on_fold.clear();
    }

    pub fn can_act(&self) -> bool {
        !self.folded && !self.all_in && self.chips > 0
    }
}