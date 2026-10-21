use crate::glyph::model::Stroke;

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConfig {
    pub size: f32,
    pub width_scale: f32,
    pub char_spacing: f32,
    pub line_spacing: f32,
    pub line_width_em: f32,
    pub kinsoku: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            size: 1.0,
            width_scale: 1.0,
            char_spacing: 0.15,
            line_spacing: 0.4,
            line_width_em: 20.0,
            kinsoku: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingGlyph {
    pub ch: char,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Default)]
pub struct Layout {
    pub strokes: Vec<Stroke>,
    pub missing: Vec<MissingGlyph>,
    pub placed: usize,
}

impl Layout {
    pub fn extent_em(&self) -> (f32, f32) {
        self.strokes
            .iter()
            .flatten()
            .fold((0.0, 0.0), |(w, h), p| (w.max(p.x), h.max(p.y)))
    }
}
