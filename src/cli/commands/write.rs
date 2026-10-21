use anyhow::bail;

use crate::cli::args::WriteArgs;
use crate::cli::prepared::Prepared;
use crate::cli::progress::ProgressPrinter;
use crate::cli::text_input::read_text;
use crate::config::loader::load_settings;
use crate::config::model::Settings;
use crate::input::hotkey_listener::{HotkeyEvent, HotkeyListener};
use crate::input::windows::WindowsPenDriver;
use crate::missing_log::writer::record_missing_glyphs;
use crate::runner::executor::{RunOutcome, execute};

pub fn run(args: &WriteArgs) -> anyhow::Result<()> {
    let mut settings = load_settings(args.common.config.as_deref())?;
    args.apply(&mut settings);
    if settings.start_hotkey == settings.abort_hotkey {
        bail!(
            "the start and abort hotkeys must differ (both are {})",
            settings.start_hotkey
        );
    }

    let text = read_text(&args.source)?;
    let prepared = Prepared::build(
        &text,
        &args.common.data.glyph_files,
        args.common.data.offline,
        &settings,
    )?;
    print!("{}", prepared.report());
    log_missing_glyphs(&settings, &prepared);

    if args.dry_run {
        return Ok(());
    }
    if prepared.layout.placed == 0 {
        bail!("nothing to write: none of the characters have stroke data");
    }
    write_with_hotkeys(&prepared, &settings)
}

fn log_missing_glyphs(settings: &Settings, prepared: &Prepared) {
    match record_missing_glyphs(&settings.missing_glyph_log, &prepared.layout.missing) {
        Ok(Some(path)) => eprintln!("Missing glyphs were logged to {}", path.display()),
        Ok(None) => {}
        Err(error) => eprintln!("warning: failed to write the missing glyph log: {error}"),
    }
}

fn write_with_hotkeys(prepared: &Prepared, settings: &Settings) -> anyhow::Result<()> {
    let listener = HotkeyListener::start(settings.start_hotkey, settings.abort_hotkey)?;
    eprintln!(
        "Waiting. Switch to the target window, then press {} to start. Press {} (or Ctrl+C) to quit.",
        settings.start_hotkey, settings.abort_hotkey
    );

    if listener.recv() == HotkeyEvent::Abort {
        eprintln!("Aborted before starting.");
        return Ok(());
    }
    eprintln!("Started. Press {} to abort.", settings.abort_hotkey);

    let mut driver = WindowsPenDriver::new(settings.button);
    let mut progress = ProgressPrinter::new();
    let outcome = execute(
        &prepared.actions,
        &mut driver,
        &settings.timing,
        &mut || listener.abort_requested(),
        &mut |done, total| progress.update(done, total),
    );
    progress.finish();

    match outcome? {
        RunOutcome::Completed => eprintln!("Done."),
        RunOutcome::Aborted => eprintln!("Aborted."),
    }
    Ok(())
}
