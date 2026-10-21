use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(from = "[f32; 2]")]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl From<[f32; 2]> for Point {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

pub type Stroke = Vec<Point>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Glyph {
    pub advance: f32,
    pub strokes: Vec<Stroke>,
}

impl Glyph {
    pub fn is_valid(&self) -> bool {
        self.advance.is_finite()
            && self.advance > 0.0
            && self
                .strokes
                .iter()
                .all(|stroke| !stroke.is_empty() && stroke.iter().all(Point::is_finite))
    }
}

impl Point {
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
