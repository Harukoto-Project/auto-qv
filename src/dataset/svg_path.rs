pub type Pt = (f32, f32);

const FLATTEN_STEP: f32 = 2.0;
const MAX_SEGMENTS: usize = 32;
const SAME_POINT_EPSILON_SQ: f32 = 1e-8;

pub fn flatten(d: &str) -> Result<Vec<Vec<Pt>>, String> {
    let mut walker = Walker::default();
    let mut scanner = Scanner::new(d);
    let mut command: Option<u8> = None;

    loop {
        scanner.skip_separators();
        let Some(next) = scanner.peek() else {
            break;
        };
        let current = if next.is_ascii_alphabetic() {
            scanner.advance();
            if matches!(next, b'Z' | b'z') {
                walker.close();
                command = None;
                continue;
            }
            next
        } else {
            command.ok_or_else(|| "expected a path command".to_string())?
        };
        command = walker.apply(current, &mut scanner)?;
    }

    Ok(walker.finish())
}

#[derive(Default)]
struct Walker {
    subpaths: Vec<Vec<Pt>>,
    current: Vec<Pt>,
    position: Pt,
    start: Pt,
    last_control: Option<Control>,
}

#[derive(Clone, Copy)]
enum Control {
    Cubic(Pt),
    Quad(Pt),
}

impl Walker {
    fn apply(&mut self, command: u8, scanner: &mut Scanner) -> Result<Option<u8>, String> {
        let relative = command.is_ascii_lowercase();
        let origin = if relative { self.position } else { (0.0, 0.0) };
        let mut next_command = Some(command);
        let mut control = None;

        match command.to_ascii_uppercase() {
            b'M' => {
                let target = add(origin, scanner.pair()?);
                self.move_to(target);
                next_command = Some(if relative { b'l' } else { b'L' });
            }
            b'L' => {
                let target = add(origin, scanner.pair()?);
                self.line_to(target);
            }
            b'H' => {
                let x = scanner.number()? + origin.0;
                self.line_to((x, self.position.1));
            }
            b'V' => {
                let y = scanner.number()? + origin.1;
                self.line_to((self.position.0, y));
            }
            b'C' => {
                let c1 = add(origin, scanner.pair()?);
                let c2 = add(origin, scanner.pair()?);
                let end = add(origin, scanner.pair()?);
                self.cubic_to(c1, c2, end);
                control = Some(Control::Cubic(c2));
            }
            b'S' => {
                let c1 = match self.last_control {
                    Some(Control::Cubic(previous)) => reflect(previous, self.position),
                    _ => self.position,
                };
                let c2 = add(origin, scanner.pair()?);
                let end = add(origin, scanner.pair()?);
                self.cubic_to(c1, c2, end);
                control = Some(Control::Cubic(c2));
            }
            b'Q' => {
                let c = add(origin, scanner.pair()?);
                let end = add(origin, scanner.pair()?);
                self.quad_to(c, end);
                control = Some(Control::Quad(c));
            }
            b'T' => {
                let c = match self.last_control {
                    Some(Control::Quad(previous)) => reflect(previous, self.position),
                    _ => self.position,
                };
                let end = add(origin, scanner.pair()?);
                self.quad_to(c, end);
                control = Some(Control::Quad(c));
            }
            other => {
                return Err(format!("unsupported path command '{}'", char::from(other)));
            }
        }

        self.last_control = control;
        scanner.skip_separators();
        if !scanner.at_number() {
            next_command = None;
        }
        Ok(next_command)
    }

    fn move_to(&mut self, target: Pt) {
        self.flush();
        self.position = target;
        self.start = target;
        self.current.push(target);
    }

    fn line_to(&mut self, target: Pt) {
        self.begin_if_needed();
        self.push(target);
        self.position = target;
    }

    fn cubic_to(&mut self, c1: Pt, c2: Pt, end: Pt) {
        self.begin_if_needed();
        let p0 = self.position;
        let length = distance(p0, c1) + distance(c1, c2) + distance(c2, end);
        let segments = segment_count(length);
        for index in 1..=segments {
            let t = index as f32 / segments as f32;
            let u = 1.0 - t;
            let x = u * u * u * p0.0
                + 3.0 * u * u * t * c1.0
                + 3.0 * u * t * t * c2.0
                + t * t * t * end.0;
            let y = u * u * u * p0.1
                + 3.0 * u * u * t * c1.1
                + 3.0 * u * t * t * c2.1
                + t * t * t * end.1;
            self.push(if index == segments { end } else { (x, y) });
        }
        self.position = end;
    }

    fn quad_to(&mut self, control: Pt, end: Pt) {
        self.begin_if_needed();
        let p0 = self.position;
        let length = distance(p0, control) + distance(control, end);
        let segments = segment_count(length);
        for index in 1..=segments {
            let t = index as f32 / segments as f32;
            let u = 1.0 - t;
            let x = u * u * p0.0 + 2.0 * u * t * control.0 + t * t * end.0;
            let y = u * u * p0.1 + 2.0 * u * t * control.1 + t * t * end.1;
            self.push(if index == segments { end } else { (x, y) });
        }
        self.position = end;
    }

    fn close(&mut self) {
        if !self.current.is_empty() {
            self.push(self.start);
        }
        self.flush();
        self.position = self.start;
        self.last_control = None;
    }

    fn begin_if_needed(&mut self) {
        if self.current.is_empty() {
            self.current.push(self.position);
        }
    }

    fn push(&mut self, point: Pt) {
        let duplicate = self
            .current
            .last()
            .is_some_and(|last| distance_sq(*last, point) < SAME_POINT_EPSILON_SQ);
        if !duplicate {
            self.current.push(point);
        }
    }

    fn flush(&mut self) {
        if !self.current.is_empty() {
            self.subpaths.push(std::mem::take(&mut self.current));
        }
    }

    fn finish(mut self) -> Vec<Vec<Pt>> {
        self.flush();
        self.subpaths
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n' | b',')) {
            self.advance();
        }
    }

    fn at_number(&self) -> bool {
        matches!(self.peek(), Some(b'0'..=b'9' | b'+' | b'-' | b'.'))
    }

    fn pair(&mut self) -> Result<Pt, String> {
        let x = self.number()?;
        let y = self.number()?;
        Ok((x, y))
    }

    fn number(&mut self) -> Result<f32, String> {
        self.skip_separators();
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.advance();
        }
        let mut digits = 0;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
            digits += 1;
        }
        if self.peek() == Some(b'.') {
            self.advance();
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
                digits += 1;
            }
        }
        if digits == 0 {
            return Err(format!("expected a number at offset {start}"));
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mark = self.pos;
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            let exponent_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
            if self.pos == exponent_start {
                self.pos = mark;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "invalid number".to_string())?;
        let value: f32 = text
            .parse()
            .map_err(|_| format!("invalid number {text:?}"))?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(format!("number out of range: {text:?}"))
        }
    }
}

fn add(a: Pt, b: Pt) -> Pt {
    (a.0 + b.0, a.1 + b.1)
}

fn reflect(control: Pt, around: Pt) -> Pt {
    (2.0 * around.0 - control.0, 2.0 * around.1 - control.1)
}

fn distance_sq(a: Pt, b: Pt) -> f32 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    dx * dx + dy * dy
}

fn distance(a: Pt, b: Pt) -> f32 {
    distance_sq(a, b).sqrt()
}

fn segment_count(length: f32) -> usize {
    ((length / FLATTEN_STEP).ceil() as usize).clamp(1, MAX_SEGMENTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single(d: &str) -> Vec<Pt> {
        let mut paths = flatten(d).unwrap();
        assert_eq!(paths.len(), 1, "{d}");
        paths.remove(0)
    }

    #[test]
    fn absolute_and_relative_lines() {
        assert_eq!(
            single("M10,20 L30,20 l0,10 H5 h-5 V0 v4"),
            vec![
                (10.0, 20.0),
                (30.0, 20.0),
                (30.0, 30.0),
                (5.0, 30.0),
                (0.0, 30.0),
                (0.0, 0.0),
                (0.0, 4.0)
            ]
        );
    }

    #[test]
    fn implicit_repeat_after_move_is_a_line() {
        assert_eq!(
            single("M0,0 10,0 10,10"),
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        );
        assert_eq!(
            single("m1,1 2,0 0,2"),
            vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0)]
        );
    }

    #[test]
    fn numbers_without_separators_are_split() {
        assert_eq!(single("M0.5.5L-.5-.25"), vec![(0.5, 0.5), (-0.5, -0.25)]);
        assert_eq!(single("M1-2L3-4"), vec![(1.0, -2.0), (3.0, -4.0)]);
    }

    #[test]
    fn exponents_are_understood() {
        assert_eq!(single("M1e1,2E0L0,0"), vec![(10.0, 2.0), (0.0, 0.0)]);
    }

    #[test]
    fn cubic_curve_ends_exactly_at_the_target() {
        let points = single("M0,0c10,0,20,0,30,0");

        assert_eq!(points.first(), Some(&(0.0, 0.0)));
        assert_eq!(points.last(), Some(&(30.0, 0.0)));
        assert!(points.len() > 2);
        assert!(points.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn relative_cubic_uses_the_current_point_as_origin() {
        let points = single("M10,10c0,10,0,20,0,30");

        assert_eq!(points.last(), Some(&(10.0, 40.0)));
    }

    #[test]
    fn repeated_cubic_groups_continue_from_the_previous_end() {
        let points = single("M0,0c1,0,2,0,3,0 1,0,2,0,3,0");

        assert_eq!(points.last(), Some(&(6.0, 0.0)));
    }

    #[test]
    fn smooth_cubic_reflects_the_previous_control_point() {
        let smooth = single("M0,0c0,10,10,10,10,0s10,-10,10,0");
        let explicit = single("M0,0c0,10,10,10,10,0c0,-10,10,-10,10,0");

        assert_eq!(smooth.len(), explicit.len());
        for (a, b) in smooth.iter().zip(&explicit) {
            assert!((a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4);
        }
    }

    #[test]
    fn smooth_cubic_without_a_previous_curve_uses_the_current_point() {
        let smooth = single("M0,0s10,10,10,0");
        let explicit = single("M0,0c0,0,10,10,10,0");

        assert_eq!(smooth, explicit);
    }

    #[test]
    fn quadratic_curves_are_flattened() {
        let points = single("M0,0q10,10,20,0");

        assert_eq!(points.last(), Some(&(20.0, 0.0)));
        assert!(points.len() > 2);
        assert!(points.iter().any(|point| point.1 > 4.0));
    }

    #[test]
    fn close_returns_to_the_start() {
        let points = single("M0,0L10,0L10,10z");

        assert_eq!(points.last(), Some(&(0.0, 0.0)));
    }

    #[test]
    fn multiple_subpaths_are_separate_polylines() {
        let paths = flatten("M0,0L1,1M5,5L6,6").unwrap();

        assert_eq!(
            paths,
            vec![vec![(0.0, 0.0), (1.0, 1.0)], vec![(5.0, 5.0), (6.0, 6.0)]]
        );
    }

    #[test]
    fn a_lone_move_is_a_single_point() {
        assert_eq!(single("M3,4"), vec![(3.0, 4.0)]);
    }

    #[test]
    fn consecutive_duplicate_points_are_dropped() {
        assert_eq!(single("M0,0L0,0L1,1L1,1"), vec![(0.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn arcs_are_rejected() {
        assert!(flatten("M0,0A5,5,0,0,1,10,0").is_err());
    }

    #[test]
    fn malformed_data_is_rejected() {
        assert!(flatten("10,10").is_err());
        assert!(flatten("M10").is_err());
        assert!(flatten("M10,x").is_err());
    }

    #[test]
    fn empty_data_has_no_subpaths() {
        assert!(flatten("").unwrap().is_empty());
    }
}
