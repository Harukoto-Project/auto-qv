use std::thread;
use std::time::{Duration, Instant};

use super::timing::StepTiming;
use crate::error::Result;
use crate::input::pen_driver::PenDriver;
use crate::planner::action::Action;

const ABORT_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    Aborted,
}

pub fn execute(
    actions: &[Action],
    driver: &mut dyn PenDriver,
    timing: &StepTiming,
    should_abort: &mut dyn FnMut() -> bool,
    on_progress: &mut dyn FnMut(usize, usize),
) -> Result<RunOutcome> {
    let mut pen_down = false;
    let outcome = run(
        actions,
        driver,
        timing,
        should_abort,
        on_progress,
        &mut pen_down,
    );
    let release = if pen_down { driver.release() } else { Ok(()) };
    let outcome = outcome?;
    release?;
    Ok(outcome)
}

fn run(
    actions: &[Action],
    driver: &mut dyn PenDriver,
    timing: &StepTiming,
    should_abort: &mut dyn FnMut() -> bool,
    on_progress: &mut dyn FnMut(usize, usize),
    pen_down: &mut bool,
) -> Result<RunOutcome> {
    let mut pacer = Pacer::new(Duration::from_millis(u64::from(timing.step_delay_ms)));
    let min_pen_down_interval = Duration::from_millis(u64::from(timing.min_pen_down_interval_ms));
    let mut last_press: Option<Instant> = None;
    let total = actions.len();

    for (index, action) in actions.iter().enumerate() {
        if should_abort() {
            return Ok(RunOutcome::Aborted);
        }

        match *action {
            Action::PenDown => {
                if wait_for_pen_down_interval(last_press, min_pen_down_interval, should_abort) {
                    return Ok(RunOutcome::Aborted);
                }
                driver.press()?;
                last_press = Some(Instant::now());
                *pen_down = true;
                pacer.wait();
            }
            Action::PenUp => {
                driver.release()?;
                *pen_down = false;
                pacer.wait();
            }
            Action::Move { dx, dy } => {
                let steps = timing.step_count(dx, dy, *pen_down);
                let mut sent = (0, 0);
                for step in 1..=steps {
                    if should_abort() {
                        return Ok(RunOutcome::Aborted);
                    }
                    let target = (split(dx, step, steps), split(dy, step, steps));
                    let delta = (target.0 - sent.0, target.1 - sent.1);
                    sent = target;
                    if delta != (0, 0) {
                        driver.move_rel(delta.0, delta.1)?;
                        pacer.wait();
                    }
                }
            }
        }

        on_progress(index + 1, total);
    }

    Ok(RunOutcome::Completed)
}

fn wait_for_pen_down_interval(
    last_press: Option<Instant>,
    interval: Duration,
    should_abort: &mut dyn FnMut() -> bool,
) -> bool {
    let Some(last_press) = last_press else {
        return false;
    };
    let deadline = last_press + interval;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        if should_abort() {
            return true;
        }
        thread::sleep(
            deadline
                .saturating_duration_since(now)
                .min(ABORT_POLL_INTERVAL),
        );
    }
}

fn split(distance: i32, step: u32, steps: u32) -> i32 {
    (i64::from(distance) * i64::from(step) / i64::from(steps)) as i32
}

struct Pacer {
    interval: Duration,
    next: Option<Instant>,
}

impl Pacer {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            next: None,
        }
    }

    fn wait(&mut self) {
        if self.interval.is_zero() {
            return;
        }
        let now = Instant::now();
        let base = match self.next {
            Some(next) if now.saturating_duration_since(next) < self.interval => next,
            _ => now,
        };
        let deadline = base + self.interval;
        self.next = Some(deadline);
        thread::sleep(deadline.saturating_duration_since(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AutoQvError;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Call {
        Move(i32, i32),
        Press,
        Release,
    }

    #[derive(Default)]
    struct FakeDriver {
        calls: Vec<Call>,
        fail_on_move: bool,
    }

    impl PenDriver for FakeDriver {
        fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()> {
            if self.fail_on_move {
                return Err(AutoQvError::Input("boom".to_string()));
            }
            self.calls.push(Call::Move(dx, dy));
            Ok(())
        }

        fn press(&mut self) -> Result<()> {
            self.calls.push(Call::Press);
            Ok(())
        }

        fn release(&mut self) -> Result<()> {
            self.calls.push(Call::Release);
            Ok(())
        }
    }

    fn timing() -> StepTiming {
        StepTiming {
            step_delay_ms: 0,
            max_counts_per_step: 8,
            move_speed_multiplier: 3.0,
            min_pen_down_interval_ms: 0,
        }
    }

    fn run_all(actions: &[Action], driver: &mut FakeDriver) -> Result<RunOutcome> {
        execute(actions, driver, &timing(), &mut || false, &mut |_, _| {})
    }

    fn total_move(calls: &[Call]) -> (i32, i32) {
        calls.iter().fold((0, 0), |acc, call| match call {
            Call::Move(dx, dy) => (acc.0 + dx, acc.1 + dy),
            _ => acc,
        })
    }

    #[test]
    fn drawing_moves_are_split_into_bounded_steps() {
        let mut driver = FakeDriver::default();
        let actions = [
            Action::PenDown,
            Action::Move { dx: 20, dy: -5 },
            Action::PenUp,
        ];

        let outcome = run_all(&actions, &mut driver).unwrap();

        assert_eq!(outcome, RunOutcome::Completed);
        let moves: Vec<&Call> = driver
            .calls
            .iter()
            .filter(|call| matches!(call, Call::Move(..)))
            .collect();
        assert_eq!(moves.len(), 3);
        for call in moves {
            let Call::Move(dx, dy) = call else {
                unreachable!()
            };
            assert!(dx.abs() <= 8 && dy.abs() <= 8, "{call:?}");
        }
        assert_eq!(total_move(&driver.calls), (20, -5));
        assert_eq!(driver.calls.first(), Some(&Call::Press));
        assert_eq!(driver.calls.last(), Some(&Call::Release));
    }

    #[test]
    fn pen_up_moves_use_larger_steps() {
        let mut driver = FakeDriver::default();

        run_all(&[Action::Move { dx: 48, dy: 0 }], &mut driver).unwrap();

        assert_eq!(driver.calls, vec![Call::Move(24, 0), Call::Move(24, 0)]);
    }

    #[test]
    fn split_moves_always_sum_to_the_requested_distance() {
        for (dx, dy) in [(37, -91), (-1, 1), (1000, 3), (0, -77)] {
            let mut driver = FakeDriver::default();
            run_all(&[Action::PenDown, Action::Move { dx, dy }], &mut driver).unwrap();

            assert_eq!(total_move(&driver.calls), (dx, dy));
        }
    }

    #[test]
    fn zero_length_moves_send_nothing() {
        let mut driver = FakeDriver::default();

        run_all(&[Action::Move { dx: 0, dy: 0 }], &mut driver).unwrap();

        assert!(driver.calls.is_empty());
    }

    #[test]
    fn abort_before_start_sends_nothing() {
        let mut driver = FakeDriver::default();

        let outcome = execute(
            &[Action::PenDown, Action::Move { dx: 10, dy: 0 }],
            &mut driver,
            &timing(),
            &mut || true,
            &mut |_, _| {},
        )
        .unwrap();

        assert_eq!(outcome, RunOutcome::Aborted);
        assert!(driver.calls.is_empty());
    }

    #[test]
    fn abort_during_a_stroke_releases_the_pen() {
        let mut driver = FakeDriver::default();
        let mut polls = 0;

        let outcome = execute(
            &[Action::PenDown, Action::Move { dx: 100, dy: 0 }],
            &mut driver,
            &timing(),
            &mut || {
                polls += 1;
                polls > 4
            },
            &mut |_, _| {},
        )
        .unwrap();

        assert_eq!(outcome, RunOutcome::Aborted);
        assert_eq!(driver.calls.first(), Some(&Call::Press));
        assert_eq!(driver.calls.last(), Some(&Call::Release));
        assert!(total_move(&driver.calls).0 < 100);
    }

    #[test]
    fn pen_left_down_at_the_end_is_released() {
        let mut driver = FakeDriver::default();

        run_all(
            &[Action::PenDown, Action::Move { dx: 4, dy: 0 }],
            &mut driver,
        )
        .unwrap();

        assert_eq!(driver.calls.last(), Some(&Call::Release));
    }

    #[test]
    fn driver_error_still_releases_the_pen_and_propagates() {
        let mut driver = FakeDriver {
            fail_on_move: true,
            ..FakeDriver::default()
        };

        let result = run_all(
            &[Action::PenDown, Action::Move { dx: 4, dy: 0 }],
            &mut driver,
        );

        assert!(result.is_err());
        assert_eq!(driver.calls, vec![Call::Press, Call::Release]);
    }

    #[test]
    fn reports_progress_after_each_action() {
        let mut driver = FakeDriver::default();
        let mut progress = Vec::new();

        execute(
            &[
                Action::PenDown,
                Action::Move { dx: 4, dy: 0 },
                Action::PenUp,
            ],
            &mut driver,
            &timing(),
            &mut || false,
            &mut |done, total| progress.push((done, total)),
        )
        .unwrap();

        assert_eq!(progress, vec![(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn consecutive_pen_downs_are_spaced_by_the_minimum_interval() {
        let mut driver = FakeDriver::default();
        let timing = StepTiming {
            min_pen_down_interval_ms: 60,
            ..timing()
        };
        let start = Instant::now();

        let outcome = execute(
            &[
                Action::PenDown,
                Action::PenUp,
                Action::PenDown,
                Action::PenUp,
            ],
            &mut driver,
            &timing,
            &mut || false,
            &mut |_, _| {},
        )
        .unwrap();

        assert_eq!(outcome, RunOutcome::Completed);
        assert!(start.elapsed() >= Duration::from_millis(55));
        assert_eq!(
            driver.calls,
            vec![Call::Press, Call::Release, Call::Press, Call::Release]
        );
    }

    #[test]
    fn first_pen_down_does_not_wait() {
        let mut driver = FakeDriver::default();
        let timing = StepTiming {
            min_pen_down_interval_ms: 1000,
            ..timing()
        };
        let start = Instant::now();

        execute(
            &[Action::PenDown, Action::PenUp],
            &mut driver,
            &timing,
            &mut || false,
            &mut |_, _| {},
        )
        .unwrap();

        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn abort_while_waiting_for_the_interval_skips_the_next_press() {
        let mut driver = FakeDriver::default();
        let timing = StepTiming {
            min_pen_down_interval_ms: 5000,
            ..timing()
        };
        let mut polls = 0;
        let start = Instant::now();

        let outcome = execute(
            &[
                Action::PenDown,
                Action::PenUp,
                Action::PenDown,
                Action::PenUp,
            ],
            &mut driver,
            &timing,
            &mut || {
                polls += 1;
                polls > 3
            },
            &mut |_, _| {},
        )
        .unwrap();

        assert_eq!(outcome, RunOutcome::Aborted);
        assert_eq!(driver.calls, vec![Call::Press, Call::Release]);
        assert!(start.elapsed() < Duration::from_millis(2000));
    }

    #[test]
    fn pacer_waits_roughly_one_interval_per_call() {
        let mut pacer = Pacer::new(Duration::from_millis(20));
        let start = Instant::now();

        pacer.wait();
        pacer.wait();

        assert!(start.elapsed() >= Duration::from_millis(35));
    }
}
