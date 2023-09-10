#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    NetworkRequests,
    RequestDetails,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Insert,
    Normal,
}

pub struct App {
    pub active_block: ActiveBlock,
    pub mode: Mode,
}

impl App {
    pub fn new() -> App {
        App {
            active_block: ActiveBlock::NetworkRequests,
            mode: Mode::Normal,
        }
    }
}
