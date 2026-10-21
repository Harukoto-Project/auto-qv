use super::advance::{MAX_ADVANCE, fit_advance_to_ink};
use super::charset::is_hangul;
use super::svg_attr::attribute;
use super::svg_path::flatten_with_step;
use crate::glyph::model::{Glyph, Point, Stroke};
use crate::glyph::store::GlyphStore;

const UNITS_PER_EM: f32 = 1000.0;
const UNITS_PER_EM_ATTRIBUTE: &str = "units-per-em=\"1000\"";
const FLATTEN_STEP: f32 = 18.0;

pub fn convert_font(svg: &str) -> Result<GlyphStore, String> {
    if !svg.contains(UNITS_PER_EM_ATTRIBUTE) {
        return Err("unexpected font scale (expected 1000 units per em)".to_string());
    }

    let mut store = GlyphStore::default();
    for (ch, data) in extract_glyphs(svg) {
        if !is_hangul(ch) {
            continue;
        }
        let glyph =
            build_glyph(data).map_err(|message| format!("U+{:04X}: {message}", ch as u32))?;
        store
            .insert(ch, fit_advance_to_ink(glyph))
            .map_err(|error| error.to_string())?;
    }

    if store.is_empty() {
        return Err("no Hangul glyphs found".to_string());
    }
    Ok(store)
}

fn extract_glyphs(svg: &str) -> Vec<(char, &str)> {
    let mut glyphs = Vec::new();
    let mut rest = svg;
    while let Some(found) = rest.find("<glyph ") {
        let after = &rest[found + "<glyph".len()..];
        let Some(end) = after.find('>') else {
            break;
        };
        let tag = &after[..end];
        rest = &after[end..];

        let Some(ch) = attribute(tag, "unicode").and_then(parse_unicode) else {
            continue;
        };
        let Some(data) = attribute(tag, "d") else {
            continue;
        };
        glyphs.push((ch, data));
    }
    glyphs
}

fn parse_unicode(value: &str) -> Option<char> {
    let code = if let Some(hex) = value
        .strip_prefix("&#x")
        .or_else(|| value.strip_prefix("&#X"))
    {
        u32::from_str_radix(hex.strip_suffix(';')?, 16).ok()?
    } else if let Some(decimal) = value.strip_prefix("&#") {
        decimal.strip_suffix(';')?.parse().ok()?
    } else {
        let mut chars = value.chars();
        return match (chars.next(), chars.next()) {
            (Some(ch), None) => Some(ch),
            _ => None,
        };
    };
    char::from_u32(code)
}

fn build_glyph(data: &str) -> Result<Glyph, String> {
    let strokes: Vec<Stroke> = flatten_with_step(data, FLATTEN_STEP)?
        .into_iter()
        .map(|polyline| {
            polyline
                .into_iter()
                .map(|(x, y)| Point {
                    x: x / UNITS_PER_EM,
                    y: (UNITS_PER_EM - y) / UNITS_PER_EM,
                })
                .collect()
        })
        .collect();

    let glyph = Glyph {
        advance: MAX_ADVANCE,
        strokes,
    };
    if glyph.strokes.is_empty() || !glyph.is_valid() {
        return Err("the converted glyph is invalid".to_string());
    }
    Ok(glyph)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font_with(glyphs: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" version="1.1"><defs><font id="f" horiz-adv-x="1000"><font-face font-family="F" units-per-em="1000" ascent="900" descent="-100" bbox="0 0 1000 1000" /><missing-glyph />{glyphs}</font></defs></svg>"#
        )
    }

    const GIYEOK: &str = r#"<glyph glyph-name="uni3131" unicode="&#x3131;" horiz-adv-x="1000" d="M70 837.857 L930 837.857 L930 162.143" />"#;

    #[test]
    fn converts_a_compatibility_jamo_and_flips_the_y_axis() {
        let store = convert_font(&font_with(GIYEOK)).unwrap();

        let glyph = store.get('ㄱ').unwrap();
        assert_eq!(glyph.strokes.len(), 1);
        let stroke = &glyph.strokes[0];
        assert_eq!(stroke.len(), 3);
        assert!((stroke[0].y - 0.162143).abs() < 1e-5);
        assert!((stroke[2].y - 0.837857).abs() < 1e-5);
        assert!(stroke[0].y < stroke[2].y);
    }

    #[test]
    fn the_advance_follows_the_ink_and_the_glyph_stays_centered() {
        let store = convert_font(&font_with(GIYEOK)).unwrap();

        let glyph = store.get('ㄱ').unwrap();
        assert_eq!(glyph.advance, 1.0);
        let xs: Vec<f32> = glyph.strokes.iter().flatten().map(|p| p.x).collect();
        let min = xs.iter().copied().fold(f32::MAX, f32::min);
        let max = xs.iter().copied().fold(f32::MIN, f32::max);
        assert!((min - (glyph.advance - max)).abs() < 1e-5);
    }

    #[test]
    fn narrow_glyphs_get_a_narrow_advance() {
        let vertical =
            r#"<glyph unicode="&#x3163;" horiz-adv-x="1000" d="M496.261 930 L496.261 70" />"#;

        let store = convert_font(&font_with(vertical)).unwrap();

        assert!((store.get('ㅣ').unwrap().advance - 0.25).abs() < 1e-5);
    }

    #[test]
    fn each_subpath_is_a_separate_stroke_in_font_order() {
        let glyph = r#"<glyph unicode="&#xAC00;" horiz-adv-x="1000" d="M768.75 899.766 L768.75 100.234 M123.75 772.109 L513.438 772.109" />"#;

        let store = convert_font(&font_with(glyph)).unwrap();

        let strokes = &store.get('가').unwrap().strokes;
        assert_eq!(strokes.len(), 2);
        assert!(strokes[0][0].x > strokes[1][0].x);
    }

    #[test]
    fn curves_are_flattened_into_polylines() {
        let glyph = r#"<glyph unicode="&#xD7A3;" horiz-adv-x="1000" d="M100 100 C100 500 900 500 900 100" />"#;

        let store = convert_font(&font_with(glyph)).unwrap();

        let stroke = &store.get('힣').unwrap().strokes[0];
        assert!(stroke.len() > 3);
        assert!(stroke.iter().all(|p| (0.0..=1.0).contains(&p.y)));
    }

    #[test]
    fn a_single_point_stroke_is_kept_as_a_dot() {
        let glyph =
            r#"<glyph unicode="&#xAC01;" horiz-adv-x="1000" d="M500 500 M100 100 L200 100" />"#;

        let store = convert_font(&font_with(glyph)).unwrap();

        let strokes = &store.get('각').unwrap().strokes;
        assert_eq!(strokes.len(), 2);
        assert_eq!(strokes[0].len(), 1);
    }

    #[test]
    fn glyphs_outside_the_hangul_ranges_are_ignored() {
        let latin = r#"<glyph unicode="&#x0041;" horiz-adv-x="600" d="M0 0 L300 900" />"#;
        let literal = r#"<glyph unicode="あ" horiz-adv-x="1000" d="M0 0 L1 1" />"#;

        let store = convert_font(&font_with(&format!("{latin}{literal}{GIYEOK}"))).unwrap();

        assert_eq!(store.len(), 1);
        assert!(store.get('A').is_none());
    }

    #[test]
    fn decimal_entities_and_literal_characters_are_understood() {
        assert_eq!(parse_unicode("&#12593;"), Some('ㄱ'));
        assert_eq!(parse_unicode("&#x3131;"), Some('ㄱ'));
        assert_eq!(parse_unicode("&#X3131;"), Some('ㄱ'));
        assert_eq!(parse_unicode("ㄱ"), Some('ㄱ'));
        assert_eq!(parse_unicode("ab"), None);
        assert_eq!(parse_unicode("&#xZZ;"), None);
        assert_eq!(parse_unicode("&#x3131"), None);
        assert_eq!(parse_unicode(""), None);
    }

    #[test]
    fn rejects_an_unexpected_font_scale() {
        let svg = format!(r#"<font-face units-per-em="2048" />{GIYEOK}"#);

        assert!(convert_font(&svg).unwrap_err().contains("scale"));
    }

    #[test]
    fn rejects_a_font_without_hangul_glyphs() {
        let latin = r#"<glyph unicode="&#x0041;" horiz-adv-x="600" d="M0 0 L300 900" />"#;

        assert!(convert_font(&font_with(latin)).is_err());
        assert!(convert_font("<html>404</html>").is_err());
    }

    #[test]
    fn a_hangul_glyph_without_strokes_names_the_code_point() {
        let empty = r#"<glyph unicode="&#xAC00;" horiz-adv-x="1000" d="" />"#;

        let error = convert_font(&font_with(empty)).unwrap_err();

        assert!(error.starts_with("U+AC00:"), "{error}");
    }

    #[test]
    fn an_unsupported_path_command_is_an_error() {
        let arc = r#"<glyph unicode="&#xAC00;" horiz-adv-x="1000" d="M0 0 A5 5 0 0 1 10 0" />"#;

        assert!(
            convert_font(&font_with(arc))
                .unwrap_err()
                .contains("unsupported")
        );
    }
}
