use crate::error::Result;
use crate::planner::action::Action;

pub trait PenDriver {
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()>;
    fn press(&mut self) -> Result<()>;
    fn release(&mut self) -> Result<()>;
}

pub fn apply_action(driver: &mut dyn PenDriver, action: Action) -> Result<()> {
    match action {
        Action::PenDown => driver.press(),
        Action::PenUp => driver.release(),
        Action::Move { dx, dy } => driver.move_rel(dx, dy),
    }
}
