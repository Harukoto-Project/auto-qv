use super::svg_path::flatten;
use crate::glyph::model::{Glyph, Point, Stroke};

const CANVAS_SIZE: f32 = 109.0;
const CANVAS_VIEW_BOX: &str = "viewBox=\"0 0 109 109\"";
const MAX_ADVANCE: f32 = 1.0;
const SIDE_BEARING: f32 = 0.125;

pub fn convert_svg(svg: &str) -> Result<Glyph, String> {
    build_glyph(svg).map(fit_advance_to_ink)
}

fn fit_advance_to_ink(mut glyph: Glyph) -> Glyph {
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

fn build_glyph(svg: &str) -> Result<Glyph, String> {
    if !svg.contains(CANVAS_VIEW_BOX) {
        return Err("unexpected canvas size (expected a 109 x 109 view box)".to_string());
    }

    let mut paths = extract_paths(svg);
    if paths.is_empty() {
        return Err("no stroke paths found".to_string());
    }
    if paths.iter().all(|path| path.number.is_some()) {
        paths.sort_by_key(|path| path.number);
    }

    let mut strokes: Vec<Stroke> = Vec::new();
    for path in &paths {
        for polyline in flatten(&path.data)? {
            strokes.push(
                polyline
                    .into_iter()
                    .map(|(x, y)| Point {
                        x: x / CANVAS_SIZE,
                        y: y / CANVAS_SIZE,
                    })
                    .collect(),
            );
        }
    }

    let glyph = Glyph {
        advance: MAX_ADVANCE,
        strokes,
    };
    if glyph.strokes.is_empty() || !glyph.is_valid() {
        return Err("the converted glyph is invalid".to_string());
    }
    Ok(glyph)
}

struct PathElement {
    number: Option<u32>,
    data: String,
}

fn extract_paths(svg: &str) -> Vec<PathElement> {
    let mut paths = Vec::new();
    let mut rest = svg;
    while let Some(found) = rest.find("<path") {
        let after = &rest[found + "<path".len()..];
        let Some(end) = after.find('>') else {
            break;
        };
        let tag = &after[..end];
        rest = &after[end..];

        if !tag.starts_with(|ch: char| ch.is_ascii_whitespace()) {
            continue;
        }
        let Some(data) = attribute(tag, "d") else {
            continue;
        };
        paths.push(PathElement {
            number: attribute(tag, "id").and_then(stroke_number),
            data: data.to_string(),
        });
    }
    paths
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let mut search_from = 0;
    while let Some(found) = tag[search_from..].find(&needle) {
        let start = search_from + found;
        let boundary = tag[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| ch.is_ascii_whitespace());
        let value_start = start + needle.len();
        if boundary {
            let value_end = tag[value_start..].find('"')?;
            return Some(&tag[value_start..value_start + value_end]);
        }
        search_from = value_start;
    }
    None
}

fn stroke_number(id: &str) -> Option<u32> {
    let (_, number) = id.rsplit_once("-s")?;
    number.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIRAGANA_A: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- Copyright (C) Ulrich Apel. -->
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.0//EN" "http://www.w3.org/TR/2001/REC-SVG-20010904/DTD/svg10.dtd" [
<!ATTLIST g
xmlns:kvg CDATA #FIXED "http://kanjivg.tagaini.net"
kvg:element CDATA #IMPLIED >
<!ATTLIST path
xmlns:kvg CDATA #FIXED "http://kanjivg.tagaini.net"
kvg:type CDATA #IMPLIED >
]>
<svg xmlns="http://www.w3.org/2000/svg" width="109" height="109" viewBox="0 0 109 109" xmlns:kvg="https://kanjivg.tagaini.net/">
<g id="kvg:StrokePaths_03042" style="fill:none;stroke:#000000;stroke-width:3;">
<g id="kvg:03042" kvg:element="あ">
	<path id="kvg:03042-s1" d="M31.01,33c0.88,0.88,2.75,1.82,5.25,1.75c8.62-0.25,20-2.12,29.5-4.25c1.51-0.34,4.62-0.88,6.62-0.5"/>
	<path id="kvg:03042-s2" d="M49.76,17.62c0.88,1,1.82,3.26,1.38,5.25c-3.75,16.75-6.25,38.13-5.13,53.63c0.41,5.7,1.88,10.88,3.38,13.62"/>
</g>
</g>
<g id="kvg:StrokeNumbers_03042" style="font-size:8;fill:#808080">
	<text transform="matrix(1 0 0 1 22.51 35)">1</text>
</g>
</svg>
"#;

    fn svg_with(paths: &str) -> String {
        format!(r#"<svg viewBox="0 0 109 109"><g id="kvg:StrokePaths_x">{paths}</g></svg>"#)
    }

    #[test]
    fn converts_a_real_world_shaped_file() {
        let glyph = build_glyph(HIRAGANA_A).unwrap();

        assert_eq!(glyph.advance, 1.0);
        assert_eq!(glyph.strokes.len(), 2);
        let first = &glyph.strokes[0];
        assert!((first[0].x - 31.01 / 109.0).abs() < 1e-6);
        assert!((first[0].y - 33.0 / 109.0).abs() < 1e-6);
        let last = first.last().unwrap();
        assert!((last.x - 72.38 / 109.0).abs() < 1e-4);
        assert!((last.y - 30.0 / 109.0).abs() < 1e-4);
        assert!(first.len() > 3);
    }

    #[test]
    fn the_advance_follows_the_ink_width_with_side_bearings() {
        let svg = svg_with(r#"<path id="a-s1" d="M30.5,20L54.5,60"/>"#);

        let glyph = convert_svg(&svg).unwrap();

        let width = 24.0 / 109.0;
        assert!((glyph.advance - (width + 0.25)).abs() < 1e-5);
        let xs: Vec<f32> = glyph.strokes.iter().flatten().map(|p| p.x).collect();
        let min = xs.iter().copied().fold(f32::MAX, f32::min);
        let max = xs.iter().copied().fold(f32::MIN, f32::max);
        assert!((min - 0.125).abs() < 1e-5);
        assert!((glyph.advance - max - 0.125).abs() < 1e-5);
    }

    #[test]
    fn wide_glyphs_are_centered_within_the_maximum_advance() {
        let svg = svg_with(r#"<path id="a-s1" d="M10,20L100,20"/>"#);

        let glyph = convert_svg(&svg).unwrap();

        assert_eq!(glyph.advance, 1.0);
        let xs: Vec<f32> = glyph.strokes.iter().flatten().map(|p| p.x).collect();
        let min = xs.iter().copied().fold(f32::MAX, f32::min);
        let max = xs.iter().copied().fold(f32::MIN, f32::max);
        assert!((min - (1.0 - max)).abs() < 1e-5);
    }

    #[test]
    fn all_points_are_normalized_to_the_unit_box() {
        let glyph = convert_svg(HIRAGANA_A).unwrap();

        for point in glyph.strokes.iter().flatten() {
            assert!((0.0..=1.0).contains(&point.x), "{point:?}");
            assert!((0.0..=1.0).contains(&point.y), "{point:?}");
        }
    }

    #[test]
    fn strokes_follow_the_stroke_number_not_the_document_order() {
        let svg = svg_with(
            r#"<path id="kvg:00041-s2" d="M20,20L30,20"/>
               <path id="kvg:00041-s10" d="M90,90L99,90"/>
               <path id="kvg:00041-s1" d="M10,10L11,10"/>"#,
        );

        let glyph = build_glyph(&svg).unwrap();

        let starts: Vec<f32> = glyph.strokes.iter().map(|s| s[0].x * 109.0).collect();
        assert_eq!(starts.len(), 3);
        assert!((starts[0] - 10.0).abs() < 1e-3);
        assert!((starts[1] - 20.0).abs() < 1e-3);
        assert!((starts[2] - 90.0).abs() < 1e-3);
    }

    #[test]
    fn document_order_is_kept_when_numbers_are_missing() {
        let svg = svg_with(r#"<path d="M50,0L51,0"/><path d="M10,0L11,0"/>"#);

        let glyph = build_glyph(&svg).unwrap();

        assert!(glyph.strokes[0][0].x > glyph.strokes[1][0].x);
    }

    #[test]
    fn attributes_before_d_do_not_confuse_the_parser() {
        let svg = svg_with(r#"<path id="kvg:06f22-s1" kvg:type="㇔" d="M20,19.5L30,28"/>"#);

        let glyph = convert_svg(&svg).unwrap();

        assert_eq!(glyph.strokes.len(), 1);
    }

    #[test]
    fn rejects_an_unexpected_canvas() {
        let svg = r#"<svg viewBox="0 0 200 200"><path d="M0,0L1,1"/></svg>"#;

        assert!(convert_svg(svg).unwrap_err().contains("canvas"));
    }

    #[test]
    fn rejects_files_without_strokes() {
        assert!(convert_svg(&svg_with("")).is_err());
        assert!(convert_svg("<html>404</html>").is_err());
    }

    #[test]
    fn rejects_unsupported_path_commands() {
        let svg = svg_with(r#"<path id="a-s1" d="M0,0A5,5,0,0,1,10,0"/>"#);

        assert!(convert_svg(&svg).unwrap_err().contains("unsupported"));
    }

    #[test]
    fn a_dot_stroke_is_kept_as_a_single_point() {
        let svg = svg_with(r#"<path id="a-s1" d="M50,50"/>"#);

        let glyph = build_glyph(&svg).unwrap();

        assert_eq!(
            glyph.strokes,
            vec![vec![Point {
                x: 50.0 / 109.0,
                y: 50.0 / 109.0
            }]]
        );
    }
}
