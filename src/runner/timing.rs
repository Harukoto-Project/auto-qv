use std::time::Duration;

use crate::planner::action::Action;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepTiming {
    pub step_delay_ms: u32,
    pub max_counts_per_step: u32,
    pub move_speed_multiplier: f32,
}

impl Default for StepTiming {
    fn default() -> Self {
        Self {
            step_delay_ms: 2,
            max_counts_per_step: 8,
            move_speed_multiplier: 3.0,
        }
    }
}

impl StepTiming {
    pub fn max_counts_for(&self, pen_down: bool) -> u32 {
        let base = self.max_counts_per_step.max(1);
        if pen_down {
            base
        } else {
            ((base as f32 * self.move_speed_multiplier).round() as u32).max(1)
        }
    }

    pub fn step_count(&self, dx: i32, dy: i32, pen_down: bool) -> u32 {
        let distance = dx.unsigned_abs().max(dy.unsigned_abs());
        distance.div_ceil(self.max_counts_for(pen_down))
    }
}

pub fn estimate_duration(actions: &[Action], timing: &StepTiming) -> Duration {
    let mut pen_down = false;
    let mut steps: u64 = 0;
    for action in actions {
        match action {
            Action::PenDown => pen_down = true,
            Action::PenUp => pen_down = false,
            Action::Move { dx, dy } => steps += u64::from(timing.step_count(*dx, *dy, pen_down)),
        }
    }
    Duration::from_millis(steps * u64::from(timing.step_delay_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> StepTiming {
        StepTiming {
            step_delay_ms: 2,
            max_counts_per_step: 8,
            move_speed_multiplier: 3.0,
        }
    }

    #[test]
    fn travel_moves_allow_more_counts_per_step() {
        assert_eq!(timing().max_counts_for(true), 8);
        assert_eq!(timing().max_counts_for(false), 24);
    }

    #[test]
    fn step_count_uses_the_larger_axis_and_rounds_up() {
        assert_eq!(timing().step_count(17, -3, true), 3);
        assert_eq!(timing().step_count(0, 0, true), 0);
        assert_eq!(timing().step_count(-24, 5, false), 1);
    }

    #[test]
    fn estimates_duration_from_steps_and_pen_state() {
        let actions = [
            Action::Move { dx: 48, dy: 0 },
            Action::PenDown,
            Action::Move { dx: 16, dy: 0 },
            Action::PenUp,
        ];

        assert_eq!(
            estimate_duration(&actions, &timing()),
            Duration::from_millis((2 + 2) * 2)
        );
    }

    #[test]
    fn zero_limits_never_divide_by_zero() {
        let timing = StepTiming {
            max_counts_per_step: 0,
            move_speed_multiplier: 0.0,
            ..timing()
        };

        assert_eq!(timing.step_count(5, 0, true), 5);
        assert_eq!(timing.step_count(5, 0, false), 5);
    }
}
