use super::action::Action;
use crate::glyph::model::{Point, Stroke};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannerConfig {
    pub counts_per_em_x: f32,
    pub counts_per_em_y: f32,
}

type Counts = (i32, i32);

pub fn plan(strokes: &[Stroke], config: &PlannerConfig) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut current: Counts = (0, 0);

    for stroke in strokes {
        let Some((first, rest)) = stroke.split_first() else {
            continue;
        };

        move_to(&mut actions, &mut current, to_counts(first, config));
        actions.push(Action::PenDown);
        for point in rest {
            move_to(&mut actions, &mut current, to_counts(point, config));
        }
        actions.push(Action::PenUp);
    }

    move_to(&mut actions, &mut current, (0, 0));
    actions
}

fn to_counts(point: &Point, config: &PlannerConfig) -> Counts {
    (
        (f64::from(point.x) * f64::from(config.counts_per_em_x)).round() as i32,
        (f64::from(point.y) * f64::from(config.counts_per_em_y)).round() as i32,
    )
}

fn move_to(actions: &mut Vec<Action>, current: &mut Counts, target: Counts) {
    let dx = target.0 - current.0;
    let dy = target.1 - current.1;
    if dx != 0 || dy != 0 {
        actions.push(Action::Move { dx, dy });
    }
    *current = target;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PlannerConfig {
        PlannerConfig {
            counts_per_em_x: 100.0,
            counts_per_em_y: 50.0,
        }
    }

    fn stroke(points: &[(f32, f32)]) -> Stroke {
        points.iter().map(|&(x, y)| Point { x, y }).collect()
    }

    fn total_move(actions: &[Action]) -> (i32, i32) {
        actions.iter().fold((0, 0), |acc, action| match action {
            Action::Move { dx, dy } => (acc.0 + dx, acc.1 + dy),
            _ => acc,
        })
    }

    #[test]
    fn empty_input_produces_no_actions() {
        assert!(plan(&[], &config()).is_empty());
    }

    #[test]
    fn plans_single_stroke_and_returns_to_origin() {
        let actions = plan(&[stroke(&[(1.0, 1.0), (2.0, 1.0)])], &config());

        assert_eq!(
            actions,
            vec![
                Action::Move { dx: 100, dy: 50 },
                Action::PenDown,
                Action::Move { dx: 100, dy: 0 },
                Action::PenUp,
                Action::Move { dx: -200, dy: -50 },
            ]
        );
    }

    #[test]
    fn moves_between_strokes_with_pen_up() {
        let actions = plan(
            &[
                stroke(&[(0.0, 0.0), (1.0, 0.0)]),
                stroke(&[(0.0, 1.0), (1.0, 1.0)]),
            ],
            &config(),
        );

        assert_eq!(
            actions,
            vec![
                Action::PenDown,
                Action::Move { dx: 100, dy: 0 },
                Action::PenUp,
                Action::Move { dx: -100, dy: 50 },
                Action::PenDown,
                Action::Move { dx: 100, dy: 0 },
                Action::PenUp,
                Action::Move { dx: -100, dy: -50 },
            ]
        );
    }

    #[test]
    fn single_point_stroke_is_a_dot() {
        let actions = plan(&[stroke(&[(0.0, 0.0)])], &config());

        assert_eq!(actions, vec![Action::PenDown, Action::PenUp]);
    }

    #[test]
    fn skips_empty_strokes() {
        let actions = plan(&[Vec::new()], &config());

        assert!(actions.is_empty());
    }

    #[test]
    fn rounding_error_does_not_accumulate() {
        let points: Vec<(f32, f32)> = (0..=10).map(|i| (i as f32 * 0.004, 0.0)).collect();
        let actions = plan(&[stroke(&points)], &config());

        let drawn: i32 = actions
            .iter()
            .take_while(|action| **action != Action::PenUp)
            .map(|action| match action {
                Action::Move { dx, .. } => *dx,
                _ => 0,
            })
            .sum();
        assert_eq!(drawn, 4);
    }

    #[test]
    fn net_movement_is_always_zero() {
        let actions = plan(
            &[
                stroke(&[(0.33, 0.77), (1.41, 0.12), (0.5, 2.9)]),
                stroke(&[(3.3, 3.3), (0.1, 0.2)]),
            ],
            &config(),
        );

        assert_eq!(total_move(&actions), (0, 0));
    }
}
