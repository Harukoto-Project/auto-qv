use super::pen_driver::PenDriver;
use crate::error::Result;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DryRunStats {
    pub move_count: usize,
    pub pen_down_count: usize,
    pub draw_distance: f64,
    pub travel_distance: f64,
}

#[derive(Debug, Default)]
pub struct DryRunDriver {
    pen_down: bool,
    stats: DryRunStats,
}

impl DryRunDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> &DryRunStats {
        &self.stats
    }
}

impl PenDriver for DryRunDriver {
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()> {
        let distance = f64::from(dx).hypot(f64::from(dy));
        self.stats.move_count += 1;
        if self.pen_down {
            self.stats.draw_distance += distance;
        } else {
            self.stats.travel_distance += distance;
        }
        Ok(())
    }

    fn press(&mut self) -> Result<()> {
        self.pen_down = true;
        self.stats.pen_down_count += 1;
        Ok(())
    }

    fn release(&mut self) -> Result<()> {
        self.pen_down = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::pen_driver::apply_action;
    use crate::planner::action::Action;

    #[test]
    fn separates_draw_and_travel_distance() {
        let mut driver = DryRunDriver::new();
        let actions = [
            Action::Move { dx: 3, dy: 4 },
            Action::PenDown,
            Action::Move { dx: 6, dy: 8 },
            Action::PenUp,
            Action::Move { dx: -9, dy: -12 },
        ];
        for action in actions {
            apply_action(&mut driver, action).unwrap();
        }

        let stats = driver.stats();
        assert_eq!(stats.move_count, 3);
        assert_eq!(stats.pen_down_count, 1);
        assert_eq!(stats.draw_distance, 10.0);
        assert_eq!(stats.travel_distance, 20.0);
    }
}
