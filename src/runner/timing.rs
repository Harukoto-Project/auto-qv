use std::time::Duration;

use crate::planner::action::Action;

pub const DEFAULT_MIN_PEN_DOWN_INTERVAL_MS: u32 = 230;
pub const DEFAULT_PEN_DOWN_SETTLE_MS: u32 = 30;
pub const DEFAULT_PEN_DOWN_DELAY_MS: u32 = 30;
pub const DEFAULT_PEN_UP_DELAY_MS: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepTiming {
    pub step_delay_ms: u32,
    pub max_counts_per_step: u32,
    pub move_speed_multiplier: f32,
    pub min_pen_down_interval_ms: u32,
    pub pen_down_settle_ms: u32,
    pub pen_down_delay_ms: u32,
    pub pen_up_delay_ms: u32,
}

impl Default for StepTiming {
    fn default() -> Self {
        Self {
            step_delay_ms: 4,
            max_counts_per_step: 4,
            move_speed_multiplier: 3.0,
            min_pen_down_interval_ms: DEFAULT_MIN_PEN_DOWN_INTERVAL_MS,
            pen_down_settle_ms: DEFAULT_PEN_DOWN_SETTLE_MS,
            pen_down_delay_ms: DEFAULT_PEN_DOWN_DELAY_MS,
            pen_up_delay_ms: DEFAULT_PEN_UP_DELAY_MS,
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
    let delay = u64::from(timing.step_delay_ms);
    let interval = u64::from(timing.min_pen_down_interval_ms);
    let pen_down_settle = u64::from(timing.pen_down_settle_ms);
    let pen_down_delay = u64::from(timing.pen_down_delay_ms);
    let pen_up_delay = u64::from(timing.pen_up_delay_ms);
    let mut pen_down = false;
    let mut elapsed: u64 = 0;
    let mut last_press: Option<u64> = None;

    for action in actions {
        match action {
            Action::PenDown => {
                elapsed += pen_down_settle;
                if let Some(last) = last_press {
                    elapsed = elapsed.max(last + interval);
                }
                last_press = Some(elapsed);
                pen_down = true;
                elapsed += delay + pen_down_delay;
            }
            Action::PenUp => {
                pen_down = false;
                elapsed += delay + pen_up_delay;
            }
            Action::Move { dx, dy } => {
                elapsed += u64::from(timing.step_count(*dx, *dy, pen_down)) * delay;
            }
        }
    }
    Duration::from_millis(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> StepTiming {
        StepTiming {
            step_delay_ms: 2,
            max_counts_per_step: 8,
            move_speed_multiplier: 3.0,
            min_pen_down_interval_ms: 0,
            pen_down_settle_ms: 0,
            pen_down_delay_ms: 0,
            pen_up_delay_ms: 0,
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
    fn estimates_duration_from_steps_pen_state_and_button_waits() {
        let actions = [
            Action::Move { dx: 48, dy: 0 },
            Action::PenDown,
            Action::Move { dx: 16, dy: 0 },
            Action::PenUp,
        ];

        assert_eq!(
            estimate_duration(&actions, &timing()),
            Duration::from_millis((2 + 1 + 2 + 1) * 2)
        );
    }

    #[test]
    fn estimate_includes_the_pen_down_and_pen_up_delays() {
        let timing = StepTiming {
            pen_down_delay_ms: 30,
            pen_up_delay_ms: 20,
            ..timing()
        };
        let actions = [Action::PenDown, Action::PenUp];

        assert_eq!(
            estimate_duration(&actions, &timing),
            Duration::from_millis(2 + 30 + 2 + 20)
        );
    }

    #[test]
    fn estimate_includes_the_pen_down_settle_wait() {
        let timing = StepTiming {
            pen_down_settle_ms: 30,
            ..timing()
        };

        assert_eq!(
            estimate_duration(&[Action::PenDown], &timing),
            Duration::from_millis(30 + 2)
        );
    }

    #[test]
    fn estimate_waits_for_the_minimum_interval_between_pen_downs() {
        let timing = StepTiming {
            min_pen_down_interval_ms: 230,
            ..timing()
        };
        let actions = [
            Action::PenDown,
            Action::Move { dx: 16, dy: 0 },
            Action::PenUp,
            Action::PenDown,
            Action::PenUp,
        ];

        assert_eq!(
            estimate_duration(&actions, &timing),
            Duration::from_millis(230 + 2 + 2)
        );
    }

    #[test]
    fn estimate_does_not_wait_when_strokes_are_already_far_apart_in_time() {
        let timing = StepTiming {
            min_pen_down_interval_ms: 10,
            ..timing()
        };
        let actions = [
            Action::PenDown,
            Action::Move { dx: 80, dy: 0 },
            Action::PenUp,
            Action::PenDown,
            Action::PenUp,
        ];

        let without_interval = estimate_duration(
            &actions,
            &StepTiming {
                min_pen_down_interval_ms: 0,
                ..timing
            },
        );

        assert_eq!(estimate_duration(&actions, &timing), without_interval);
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
