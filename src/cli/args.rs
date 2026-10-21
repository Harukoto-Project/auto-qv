use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::layout::model::LayoutConfig;
use crate::planner::path_planner::PlannerConfig;
use crate::runner::timing::StepTiming;

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
    #[command(about = "Show the planned pen path summary without sending any input")]
    Preview(PreviewArgs),
}

#[derive(Debug, Args)]
pub struct PreviewArgs {
    #[arg(value_name = "TEXT", help = "Text to write")]
    pub text: String,

    #[command(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, Args)]
pub struct CommonArgs {
    #[arg(
        long = "glyph-file",
        value_name = "PATH",
        help = "Stroke data JSON file (repeatable, later files override earlier ones)"
    )]
    pub glyph_files: Vec<PathBuf>,

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
}

impl CommonArgs {
    pub fn layout_config(&self) -> LayoutConfig {
        let defaults = LayoutConfig::default();
        LayoutConfig {
            size: self.size.unwrap_or(defaults.size),
            char_spacing: self.char_spacing.unwrap_or(defaults.char_spacing),
            line_spacing: self.line_spacing.unwrap_or(defaults.line_spacing),
            line_width_em: self.line_width_em.unwrap_or(defaults.line_width_em),
            kinsoku: defaults.kinsoku && !self.no_kinsoku,
        }
    }

    pub fn planner_config(&self) -> PlannerConfig {
        let defaults = PlannerConfig::default();
        PlannerConfig {
            counts_per_em_x: self
                .counts_per_em_x
                .or(self.counts_per_em)
                .unwrap_or(defaults.counts_per_em_x),
            counts_per_em_y: self
                .counts_per_em_y
                .or(self.counts_per_em)
                .unwrap_or(defaults.counts_per_em_y),
        }
    }

    pub fn step_timing(&self) -> StepTiming {
        let defaults = StepTiming::default();
        StepTiming {
            step_delay_ms: self.step_delay_ms.unwrap_or(defaults.step_delay_ms),
            max_counts_per_step: self
                .max_counts_per_step
                .unwrap_or(defaults.max_counts_per_step),
            move_speed_multiplier: self
                .move_speed_multiplier
                .unwrap_or(defaults.move_speed_multiplier),
        }
    }
}

fn positive_f32(value: &str) -> Result<f32, String> {
    let parsed: f32 = value.parse().map_err(|e| format!("{e}"))?;
    if parsed.is_finite() && parsed > 0.0 {
        Ok(parsed)
    } else {
        Err("must be a positive number".to_string())
    }
}

fn non_negative_f32(value: &str) -> Result<f32, String> {
    let parsed: f32 = value.parse().map_err(|e| format!("{e}"))?;
    if parsed.is_finite() && parsed >= 0.0 {
        Ok(parsed)
    } else {
        Err("must be zero or a positive number".to_string())
    }
}

fn positive_u32(value: &str) -> Result<u32, String> {
    let parsed: u32 = value.parse().map_err(|e| format!("{e}"))?;
    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("must be a positive integer".to_string())
    }
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
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn uses_defaults_when_options_are_omitted() {
        let common = common(parse(&["preview", "abc"]).unwrap());

        assert_eq!(common.layout_config(), LayoutConfig::default());
        assert_eq!(common.planner_config(), PlannerConfig::default());
        assert_eq!(common.step_timing(), StepTiming::default());
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
        let planner = common.planner_config();

        assert_eq!(planner.counts_per_em_x, 300.0);
        assert_eq!(planner.counts_per_em_y, 200.0);
    }

    #[test]
    fn no_kinsoku_disables_kinsoku() {
        let common = common(parse(&["preview", "abc", "--no-kinsoku"]).unwrap());

        assert!(!common.layout_config().kinsoku);
    }

    #[test]
    fn rejects_non_positive_values() {
        assert!(parse(&["preview", "abc", "--size", "0"]).is_err());
        assert!(parse(&["preview", "abc", "--max-counts-per-step", "0"]).is_err());
        assert!(parse(&["preview", "abc", "--char-spacing", "-0.1"]).is_err());
    }
}
