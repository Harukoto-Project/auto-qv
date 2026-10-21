use std::path::PathBuf;
use std::time::Duration;

use super::report::PreviewReport;
use super::stroke_data::load_glyphs;
use crate::config::model::Settings;
use crate::input::dry_run::{DryRunDriver, DryRunStats};
use crate::input::pen_driver::apply_action;
use crate::layout::model::Layout;
use crate::layout::text_layout::layout_text;
use crate::planner::action::Action;
use crate::planner::path_planner::plan;
use crate::runner::timing::estimate_duration;

pub struct Prepared {
    pub layout: Layout,
    pub actions: Vec<Action>,
    stats: DryRunStats,
    duration: Duration,
}

impl Prepared {
    pub fn build(
        text: &str,
        glyph_files: &[PathBuf],
        offline: bool,
        settings: &Settings,
    ) -> anyhow::Result<Self> {
        let store = load_glyphs(text, glyph_files, offline)?;

        let layout = layout_text(text, &store, &settings.layout);
        let actions = plan(&layout.strokes, &settings.planner);

        let mut driver = DryRunDriver::new();
        for action in &actions {
            apply_action(&mut driver, *action)?;
        }

        Ok(Self {
            stats: driver.stats().clone(),
            duration: estimate_duration(&actions, &settings.timing),
            layout,
            actions,
        })
    }

    pub fn report(&self) -> PreviewReport<'_> {
        PreviewReport {
            placed_chars: self.layout.placed,
            stroke_count: self.layout.strokes.len(),
            extent_em: self.layout.extent_em(),
            stats: &self.stats,
            duration: self.duration,
            missing: &self.layout.missing,
        }
    }
}
