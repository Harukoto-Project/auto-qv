use std::path::PathBuf;

use crate::input::hotkey::Hotkey;
use crate::input::pen_button::PenButton;
use crate::layout::model::LayoutConfig;
use crate::planner::path_planner::PlannerConfig;
use crate::runner::timing::StepTiming;

pub const DEFAULT_MISSING_GLYPH_LOG: &str = "autoqv-missing-glyphs.log";

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub layout: LayoutConfig,
    pub planner: PlannerConfig,
    pub timing: StepTiming,
    pub button: PenButton,
    pub start_hotkey: Hotkey,
    pub abort_hotkey: Hotkey,
    pub missing_glyph_log: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            planner: PlannerConfig::default(),
            timing: StepTiming::default(),
            button: PenButton::default(),
            start_hotkey: Hotkey::DEFAULT_START,
            abort_hotkey: Hotkey::DEFAULT_ABORT,
            missing_glyph_log: PathBuf::from(DEFAULT_MISSING_GLYPH_LOG),
        }
    }
}
