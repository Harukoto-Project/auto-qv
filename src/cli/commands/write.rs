use std::fs;
use std::path::Path;

use anyhow::{Context, bail};

use crate::cli::args::WriteArgs;
use crate::cli::prepared::Prepared;
use crate::cli::progress::ProgressPrinter;
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

    let text = read_text(args)?;
    let prepared = Prepared::build(&text, &args.common.glyph_files, &settings)?;
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

fn read_text(args: &WriteArgs) -> anyhow::Result<String> {
    match (&args.text, &args.file) {
        (Some(text), _) => Ok(text.clone()),
        (None, Some(path)) => read_text_file(path),
        (None, None) => bail!("specify the text to write or --file <PATH>"),
    }
}

fn read_text_file(path: &Path) -> anyhow::Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read {} as UTF-8 text", path.display()))?;
    Ok(match text.strip_prefix('\u{feff}') {
        Some(stripped) => stripped.to_string(),
        None => text,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_utf8_text_and_strips_a_byte_order_mark() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text.txt");
        fs::write(&path, "\u{feff}こんにちは\nabc").unwrap();

        assert_eq!(read_text_file(&path).unwrap(), "こんにちは\nabc");
    }

    #[test]
    fn missing_text_file_reports_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("none.txt");

        let error = read_text_file(&path).unwrap_err();

        assert!(error.to_string().contains("none.txt"));
    }

    #[test]
    fn non_utf8_text_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sjis.txt");
        fs::write(&path, [0x82, 0xA0]).unwrap();

        assert!(read_text_file(&path).is_err());
    }
}
