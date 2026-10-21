use crate::cli::args::CheckArgs;
use crate::cli::check_report::CheckReport;
use crate::cli::stroke_data::load_glyphs;
use crate::cli::text_input::read_text;
use crate::layout::model::LayoutConfig;
use crate::layout::text_layout::layout_text;

pub fn run(args: &CheckArgs) -> anyhow::Result<()> {
    let text = read_text(&args.source)?;
    let store = load_glyphs(&text, &args.data.glyph_files, args.data.offline)?;
    let layout = layout_text(&text, &store, &LayoutConfig::default());
    print!("{}", CheckReport::from_layout(&layout));
    Ok(())
}
