use crate::cli::args::PreviewArgs;
use crate::cli::prepared::Prepared;
use crate::config::loader::load_settings;

pub fn run(args: &PreviewArgs) -> anyhow::Result<()> {
    let mut settings = load_settings(args.common.config.as_deref())?;
    args.common.apply(&mut settings);

    let prepared = Prepared::build(
        &args.text,
        &args.common.glyph_files,
        args.common.offline,
        &settings,
    )?;
    print!("{}", prepared.report());
    Ok(())
}
