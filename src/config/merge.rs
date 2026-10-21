use std::path::PathBuf;

use super::file::ConfigFile;
use super::model::Settings;
use super::validate;
use crate::error::{AutoQvError, Result};

pub fn settings_from_file(file: ConfigFile) -> Result<Settings> {
    let mut settings = Settings::default();

    if let Some(value) = file.sensitivity.counts_per_em_x {
        settings.planner.counts_per_em_x =
            checked("sensitivity.counts_per_em_x", validate::positive_f32(value))?;
    }
    if let Some(value) = file.sensitivity.counts_per_em_y {
        settings.planner.counts_per_em_y =
            checked("sensitivity.counts_per_em_y", validate::positive_f32(value))?;
    }

    if let Some(value) = file.pen.button {
        settings.button = value.parse()?;
    }
    if let Some(value) = file.pen.step_delay_ms {
        settings.timing.step_delay_ms =
            checked("pen.step_delay_ms", validate::positive_u32(value))?;
    }
    if let Some(value) = file.pen.max_counts_per_step {
        settings.timing.max_counts_per_step =
            checked("pen.max_counts_per_step", validate::positive_u32(value))?;
    }
    if let Some(value) = file.pen.move_speed_multiplier {
        settings.timing.move_speed_multiplier =
            checked("pen.move_speed_multiplier", validate::positive_f32(value))?;
    }
    if let Some(value) = file.pen.min_pen_down_interval_ms {
        settings.timing.min_pen_down_interval_ms = value;
    }

    if let Some(value) = file.layout.size {
        settings.layout.size = checked("layout.size", validate::positive_f32(value))?;
    }
    if let Some(value) = file.layout.char_spacing {
        settings.layout.char_spacing =
            checked("layout.char_spacing", validate::non_negative_f32(value))?;
    }
    if let Some(value) = file.layout.line_spacing {
        settings.layout.line_spacing =
            checked("layout.line_spacing", validate::non_negative_f32(value))?;
    }
    if let Some(value) = file.layout.line_width_em {
        settings.layout.line_width_em =
            checked("layout.line_width_em", validate::positive_f32(value))?;
    }
    if let Some(value) = file.layout.kinsoku {
        settings.layout.kinsoku = value;
    }

    if let Some(value) = file.log.missing_glyph_log {
        if value.trim().is_empty() {
            return Err(AutoQvError::Config(
                "log.missing_glyph_log: must not be empty".to_string(),
            ));
        }
        settings.missing_glyph_log = PathBuf::from(value);
    }

    if let Some(value) = file.safety.start_hotkey {
        settings.start_hotkey = value.parse()?;
    }
    if let Some(value) = file.safety.abort_hotkey {
        settings.abort_hotkey = value.parse()?;
    }

    Ok(settings)
}

fn checked<T>(name: &str, result: std::result::Result<T, &'static str>) -> Result<T> {
    result.map_err(|reason| AutoQvError::Config(format!("{name}: {reason}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::hotkey::Hotkey;
    use crate::input::pen_button::PenButton;

    fn parse(text: &str) -> Result<Settings> {
        let file: ConfigFile =
            toml::from_str(text).map_err(|e| AutoQvError::Config(e.to_string()))?;
        settings_from_file(file)
    }

    #[test]
    fn empty_file_yields_defaults() {
        assert_eq!(parse("").unwrap(), Settings::default());
    }

    #[test]
    fn applies_every_section() {
        let settings = parse(
            r#"
            [sensitivity]
            counts_per_em_x = 300.0
            counts_per_em_y = 200.0

            [pen]
            button = "right"
            step_delay_ms = 5
            max_counts_per_step = 6
            move_speed_multiplier = 2.5
            min_pen_down_interval_ms = 300

            [layout]
            size = 1.5
            char_spacing = 0.2
            line_spacing = 0.5
            line_width_em = 12.0
            kinsoku = false

            [log]
            missing_glyph_log = "missing.log"

            [safety]
            start_hotkey = "Ctrl+F8"
            abort_hotkey = "Ctrl+F9"
            "#,
        )
        .unwrap();

        assert_eq!(settings.planner.counts_per_em_x, 300.0);
        assert_eq!(settings.planner.counts_per_em_y, 200.0);
        assert_eq!(settings.button, PenButton::Right);
        assert_eq!(settings.timing.step_delay_ms, 5);
        assert_eq!(settings.timing.max_counts_per_step, 6);
        assert_eq!(settings.timing.move_speed_multiplier, 2.5);
        assert_eq!(settings.timing.min_pen_down_interval_ms, 300);
        assert_eq!(settings.layout.size, 1.5);
        assert_eq!(settings.layout.char_spacing, 0.2);
        assert_eq!(settings.layout.line_spacing, 0.5);
        assert_eq!(settings.layout.line_width_em, 12.0);
        assert!(!settings.layout.kinsoku);
        assert_eq!(settings.missing_glyph_log, PathBuf::from("missing.log"));
        assert_eq!(settings.start_hotkey.to_string(), "Ctrl+F8");
        assert_eq!(settings.abort_hotkey.to_string(), "Ctrl+F9");
    }

    #[test]
    fn omitted_values_keep_their_defaults() {
        let settings = parse("[sensitivity]\ncounts_per_em_x = 123.0\n").unwrap();

        assert_eq!(settings.planner.counts_per_em_x, 123.0);
        assert_eq!(
            settings.planner.counts_per_em_y,
            Settings::default().planner.counts_per_em_y
        );
        assert_eq!(settings.start_hotkey, Hotkey::DEFAULT_START);
    }

    #[test]
    fn rejects_unknown_keys_and_sections() {
        assert!(parse("[pen]\nbutton_name = \"left\"\n").is_err());
        assert!(parse("[unknown]\nvalue = 1\n").is_err());
    }

    #[test]
    fn rejects_invalid_values_with_the_field_name() {
        let error = parse("[sensitivity]\ncounts_per_em_x = 0.0\n").unwrap_err();
        assert!(error.to_string().contains("sensitivity.counts_per_em_x"));

        assert!(parse("[pen]\nstep_delay_ms = 0\n").is_err());
        assert!(parse("[layout]\nchar_spacing = -0.1\n").is_err());
        assert!(parse("[pen]\nbutton = \"x1\"\n").is_err());
        assert!(parse("[safety]\nstart_hotkey = \"P\"\n").is_err());
        assert!(parse("[log]\nmissing_glyph_log = \" \"\n").is_err());
    }

    #[test]
    fn allows_zero_spacing() {
        let settings = parse("[layout]\nchar_spacing = 0.0\nline_spacing = 0.0\n").unwrap();

        assert_eq!(settings.layout.char_spacing, 0.0);
        assert_eq!(settings.layout.line_spacing, 0.0);
    }
}
