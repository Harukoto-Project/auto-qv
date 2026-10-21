#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PenDown,
    PenUp,
    Move { dx: i32, dy: i32 },
}
