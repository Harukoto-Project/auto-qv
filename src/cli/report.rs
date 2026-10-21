use std::fmt;
use std::time::Duration;

use crate::input::dry_run::DryRunStats;
use crate::layout::model::MissingGlyph;

const MAX_LISTED_MISSING: usize = 20;

pub struct PreviewReport<'a> {
    pub placed_chars: usize,
    pub stroke_count: usize,
    pub extent_em: (f32, f32),
    pub stats: &'a DryRunStats,
    pub duration: Duration,
    pub missing: &'a [MissingGlyph],
}

impl fmt::Display for PreviewReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Characters placed : {}", self.placed_chars)?;
        writeln!(f, "Strokes           : {}", self.stroke_count)?;
        writeln!(
            f,
            "Extent            : {:.2} x {:.2} em",
            self.extent_em.0, self.extent_em.1
        )?;
        writeln!(f, "Pen down count    : {}", self.stats.pen_down_count)?;
        writeln!(f, "Moves             : {}", self.stats.move_count)?;
        writeln!(
            f,
            "Draw distance     : {:.0} counts",
            self.stats.draw_distance
        )?;
        writeln!(
            f,
            "Travel distance   : {:.0} counts",
            self.stats.travel_distance
        )?;
        writeln!(f, "Estimated time    : {}", format_duration(self.duration))?;

        if !self.missing.is_empty() {
            writeln!(f, "Missing glyphs    : {}", self.missing.len())?;
            for glyph in self.missing.iter().take(MAX_LISTED_MISSING) {
                writeln!(
                    f,
                    "  {:?} (U+{:04X}) at line {}, column {}",
                    glyph.ch, glyph.ch as u32, glyph.line, glyph.column
                )?;
            }
            if self.missing.len() > MAX_LISTED_MISSING {
                writeln!(
                    f,
                    "  ... and {} more",
                    self.missing.len() - MAX_LISTED_MISSING
                )?;
            }
        }
        Ok(())
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs_f64();
    let minutes = (total_seconds / 60.0).floor() as u64;
    let seconds = total_seconds - (minutes as f64) * 60.0;
    if minutes > 0 {
        format!("{minutes}m {seconds:.1}s")
    } else {
        format!("{seconds:.1}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing(count: usize) -> Vec<MissingGlyph> {
        (0..count)
            .map(|i| MissingGlyph {
                ch: 'x',
                line: 1,
                column: i + 1,
            })
            .collect()
    }

    fn render(missing: &[MissingGlyph]) -> String {
        let stats = DryRunStats::default();
        PreviewReport {
            placed_chars: 3,
            stroke_count: 5,
            extent_em: (2.0, 1.0),
            stats: &stats,
            duration: Duration::from_millis(61_500),
            missing,
        }
        .to_string()
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(Duration::from_millis(4_500)), "4.5s");
        assert_eq!(format_duration(Duration::from_millis(61_500)), "1m 1.5s");
    }

    #[test]
    fn omits_missing_section_when_nothing_is_missing() {
        let text = render(&[]);

        assert!(text.contains("Characters placed : 3"));
        assert!(text.contains("Estimated time    : 1m 1.5s"));
        assert!(!text.contains("Missing glyphs"));
    }

    #[test]
    fn lists_missing_glyphs_and_truncates_long_lists() {
        let text = render(&missing(22));

        assert!(text.contains("Missing glyphs    : 22"));
        assert!(text.contains("'x' (U+0078) at line 1, column 1"));
        assert!(!text.contains("column 21"));
        assert!(text.contains("... and 2 more"));
    }
}
