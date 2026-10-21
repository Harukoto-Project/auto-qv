use thiserror::Error;

#[derive(Debug, Error)]
pub enum AutoQvError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("invalid glyph data: {0}")]
    GlyphData(String),

    #[error("input device error: {0}")]
    Input(String),

    #[error("aborted by user")]
    Aborted,
}

pub type Result<T> = std::result::Result<T, AutoQvError>;
