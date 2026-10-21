use anyhow::bail;

use crate::cli::args::PreviewArgs;
use crate::cli::report::PreviewReport;
use crate::glyph::store::GlyphStore;
use crate::input::dry_run::DryRunDriver;
use crate::input::pen_driver::apply_action;
use crate::layout::text_layout::layout_text;
use crate::planner::path_planner::plan;
use crate::runner::timing::estimate_duration;

pub fn run(args: &PreviewArgs) -> anyhow::Result<()> {
    if args.common.glyph_files.is_empty() {
        bail!("no stroke data available; specify at least one --glyph-file <PATH>");
    }
    let store = GlyphStore::from_json_files(&args.common.glyph_files)?;

    let layout = layout_text(&args.text, &store, &args.common.layout_config());
    let actions = plan(&layout.strokes, &args.common.planner_config());

    let mut driver = DryRunDriver::new();
    for action in &actions {
        apply_action(&mut driver, *action)?;
    }

    let report = PreviewReport {
        placed_chars: layout.placed,
        stroke_count: layout.strokes.len(),
        extent_em: layout.extent_em(),
        stats: driver.stats(),
        duration: estimate_duration(&actions, &args.common.step_timing()),
        missing: &layout.missing,
    };
    print!("{report}");
    Ok(())
}
