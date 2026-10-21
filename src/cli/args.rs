use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, value_parser};

use crate::config::model::Settings;
use crate::config::validate;
use crate::input::hotkey::Hotkey;
use crate::input::pen_button::PenButton;

#[derive(Debug, Parser)]
#[command(
    name = "autoqv",
    version,
    about = "Automates pen writing in VRChat with relative mouse movement"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Write text with the pen (waits for the start hotkey)")]
    Write(WriteArgs),
    #[command(about = "Show the planned pen path summary without sending any input")]
    Preview(PreviewArgs),
    #[command(about = "List the characters in the text that have no stroke data")]
    Check(CheckArgs),
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    pub source: TextSourceArgs,

    #[command(flatten)]
    pub data: StrokeDataArgs,
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("source").required(true).args(["text", "file"])))]
pub struct TextSourceArgs {
    #[arg(value_name = "TEXT", help = "Text to use")]
    pub text: Option<String>,

    #[arg(long, value_name = "PATH", help = "Read the text from a UTF-8 file")]
    pub file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct StrokeDataArgs {
    #[arg(
        long = "glyph-file",
        value_name = "PATH",
        help = "Stroke data JSON file merged over the downloaded data (repeatable, later files win)"
    )]
    pub glyph_files: Vec<PathBuf>,

    #[arg(
        long,
        help = "Do not download stroke data; use only the cache and --glyph-file"
    )]
    pub offline: bool,
}

#[derive(Debug, Args)]
pub struct PreviewArgs {
    #[arg(value_name = "TEXT", help = "Text to write")]
    pub text: String,

    #[command(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, Args)]
pub struct WriteArgs {
    #[command(flatten)]
    pub source: TextSourceArgs,

    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(
        long,
        value_name = "BUTTON",
        value_parser = value_parser!(PenButton),
        help = "Mouse button used to put the pen down (left, right or middle)"
    )]
    pub button: Option<PenButton>,

    #[arg(
        long,
        value_name = "KEY",
        value_parser = value_parser!(Hotkey),
        help = "Hotkey that starts writing (e.g. Alt+P)"
    )]
    pub start_hotkey: Option<Hotkey>,

    #[arg(
        long,
        value_name = "KEY",
        value_parser = value_parser!(Hotkey),
        help = "Hotkey that aborts writing (e.g. Alt+C)"
    )]
    pub abort_hotkey: Option<Hotkey>,

    #[arg(long, help = "Show the plan and exit without sending any input")]
    pub dry_run: bool,
}

impl WriteArgs {
    pub fn apply(&self, settings: &mut Settings) {
        self.common.apply(settings);
        if let Some(button) = self.button {
            settings.button = button;
        }
        if let Some(hotkey) = self.start_hotkey {
            settings.start_hotkey = hotkey;
        }
        if let Some(hotkey) = self.abort_hotkey {
            settings.abort_hotkey = hotkey;
        }
    }
}

#[derive(Debug, Args)]
pub struct CommonArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "Configuration file (default: autoqv.toml next to the executable, if present)"
    )]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub data: StrokeDataArgs,

    #[arg(long, value_name = "N", value_parser = positive_f32, help = "Glyph scale factor")]
    pub size: Option<f32>,

    #[arg(
        long,
        value_name = "COUNTS",
        value_parser = positive_f32,
        help = "Mouse counts per em for both axes"
    )]
    pub counts_per_em: Option<f32>,

    #[arg(long, value_name = "COUNTS", value_parser = positive_f32, help = "Mouse counts per em (horizontal)")]
    pub counts_per_em_x: Option<f32>,

    #[arg(long, value_name = "COUNTS", value_parser = positive_f32, help = "Mouse counts per em (vertical)")]
    pub counts_per_em_y: Option<f32>,

    #[arg(long, value_name = "N", value_parser = positive_f32, help = "Horizontal glyph scale factor (relative to size)")]
    pub width_scale: Option<f32>,

    #[arg(long, value_name = "N", value_parser = non_negative_f32, help = "Character spacing (relative to size)")]
    pub char_spacing: Option<f32>,

    #[arg(long, value_name = "N", value_parser = non_negative_f32, help = "Line spacing (relative to size)")]
    pub line_spacing: Option<f32>,

    #[arg(long, value_name = "EM", value_parser = positive_f32, help = "Maximum line width in em")]
    pub line_width_em: Option<f32>,

    #[arg(long, help = "Disable line-start prohibition rules when wrapping")]
    pub no_kinsoku: bool,

    #[arg(long, value_name = "MS", value_parser = positive_u32, help = "Delay between mouse steps in milliseconds")]
    pub step_delay_ms: Option<u32>,

    #[arg(long, value_name = "COUNTS", value_parser = positive_u32, help = "Maximum mouse counts per step while drawing")]
    pub max_counts_per_step: Option<u32>,

    #[arg(long, value_name = "N", value_parser = positive_f32, help = "Step size multiplier while the pen is up")]
    pub move_speed_multiplier: Option<f32>,

    #[arg(
        long,
        value_name = "MS",
        help = "Minimum interval between pen-down events in milliseconds (0 to disable)"
    )]
    pub min_pen_down_interval_ms: Option<u32>,

    #[arg(
        long,
        value_name = "MS",
        help = "Wait after pressing the pen button before drawing starts, in milliseconds (0 to disable)"
    )]
    pub pen_down_delay_ms: Option<u32>,

    #[arg(
        long,
        value_name = "MS",
        help = "Wait after releasing the pen button before moving on, in milliseconds (0 to disable)"
    )]
    pub pen_up_delay_ms: Option<u32>,
}

impl CommonArgs {
    pub fn apply(&self, settings: &mut Settings) {
        if let Some(size) = self.size {
            settings.layout.size = size;
        }
        if let Some(value) = self.width_scale {
            settings.layout.width_scale = value;
        }
        if let Some(value) = self.char_spacing {
            settings.layout.char_spacing = value;
        }
        if let Some(value) = self.line_spacing {
            settings.layout.line_spacing = value;
        }
        if let Some(value) = self.line_width_em {
            settings.layout.line_width_em = value;
        }
        if self.no_kinsoku {
            settings.layout.kinsoku = false;
        }

        if let Some(value) = self.counts_per_em {
            settings.planner.counts_per_em_x = value;
            settings.planner.counts_per_em_y = value;
        }
        if let Some(value) = self.counts_per_em_x {
            settings.planner.counts_per_em_x = value;
        }
        if let Some(value) = self.counts_per_em_y {
            settings.planner.counts_per_em_y = value;
        }

        if let Some(value) = self.step_delay_ms {
            settings.timing.step_delay_ms = value;
        }
        if let Some(value) = self.max_counts_per_step {
            settings.timing.max_counts_per_step = value;
        }
        if let Some(value) = self.move_speed_multiplier {
            settings.timing.move_speed_multiplier = value;
        }
        if let Some(value) = self.min_pen_down_interval_ms {
            settings.timing.min_pen_down_interval_ms = value;
        }
        if let Some(value) = self.pen_down_delay_ms {
            settings.timing.pen_down_delay_ms = value;
        }
        if let Some(value) = self.pen_up_delay_ms {
            settings.timing.pen_up_delay_ms = value;
        }
    }
}

fn positive_f32(value: &str) -> Result<f32, String> {
    let parsed: f32 = value.parse().map_err(|e| format!("{e}"))?;
    validate::positive_f32(parsed).map_err(String::from)
}

fn non_negative_f32(value: &str) -> Result<f32, String> {
    let parsed: f32 = value.parse().map_err(|e| format!("{e}"))?;
    validate::non_negative_f32(parsed).map_err(String::from)
}

fn positive_u32(value: &str) -> Result<u32, String> {
    let parsed: u32 = value.parse().map_err(|e| format!("{e}"))?;
    validate::positive_u32(parsed).map_err(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("autoqv").chain(args.iter().copied()))
    }

    fn common(cli: Cli) -> CommonArgs {
        match cli.command {
            Command::Preview(args) => args.common,
            Command::Write(args) => args.common,
            Command::Check(_) => panic!("the check command has no common options"),
        }
    }

    fn write_args(args: &[&str]) -> WriteArgs {
        match parse(args).unwrap().command {
            Command::Write(args) => args,
            _ => panic!("expected the write command"),
        }
    }

    fn check_args(args: &[&str]) -> CheckArgs {
        match parse(args).unwrap().command {
            Command::Check(args) => args,
            _ => panic!("expected the check command"),
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn leaves_settings_untouched_when_options_are_omitted() {
        let common = common(parse(&["preview", "abc"]).unwrap());
        let mut settings = Settings::default();

        common.apply(&mut settings);

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn per_axis_counts_override_the_shared_value() {
        let common = common(
            parse(&[
                "preview",
                "abc",
                "--counts-per-em",
                "300",
                "--counts-per-em-y",
                "200",
            ])
            .unwrap(),
        );
        let mut settings = Settings::default();

        common.apply(&mut settings);

        assert_eq!(settings.planner.counts_per_em_x, 300.0);
        assert_eq!(settings.planner.counts_per_em_y, 200.0);
    }

    #[test]
    fn cli_values_override_existing_settings() {
        let common =
            common(parse(&["preview", "abc", "--size", "2", "--step-delay-ms", "7"]).unwrap());
        let mut settings = Settings::default();
        settings.layout.size = 1.5;
        settings.layout.line_spacing = 0.9;
        settings.timing.step_delay_ms = 4;

        common.apply(&mut settings);

        assert_eq!(settings.layout.size, 2.0);
        assert_eq!(settings.timing.step_delay_ms, 7);
        assert_eq!(settings.layout.line_spacing, 0.9);
    }

    #[test]
    fn min_pen_down_interval_can_be_overridden_and_disabled() {
        let mut settings = Settings::default();
        assert_eq!(settings.timing.min_pen_down_interval_ms, 230);

        common(parse(&["preview", "abc", "--min-pen-down-interval-ms", "300"]).unwrap())
            .apply(&mut settings);
        assert_eq!(settings.timing.min_pen_down_interval_ms, 300);

        common(parse(&["preview", "abc", "--min-pen-down-interval-ms", "0"]).unwrap())
            .apply(&mut settings);
        assert_eq!(settings.timing.min_pen_down_interval_ms, 0);
    }

    #[test]
    fn pen_delays_can_be_overridden_and_disabled() {
        let mut settings = Settings::default();
        assert_eq!(settings.timing.pen_down_delay_ms, 30);
        assert_eq!(settings.timing.pen_up_delay_ms, 30);

        common(
            parse(&[
                "preview",
                "abc",
                "--pen-down-delay-ms",
                "50",
                "--pen-up-delay-ms",
                "0",
            ])
            .unwrap(),
        )
        .apply(&mut settings);

        assert_eq!(settings.timing.pen_down_delay_ms, 50);
        assert_eq!(settings.timing.pen_up_delay_ms, 0);
    }

    #[test]
    fn width_scale_can_be_overridden_and_must_be_positive() {
        let mut settings = Settings::default();
        assert_eq!(settings.layout.width_scale, 1.0);

        common(parse(&["preview", "abc", "--width-scale", "0.6"]).unwrap()).apply(&mut settings);

        assert_eq!(settings.layout.width_scale, 0.6);
        assert!(parse(&["preview", "abc", "--width-scale", "0"]).is_err());
    }

    #[test]
    fn no_kinsoku_disables_kinsoku() {
        let common = common(parse(&["preview", "abc", "--no-kinsoku"]).unwrap());
        let mut settings = Settings::default();

        common.apply(&mut settings);

        assert!(!settings.layout.kinsoku);
    }

    #[test]
    fn rejects_non_positive_values() {
        assert!(parse(&["preview", "abc", "--size", "0"]).is_err());
        assert!(parse(&["preview", "abc", "--max-counts-per-step", "0"]).is_err());
        assert!(parse(&["preview", "abc", "--char-spacing", "-0.1"]).is_err());
    }

    #[test]
    fn write_requires_exactly_one_text_source() {
        assert!(parse(&["write"]).is_err());
        assert!(parse(&["write", "abc", "--file", "a.txt"]).is_err());
        assert!(parse(&["write", "abc"]).is_ok());
        assert!(parse(&["write", "--file", "a.txt"]).is_ok());
    }

    #[test]
    fn check_requires_exactly_one_text_source() {
        assert!(parse(&["check"]).is_err());
        assert!(parse(&["check", "abc", "--file", "a.txt"]).is_err());
        assert!(parse(&["check", "abc"]).is_ok());
        assert!(parse(&["check", "--file", "a.txt"]).is_ok());
    }

    #[test]
    fn check_accepts_only_stroke_data_options() {
        let args = check_args(&["check", "abc", "--offline", "--glyph-file", "g.json"]);

        assert!(args.data.offline);
        assert_eq!(args.data.glyph_files, vec![PathBuf::from("g.json")]);
        assert!(parse(&["check", "abc", "--size", "2"]).is_err());
    }

    #[test]
    fn write_options_override_settings() {
        let args = write_args(&[
            "write",
            "abc",
            "--button",
            "right",
            "--start-hotkey",
            "Ctrl+F8",
            "--abort-hotkey",
            "Ctrl+F9",
        ]);
        let mut settings = Settings::default();

        args.apply(&mut settings);

        assert_eq!(settings.button, PenButton::Right);
        assert_eq!(settings.start_hotkey.to_string(), "Ctrl+F8");
        assert_eq!(settings.abort_hotkey.to_string(), "Ctrl+F9");
    }

    #[test]
    fn write_rejects_invalid_button_and_hotkey() {
        assert!(parse(&["write", "abc", "--button", "x1"]).is_err());
        assert!(parse(&["write", "abc", "--start-hotkey", "P"]).is_err());
    }

    #[test]
    fn write_dry_run_flag_is_parsed() {
        assert!(write_args(&["write", "abc", "--dry-run"]).dry_run);
        assert!(!write_args(&["write", "abc"]).dry_run);
    }
}
