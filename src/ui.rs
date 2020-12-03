use tui::widgets::TableState;

// https://github.com/fdehau/tui-rs/blob/master/examples/table.rs
pub struct StatefulTable {
    pub state: TableState,
    pub items: Vec<Vec<String>>,
}

impl StatefulTable {
    pub fn new() -> StatefulTable {
        StatefulTable {
            state: TableState::default(),
            items: vec![],
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn jump_forward(&mut self, n: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                usize::min(i + n, self.items.len() - 1)
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn jump_backward(&mut self, n: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if n > i {
                    0
                } else {
                    i - n
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn set_items(&mut self, items: Vec<Vec<String>>) {
        self.items = items;
    }
}
