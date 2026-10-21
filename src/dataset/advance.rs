use crate::glyph::model::Glyph;

pub(super) const MAX_ADVANCE: f32 = 1.0;
const SIDE_BEARING: f32 = 0.125;

pub(super) fn fit_advance_to_ink(mut glyph: Glyph) -> Glyph {
    let (min_x, max_x) = glyph
        .strokes
        .iter()
        .flatten()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
    let width = max_x - min_x;
    let advance = (width + 2.0 * SIDE_BEARING).min(MAX_ADVANCE);
    let shift = (advance - width) / 2.0 - min_x;
    for point in glyph.strokes.iter_mut().flatten() {
        point.x += shift;
    }
    glyph.advance = advance;
    glyph
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::model::Point;

    fn glyph(xs: &[f32]) -> Glyph {
        Glyph {
            advance: MAX_ADVANCE,
            strokes: vec![xs.iter().map(|x| Point { x: *x, y: 0.5 }).collect()],
        }
    }

    #[test]
    fn narrow_ink_gets_side_bearings_on_both_sides() {
        let fitted = fit_advance_to_ink(glyph(&[0.4, 0.6]));

        assert!((fitted.advance - 0.45).abs() < 1e-6);
        let xs: Vec<f32> = fitted.strokes[0].iter().map(|p| p.x).collect();
        assert!((xs[0] - 0.125).abs() < 1e-6);
        assert!((fitted.advance - xs[1] - 0.125).abs() < 1e-6);
    }

    #[test]
    fn wide_ink_is_capped_and_centered() {
        let fitted = fit_advance_to_ink(glyph(&[0.0, 0.9]));

        assert_eq!(fitted.advance, MAX_ADVANCE);
        let xs: Vec<f32> = fitted.strokes[0].iter().map(|p| p.x).collect();
        assert!((xs[0] - (MAX_ADVANCE - xs[1])).abs() < 1e-6);
    }
}
